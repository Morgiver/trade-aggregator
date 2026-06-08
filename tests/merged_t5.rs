//! T5 — Replay fusionné event-time (issue #17, fiches `UC-T5-1..4`).
//!
//! Tests **synthétiques** des briques du flux fusionné au niveau `SymbolAggregator` :
//! ingestion de snapshots de carnet (MBP-10) intercalée avec les trades, synchronisation
//! de `book()`, et détection de désordre. Le replay DBN réel (`replay_merged`) est testé
//! sous feature `databento` + `TRADE_AGG_DATA_DIR` (cf. `databento_replay.rs`).

use std::cell::RefCell;
use std::rc::Rc;

use trade_aggregator::{
    AggressorSide, Bar, BookSide, Granularity, Instrument, MarketEvent, OrderBook, Subscriber,
    SymbolAggregator, Trade,
};

fn trade(ts: i64, price: i64, size: u64) -> MarketEvent {
    MarketEvent::Trade(Trade {
        ts,
        price,
        size,
        aggressor: AggressorSide::Buy,
        instrument_id: 1,
    })
}

fn snapshot(bid: (i64, u64), ask: (i64, u64)) -> OrderBook {
    let mut b = OrderBook::new();
    b.set_level(BookSide::Bid, bid.0, bid.1);
    b.set_level(BookSide::Ask, ask.0, ask.1);
    b
}

// UC-T5-2 : un snapshot ingéré synchronise `book()` avec le tape.
#[test]
fn ingest_book_snapshot_syncs_book() {
    let mut agg = SymbolAggregator::builder(
        Instrument {
            id: 1,
            tick_size: 1,
        },
        Granularity::L2,
    )
    .with_time_period(100)
    .with_passive()
    .build()
    .unwrap();

    // Flux fusionné trié : snapshot @5, trade @10, snapshot @20, trade @30.
    agg.ingest_book_snapshot(5, snapshot((99, 4), (101, 7)));
    agg.process(&trade(10, 100, 1));
    assert_eq!(agg.book().unwrap().best_bid(), Some((99, 4)));
    assert_eq!(agg.book().unwrap().best_ask(), Some((101, 7)));

    agg.ingest_book_snapshot(20, snapshot((100, 2), (102, 9)));
    agg.process(&trade(30, 101, 1));
    // `book()` reflète le dernier snapshot ≤ ts courant.
    assert_eq!(agg.book().unwrap().best_bid(), Some((100, 2)));
    assert_eq!(agg.book().unwrap().best_ask(), Some((102, 9)));

    agg.finish();
    // Flux trié → aucun désordre temporel.
    assert_eq!(agg.out_of_order_count(), 0);
}

// ---- Issue #18 : snapshot du carnet à la clôture de barre ------------------

/// `(best_bid, best_ask)` capturés à une clôture de barre.
type BestQuotes = (Option<i64>, Option<i64>);

/// Abonné capturant `(best_bid, best_ask)` du carnet **au ts de clôture** de chaque barre.
#[derive(Clone)]
struct BookAtClose(Rc<RefCell<Vec<BestQuotes>>>);
impl Subscriber for BookAtClose {
    fn on_bar_close(&mut self, _period: &str, _bar: &Bar) {}
    fn on_bar_close_with_book(&mut self, _period: &str, _bar: &Bar, book: Option<&OrderBook>) {
        let bb = book.and_then(|b| b.best_bid()).map(|(p, _)| p);
        let ba = book.and_then(|b| b.best_ask()).map(|(p, _)| p);
        self.0.borrow_mut().push((bb, ba));
    }
}

