//! Tests de la tranche T0 — walking skeleton.
//! Critères d'acceptation : `docs/tranches/T0-walking-skeleton/tests.md`.

use std::cell::RefCell;
use std::rc::Rc;

use trade_aggregator::{
    AggressorSide, Bar, Boundary, ConfigError, Granularity, Instrument, MarketEvent, Period,
    Subscriber, SymbolAggregator, Trade,
};

const INSTR: Instrument = Instrument {
    id: 42,
    tick_size: 25,
};

fn trade(ts: i64, price: i64, size: u64, side: AggressorSide) -> MarketEvent {
    MarketEvent::Trade(Trade {
        ts,
        price,
        size,
        aggressor: side,
        instrument_id: 42,
    })
}

/// Abonné de test qui enregistre les barres fermées, dans l'ordre.
#[derive(Clone)]
struct Recorder(Rc<RefCell<Vec<(String, Bar)>>>);

impl Recorder {
    fn new() -> Self {
        Recorder(Rc::new(RefCell::new(Vec::new())))
    }
    fn bars(&self) -> Vec<(String, Bar)> {
        self.0.borrow().clone()
    }
}

impl Subscriber for Recorder {
    fn on_bar_close(&mut self, period: &str, bar: &Bar) {
        self.0.borrow_mut().push((period.to_string(), bar.clone()));
    }
}

/// Rejoue une séquence et renvoie les barres fermées (helper pour le déterminisme).
fn replay(events: &[MarketEvent]) -> Vec<(String, Bar)> {
    let rec = Recorder::new();
    let mut agg = SymbolAggregator::builder(INSTR, Granularity::L1)
        .with_time_period(100)
        .build()
        .unwrap();
    agg.subscribe(Box::new(rec.clone()));
    for ev in events {
        agg.process(ev);
    }
    agg.finish();
    rec.bars()
}

// --- UC-T0-3 : construction OK ------------------------------------------------
#[test]
fn uc_t0_3_build_ok() {
    let agg = SymbolAggregator::builder(INSTR, Granularity::L1)
        .with_time_period(60)
        .build();
    assert!(agg.is_ok());
}

// --- UC-T0-4 : fail-fast granularité incompatible -----------------------------
struct NeedsL3;
impl Period for NeedsL3 {
    fn on_trade(&mut self, _t: &Trade) -> Boundary {
        Boundary::Continue
    }
    fn min_granularity(&self) -> Granularity {
        Granularity::L3
    }
    fn label(&self) -> String {
        "needs-l3".to_string()
    }
}

#[test]
fn uc_t0_4_fail_fast_incompatible_granularity() {
    let res = SymbolAggregator::builder(INSTR, Granularity::L1)
        .with_period(Box::new(NeedsL3))
        .build();
    assert_eq!(
        res.err(),
        Some(ConfigError::IncompatibleGranularity {
            required: Granularity::L3,
            declared: Granularity::L1,
        })
    );
}

// --- UC-T0-2 : côté agresseur inconnu -----------------------------------------
#[test]
fn uc_t0_2_unknown_aggressor_is_accepted() {
    let bars = replay(&[trade(0, 100, 7, AggressorSide::Unknown)]);
    assert_eq!(bars.len(), 1);
    assert_eq!(bars[0].1.ohlcv.volume, 7);
}

// --- UC-T0-5/6/8 : agrégation, fermeture de barre, on_bar_close ---------------
#[test]
fn uc_t0_5_6_8_aggregation_and_close() {
    let rec = Recorder::new();
    let mut agg = SymbolAggregator::builder(INSTR, Granularity::L1)
        .with_time_period(100)
        .build()
        .unwrap();
    agg.subscribe(Box::new(rec.clone()));

    // Trois trades dans [0,100), puis un trade à ts=100 qui ferme la barre.
    agg.process(&trade(0, 100, 1, AggressorSide::Buy));
    agg.process(&trade(10, 105, 2, AggressorSide::Sell));
    agg.process(&trade(50, 95, 3, AggressorSide::Buy));
    agg.process(&trade(100, 102, 4, AggressorSide::Buy));

    let bars = rec.bars();
    assert_eq!(bars.len(), 1, "une seule barre fermée à ce stade");
    let (label, bar) = &bars[0];
    assert_eq!(label, "time:100ns");
    assert_eq!(bar.start, 0);
    assert_eq!(bar.end, 100);
    assert_eq!(bar.ohlcv.open, 100);
    assert_eq!(bar.ohlcv.high, 105);
    assert_eq!(bar.ohlcv.low, 95);
    assert_eq!(bar.ohlcv.close, 95);
    assert_eq!(bar.ohlcv.volume, 6);
    assert!(!bar.partial);
}

// --- UC-T0-7 : flush de fin de flux -------------------------------------------
#[test]
fn uc_t0_7_finish_flushes_partial_bar() {
    let bars = replay(&[
        trade(0, 100, 1, AggressorSide::Buy),
        trade(100, 102, 4, AggressorSide::Buy),
    ]);
    // Barre [0,100) fermée par la borne, puis [100,200) fermée par finish() = partielle.
    assert_eq!(bars.len(), 2);
    assert!(!bars[0].1.partial);
    assert_eq!(bars[0].1.ohlcv.close, 100);
    assert!(bars[1].1.partial, "la dernière barre est partielle");
    assert_eq!(bars[1].1.start, 100);
    assert_eq!(bars[1].1.ohlcv.volume, 4);
}

// --- UC-T3-10 : détection de désordre temporel (TR-5) -------------------------
#[test]
fn out_of_order_events_are_counted_not_rejected() {
    let mut agg = SymbolAggregator::builder(INSTR, Granularity::L1)
        .with_time_period(100)
        .build()
        .unwrap();
    agg.process(&trade(0, 100, 1, AggressorSide::Buy));
    agg.process(&trade(50, 101, 1, AggressorSide::Buy));
    agg.process(&trade(30, 102, 1, AggressorSide::Buy)); // ts recule
    assert_eq!(agg.out_of_order_count(), 1);
}

// --- UC-T0-9 : déterminisme du rejeu ------------------------------------------
#[test]
fn uc_t0_9_replay_is_deterministic() {
    let events = [
        trade(0, 100, 1, AggressorSide::Buy),
        trade(10, 105, 2, AggressorSide::Sell),
        trade(120, 95, 3, AggressorSide::Buy),
        trade(250, 102, 4, AggressorSide::Sell),
    ];
    assert_eq!(replay(&events), replay(&events));
}

// --- Filtrage par instrument --------------------------------------------------
#[test]
fn trades_of_other_instruments_are_ignored() {
    let other = MarketEvent::Trade(Trade {
        ts: 5,
        price: 999,
        size: 100,
        aggressor: AggressorSide::Buy,
        instrument_id: 7, // != 42
    });
    let bars = replay(&[trade(0, 100, 1, AggressorSide::Buy), other]);
    assert_eq!(bars.len(), 1);
    assert_eq!(
        bars[0].1.ohlcv.volume, 1,
        "le trade d'un autre instrument est ignoré"
    );
}
