//! T6 — Primitif « screenshot » (issues #31/#32).
//!
//! #31 : order flow interrogeable d'une barre **en formation** (`forming_orderflow`,
//! `forming_bar`), en lecture seule, sans clôturer. Tests synthétiques.

use std::cell::RefCell;
use std::rc::Rc;

use trade_aggregator::orderflow::LensKind;
use trade_aggregator::{
    AggressorSide, Bar, Granularity, Instrument, MarketEvent, Subscriber, SymbolAggregator,
    TickPeriod, TimePeriod, Trade,
};

const INSTR: Instrument = Instrument {
    id: 1,
    tick_size: 1,
};
const LABEL: &str = "time:100ns";

fn trade(ts: i64, price: i64, size: u64, side: AggressorSide) -> MarketEvent {
    MarketEvent::Trade(Trade {
        ts,
        price,
        size,
        aggressor: side,
        instrument_id: 1,
    })
}

#[derive(Clone)]
struct Recorder(Rc<RefCell<Vec<Bar>>>);
impl Subscriber for Recorder {
    fn on_bar_close(&mut self, _period: &str, bar: &Bar) {
        self.0.borrow_mut().push(bar.clone());
    }
}

fn agg_with_lenses() -> SymbolAggregator {
    SymbolAggregator::builder(INSTR, Granularity::L1)
        .with_period_and_lenses(
            Box::new(TimePeriod::new(100)),
            vec![
                LensKind::Footprint,
                LensKind::Delta,
                LensKind::TradeCount,
                LensKind::Vwap,
            ],
        )
        .build()
        .unwrap()
}

// UC-T6-1 : l'order flow en formation reflète tous les trades depuis l'ouverture.
#[test]
fn forming_orderflow_reflects_trades_so_far() {
    let mut agg = agg_with_lenses();
    // Pas de barre ouverte → None.
    assert!(agg.forming_orderflow(LABEL).is_none());

    agg.process(&trade(0, 100, 2, AggressorSide::Buy));
    agg.process(&trade(10, 100, 3, AggressorSide::Sell));

    let of = agg.forming_orderflow(LABEL).expect("barre en formation");
    assert_eq!(of.delta, Some(-1), "2 - 3");
    assert_eq!(
        of.cvd,
        Some(-1),
        "aucune barre fermée → cumul = delta courant"
    );
    assert_eq!(of.trade_count, Some((1, 1)));
    assert_eq!(of.footprint.as_ref().unwrap().cell(100), (2, 3));
    assert_eq!(of.vwap, Some(100.0)); // (200+300)/5

    // Barre en formation complète.
    let bar = agg.forming_bar(LABEL).unwrap();
    assert_eq!(bar.ohlcv.volume, 5);
    assert_eq!(bar.ohlcv.close, 100);
    assert!(bar.partial, "barre en formation = partielle");

    // Label inconnu → None.
    assert!(agg.forming_orderflow("nope").is_none());
}

// UC-T6-2 : forming_orderflow est en lecture seule (idempotent, ne mute pas l'état) et
// cohérent avec l'OrderFlow produit à la clôture si appelé juste avant.
#[test]
fn forming_is_readonly_and_consistent_with_close() {
    let rec = Recorder(Rc::new(RefCell::new(Vec::new())));
    let mut agg = agg_with_lenses();
    agg.subscribe(Box::new(rec.clone()));

    agg.process(&trade(0, 100, 2, AggressorSide::Buy));
    agg.process(&trade(10, 100, 3, AggressorSide::Sell));

    // Appels répétés → identiques (aucune mutation).
    let a = agg.forming_orderflow(LABEL).unwrap();
    let b = agg.forming_orderflow(LABEL).unwrap();
    assert_eq!(a, b);

    // Ce trade @150 ferme la barre [0,100) (sans inclure ce trade).
    agg.process(&trade(150, 101, 4, AggressorSide::Buy));

    let closed = &rec.0.borrow()[0];
    // forming juste avant == order flow de la barre fermée.
    assert_eq!(a, closed.orderflow);

    // Barre 2 en formation : delta +4, cvd cumulé (-1 + 4 = 3) — pas de double comptage.
    let of2 = agg.forming_orderflow(LABEL).unwrap();
    assert_eq!(of2.delta, Some(4));
    assert_eq!(of2.cvd, Some(3));
    assert_eq!(of2.trade_count, Some((1, 0)));
}

