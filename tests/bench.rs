//! Benchmark du hot path (fiche `UC-T4-6`). Ignoré par défaut :
//! `cargo test --release -- --ignored --nocapture` pour le lancer.

use std::time::Instant;

use trade_aggregator::orderflow::LensKind;
use trade_aggregator::{
    AggressorSide, Bar, Granularity, Instrument, MarketEvent, Subscriber, SymbolAggregator,
    TimePeriod, Trade,
};

struct Counter(u64);
impl Subscriber for Counter {
    fn on_bar_close(&mut self, _p: &str, _b: &Bar) {
        self.0 += 1;
    }
}

#[test]
#[ignore = "benchmark perf — lancer explicitement avec --ignored --release"]
fn throughput_aggressive_hot_path() {
    const N: u64 = 2_000_000;
    let instr = Instrument {
        id: 1,
        tick_size: 1,
    };
    let mut agg = SymbolAggregator::builder(instr, Granularity::L1)
        .with_period_and_lenses(
            Box::new(TimePeriod::new(60_000_000_000)),
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
    agg.subscribe(Box::new(Counter(0)));

    let start = Instant::now();
    for i in 0..N {
        let side = if i % 2 == 0 {
            AggressorSide::Buy
        } else {
            AggressorSide::Sell
        };
        let ev = MarketEvent::Trade(Trade {
            ts: i as i64 * 1000,
            price: 100 + (i % 20) as i64,
            size: 1 + (i % 5),
            aggressor: side,
            instrument_id: 1,
        });
        agg.process(&ev);
    }
    agg.finish();
    let dt = start.elapsed();
    let per_s = N as f64 / dt.as_secs_f64();
    eprintln!("hot path : {N} trades en {:?} → {:.0} trades/s", dt, per_s);
    assert!(per_s > 0.0);
}
