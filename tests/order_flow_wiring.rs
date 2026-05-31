//! T1 Lot A — câblage des lentilles dans le `SymbolAggregator` (UC-T1-2).

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

#[test]
fn lenses_are_composed_and_attached_to_bars() {
    let rec = Recorder(Rc::new(RefCell::new(Vec::new())));
    let mut agg = SymbolAggregator::builder(INSTR, Granularity::L1)
        .with_period_and_lenses(
            Box::new(TimePeriod::new(100)),
            vec![
                LensKind::Footprint,
                LensKind::VolumeProfile {
                    value_area_pct: 0.7,
                },
                LensKind::Delta,
            ],
        )
        .build()
        .unwrap();
    agg.subscribe(Box::new(rec.clone()));

    // Barre [0,100) : Buy@100×2, Sell@100×5, Buy@101×1  → delta -2, poc 100.
    agg.process(&trade(0, 100, 2, AggressorSide::Buy));
    agg.process(&trade(10, 100, 5, AggressorSide::Sell));
    agg.process(&trade(50, 101, 1, AggressorSide::Buy));
    // Barre [100,200) : Buy@102×3 → delta +3.
    agg.process(&trade(100, 102, 3, AggressorSide::Buy));
    agg.finish();

    let bars = rec.0.borrow();
    assert_eq!(bars.len(), 2);

    // Barre 1 : order flow attaché.
    let of1 = &bars[0].orderflow;
    let fp = of1.footprint.as_ref().expect("footprint présent");
    assert_eq!(fp.cell(100), (2, 5));
    assert_eq!(fp.cell(101), (1, 0));
    assert_eq!(
        of1.volume_profile.as_ref().and_then(|vp| vp.poc()),
        Some(100)
    );
    assert_eq!(of1.delta, Some(-2));
    assert_eq!(of1.cvd, Some(-2));

    // Barre 2 : CVD cumulé inter-barres (-2 + 3 = 1).
    let of2 = &bars[1].orderflow;
    assert_eq!(of2.delta, Some(3));
    assert_eq!(of2.cvd, Some(1));
    assert!(bars[1].partial, "dernière barre fermée par finish()");
}

#[test]
fn no_lenses_means_empty_orderflow() {
    let rec = Recorder(Rc::new(RefCell::new(Vec::new())));
    let mut agg = SymbolAggregator::builder(INSTR, Granularity::L1)
        .with_time_period(100)
        .build()
        .unwrap();
    agg.subscribe(Box::new(rec.clone()));
    agg.process(&trade(0, 100, 1, AggressorSide::Buy));
    agg.finish();

    let bars = rec.0.borrow();
    assert_eq!(bars.len(), 1);
    assert_eq!(
        bars[0].orderflow,
        trade_aggregator::orderflow::OrderFlow::default()
    );
}