// UC-T6-3 : multi-frames — forming interrogeable indépendamment par label.
#[test]
fn forming_orderflow_multi_frame() {
    let mut agg = SymbolAggregator::builder(INSTR, Granularity::L1)
        .with_period_and_lenses(Box::new(TimePeriod::new(100)), vec![LensKind::Delta])
        .with_period_and_lenses(Box::new(TimePeriod::new(1000)), vec![LensKind::Delta])
        .build()
        .unwrap();

    agg.process(&trade(0, 100, 2, AggressorSide::Buy));
    agg.process(&trade(150, 100, 1, AggressorSide::Sell)); // ferme la frame 100ns, pas la 1000ns

    // Frame 100ns : nouvelle barre (le trade @150 l'a ouverte) → delta -1.
    assert_eq!(agg.forming_orderflow("time:100ns").unwrap().delta, Some(-1));
    // Frame 1000ns : toujours la 1ʳᵉ barre → delta +2 -1 = +1.
    assert_eq!(agg.forming_orderflow("time:1000ns").unwrap().delta, Some(1));
}

// ---- Issue #32 : historique FIFO + snapshot() ------------------------------

fn buy(ts: i64, price: i64) -> MarketEvent {
    trade(ts, price, 1, AggressorSide::Buy)
}

// UC-T6-4 : opt-in — sans with_history, aucune rétention (history == None).
#[test]
fn history_is_opt_in() {
    let mut agg = SymbolAggregator::builder(INSTR, Granularity::L1)
        .with_time_period(100)
        .build()
        .unwrap();
    agg.process(&buy(0, 100));
    agg.process(&buy(150, 101)); // ferme la 1ʳᵉ barre
    assert!(
        agg.history(LABEL).is_none(),
        "pas d'historique sans with_history"
    );
    // snapshot() expose quand même la barre en formation, closed vide.
    let snap = agg.snapshot();
    assert_eq!(snap.len(), 1);
    assert!(snap[0].closed.is_empty());
    assert!(snap[0].forming.is_some());
}

// UC-T6-5 : FIFO borné à depth (la plus ancienne tombe), du plus ancien au plus récent.
#[test]
fn history_fifo_bounded_to_depth() {
    let mut agg = SymbolAggregator::builder(INSTR, Granularity::L1)
        .with_period_lenses_history(Box::new(TickPeriod::new(1)), vec![], 3)
        .build()
        .unwrap();
    // TickPeriod(1) : chaque trade ferme la barre précédente et en ouvre une.
    for (i, px) in [100, 101, 102, 103, 104].into_iter().enumerate() {
        agg.process(&buy(i as i64, px));
    }
    // 5 barres ouvertes → 4 fermées (la 5ᵉ est en formation). FIFO depth 3 → garde les 3 dernières fermées.
    let h = agg.history("tick:1").unwrap();
    let closes: Vec<i64> = h.iter().map(|b| b.ohlcv.close).collect();
    assert_eq!(
        closes,
        vec![101, 102, 103],
        "3 dernières fermées, plus ancienne→récente"
    );
}

// UC-T6-6 : snapshot() = par frame, [≤X fermées] + [barre en formation] ; multi-frames.
#[test]
fn snapshot_multi_frame_closed_plus_forming() {
    let mut agg = SymbolAggregator::builder(INSTR, Granularity::L1)
        .with_history(5) // défaut global
        .with_period_and_lenses(Box::new(TimePeriod::new(100)), vec![LensKind::Delta])
        .with_period_and_lenses(Box::new(TimePeriod::new(1000)), vec![LensKind::Delta])
        .build()
        .unwrap();

    // Trades sur [0,100), puis un trade @150 ferme la frame 100ns (pas la 1000ns).
    agg.process(&buy(0, 100));
    agg.process(&buy(50, 101));
    agg.process(&buy(150, 102));

    let snap = agg.snapshot();
    assert_eq!(snap.len(), 2);

    let f100 = snap.iter().find(|f| f.label == "time:100ns").unwrap();
    assert_eq!(f100.closed.len(), 1, "1 barre fermée sur la frame 100ns");
    assert_eq!(f100.closed[0].ohlcv.close, 101);
    assert!(f100.forming.is_some(), "barre en formation présente");
    assert_eq!(f100.forming.as_ref().unwrap().ohlcv.close, 102);

    let f1000 = snap.iter().find(|f| f.label == "time:1000ns").unwrap();
    assert!(
        f1000.closed.is_empty(),
        "aucune barre fermée sur la frame 1000ns"
    );
    assert!(f1000.forming.is_some());
}

// UC-T6-7 : l'historique est cohérent avec ce que les abonnés ont reçu à la clôture.
#[test]
fn history_matches_subscriber_closes() {
    let rec = Recorder(Rc::new(RefCell::new(Vec::new())));
    let mut agg = SymbolAggregator::builder(INSTR, Granularity::L1)
        .with_history(10)
        .with_period_and_lenses(Box::new(TimePeriod::new(100)), vec![LensKind::Delta])
        .build()
        .unwrap();
    agg.subscribe(Box::new(rec.clone()));

    for i in 0..5 {
        agg.process(&buy(i * 150, 100 + i));
    }
    agg.finish();

    let received = rec.0.borrow();
    let history: Vec<Bar> = agg.history(LABEL).unwrap().iter().cloned().collect();
    assert_eq!(
        history, *received,
        "historique == barres notifiées aux abonnés"
    );
}
