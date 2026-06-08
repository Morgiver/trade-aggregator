//! T6 — Primitif « screenshot » (issues #31/#32).
//!
//! #31 : order flow interrogeable d'une barre **en formation** (`forming_orderflow`,
//! `forming_bar`), en lecture seule, sans clôturer. Tests synthétiques.

use std::cell::RefCell;
use std::rc::Rc;

use trade_aggregator::orderflow::LensKind;
use trade_aggregator::{
    AggressorSide, Bar, Granularity, Instrument, MarketEvent, Subscriber, SymbolAggregator,
    TimePeriod, Trade,
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
