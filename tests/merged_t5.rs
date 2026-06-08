//! T5 — Replay fusionné event-time (issue #17, fiches `UC-T5-1..4`).
//!
//! Tests **synthétiques** des briques du flux fusionné au niveau `SymbolAggregator` :
//! ingestion de snapshots de carnet (MBP-10) intercalée avec les trades, synchronisation
//! de `book()`, et détection de désordre. Le replay DBN réel (`replay_merged`) est testé
//! sous feature `databento` + `TRADE_AGG_DATA_DIR` (cf. `databento_replay.rs`).

use trade_aggregator::{
    AggressorSide, BookSide, Granularity, Instrument, MarketEvent, OrderBook, SymbolAggregator,
    Trade,
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