// UC-T5-5 : à la clôture, l'abonné reçoit le carnet échantillonné au ts de clôture.
#[test]
fn book_snapshot_at_bar_close() {
    let captured = BookAtClose(Rc::new(RefCell::new(Vec::new())));
    let mut agg = SymbolAggregator::builder(
        Instrument {
            id: 1,
            tick_size: 1,
        },
        Granularity::L2,
    )
    .with_period_and_lenses(
        Box::new(trade_aggregator::AlignedTimePeriod::new(100)),
        vec![],
    )
    .with_passive()
    .build()
    .unwrap();
    agg.subscribe(Box::new(captured.clone()));

    // Flux fusionné trié par ts (carnet avant trade à ts égal).
    agg.ingest_book_snapshot(0, snapshot((99, 1), (101, 1)));
    agg.process(&trade(10, 100, 1));
    agg.ingest_book_snapshot(50, snapshot((100, 1), (102, 1)));
    agg.process(&trade(60, 100, 1));
    // Ce trade @120 ferme la barre [0,100) ; le book reflète l'ingest @50.
    agg.process(&trade(120, 100, 1));
    agg.ingest_book_snapshot(150, snapshot((105, 1), (107, 1)));
    agg.finish();

    let rows = captured.0.borrow();
    assert_eq!(rows.len(), 2, "deux barres fermées (une pleine + flush)");
    // Clôture de [0,100) déclenchée à ts=120 : book = dernier snapshot ≤ 120 = ingest @50.
    assert_eq!(rows[0], (Some(100), Some(102)));
    // Barre [100,200) flushée par finish : book = ingest @150.
    assert_eq!(rows[1], (Some(105), Some(107)));
}

// UC-T5-6 : rétro-compatibilité — un abonné T0 (seulement on_bar_close) reçoit les
// clôtures même quand le côté passif est actif (délégation par défaut).
#[test]
fn legacy_subscriber_still_receives_closes_with_passive() {
    #[derive(Clone)]
    struct Counter(Rc<RefCell<usize>>);
    impl Subscriber for Counter {
        fn on_bar_close(&mut self, _period: &str, _bar: &Bar) {
            *self.0.borrow_mut() += 1;
        }
    }
    let counter = Counter(Rc::new(RefCell::new(0)));
    let mut agg = SymbolAggregator::builder(
        Instrument {
            id: 1,
            tick_size: 1,
        },
        Granularity::L2,
    )
    .with_time_period(100)
    .with_passive()
    .build()
    .unwrap();
    agg.subscribe(Box::new(counter.clone()));

    agg.ingest_book_snapshot(0, snapshot((99, 1), (101, 1)));
    agg.process(&trade(10, 100, 1));
    agg.process(&trade(120, 100, 1)); // ferme la 1ʳᵉ barre
    agg.finish(); // flush la 2ᵉ
    assert_eq!(*counter.0.borrow(), 2);
}

// UC-T5-3 : l'ingestion participe à la détection de désordre temporel (TR-5).
#[test]
fn ingest_snapshot_counts_in_out_of_order() {
    let mut agg = SymbolAggregator::builder(
        Instrument {
            id: 1,
            tick_size: 1,
        },
        Granularity::L2,
    )
    .with_time_period(100)
    .with_passive()
    .build()
    .unwrap();

    agg.process(&trade(50, 100, 1));
    // Snapshot daté *avant* le dernier event → compté en désordre.
    agg.ingest_book_snapshot(40, snapshot((99, 1), (101, 1)));
    assert_eq!(agg.out_of_order_count(), 1);
}

// UC-T5-4 : sans côté passif, l'ingestion est un no-op sûr (book() == None).
#[test]
fn ingest_snapshot_without_passive_is_noop() {
    let mut agg = SymbolAggregator::builder(
        Instrument {
            id: 1,
            tick_size: 1,
        },
        Granularity::L1,
    )
    .with_time_period(100)
    .build()
    .unwrap();

    agg.ingest_book_snapshot(5, snapshot((99, 1), (101, 1)));
    agg.process(&trade(10, 100, 1));
    assert!(agg.book().is_none());
    // L'ingestion compte tout de même dans le suivi temporel.
    assert_eq!(agg.out_of_order_count(), 0);
}
