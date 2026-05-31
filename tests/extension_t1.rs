//! T1 Lot C — point d'extension (on_bar_update, channel, closure).

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;

use trade_aggregator::{
    AggressorSide, Bar, ChannelSink, FnSubscriber, Granularity, Instrument, MarketEvent,
    Subscriber, SymbolAggregator, Trade,
};

const INSTR: Instrument = Instrument {
    id: 1,
    tick_size: 1,
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

// EXT-2 / AGG-B3 : on_bar_update appelé à chaque trade intégré.
#[derive(Clone, Default)]
struct Counts(Rc<RefCell<(usize, usize)>>); // (updates, closes)
impl Subscriber for Counts {
    fn on_bar_close(&mut self, _p: &str, _b: &Bar) {
        self.0.borrow_mut().1 += 1;
    }
    fn on_bar_update(&mut self, _p: &str, _b: &Bar) {
        self.0.borrow_mut().0 += 1;
    }
}

#[test]
fn on_bar_update_fires_per_trade() {
    let c = Counts::default();
    let mut agg = SymbolAggregator::builder(INSTR, Granularity::L1)
        .with_time_period(100)
        .build()
        .unwrap();
    agg.subscribe(Box::new(c.clone()));
    // 3 trades dans [0,100) puis 1 à ts=100 (ferme la 1ʳᵉ barre).
    agg.process(&trade(0, 100, 1));
    agg.process(&trade(10, 101, 1));
    agg.process(&trade(50, 102, 1));
    agg.process(&trade(100, 103, 1));
    let (updates, closes) = *c.0.borrow();
    assert_eq!(updates, 4, "un update par trade intégré");
    assert_eq!(closes, 1, "une barre fermée (la [0,100))");
}

// EXT-4 / EXT-5 : channel push + consommation pull via le Receiver (itérateur).
#[test]
fn channel_sink_pushes_closed_bars() {
    let (tx, rx) = mpsc::channel::<(String, Bar)>();
    let mut agg = SymbolAggregator::builder(INSTR, Granularity::L1)
        .with_time_period(100)
        .build()
        .unwrap();
    agg.subscribe(Box::new(ChannelSink::new(tx)));
    agg.process(&trade(0, 100, 1));
    agg.process(&trade(100, 101, 1)); // ferme [0,100)
    agg.process(&trade(250, 102, 1)); // ferme [100,200)
    agg.finish(); // ferme [200,300) (partielle)
    drop(agg); // lâche le Sender → le Receiver se termine

    let bars: Vec<(String, Bar)> = rx.into_iter().collect(); // pull (itérateur)
    assert_eq!(bars.len(), 3);
    assert!(bars.iter().all(|(label, _)| label == "time:100ns"));
    assert!(bars.last().unwrap().1.partial);
}

// EXT-1 : adaptateur closure.
#[test]
fn fn_subscriber_closure() {
    let seen = Rc::new(RefCell::new(0usize));
    let seen2 = seen.clone();
    let mut agg = SymbolAggregator::builder(INSTR, Granularity::L1)
        .with_time_period(100)
        .build()
        .unwrap();
    agg.subscribe(Box::new(FnSubscriber(move |_p: &str, _b: &Bar| {
        *seen2.borrow_mut() += 1;
    })));
    agg.process(&trade(0, 100, 1));
    agg.process(&trade(100, 101, 1));
    assert_eq!(*seen.borrow(), 1);
}
