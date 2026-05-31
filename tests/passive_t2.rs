//! T2 Lot A — routage des BookUpdate vers le côté passif + fail-fast L1.

use trade_aggregator::{
    BookAction, BookSide, BookUpdate, ConfigError, Granularity, Instrument, MarketEvent,
    SymbolAggregator,
};

const INSTR: Instrument = Instrument {
    id: 1,
    tick_size: 1,
};

fn book_add(side: BookSide, price: i64, size: u64) -> MarketEvent {
    MarketEvent::BookUpdate(BookUpdate {
        ts: 0,
        action: BookAction::Add,
        side,
        price,
        size,
        order_id: None,
        instrument_id: 1,
    })
}

// UC-T2-6 : un BookUpdate est routé vers le côté passif et met à jour le carnet.
#[test]
fn book_updates_are_routed_to_passive() {
    let mut agg = SymbolAggregator::builder(INSTR, Granularity::L2)
        .with_passive()
        .build()
        .unwrap();
    agg.process(&book_add(BookSide::Bid, 100, 5));
    agg.process(&book_add(BookSide::Ask, 101, 3));

    let book = agg.book().expect("côté passif actif");
    assert_eq!(book.best_bid(), Some((100, 5)));
    assert_eq!(book.best_ask(), Some((101, 3)));
}

// UC-T2-7 : fail-fast si le côté passif est demandé en L1.
#[test]
fn passive_requires_l2_or_more() {
    let res = SymbolAggregator::builder(INSTR, Granularity::L1)
        .with_passive()
        .build();
    assert_eq!(
        res.err(),
        Some(ConfigError::IncompatibleGranularity {
            required: Granularity::L2,
            declared: Granularity::L1,
        })
    );
}

// Sans `with_passive`, pas de carnet.
#[test]
fn no_passive_means_no_book() {
    let agg = SymbolAggregator::builder(INSTR, Granularity::L2)
        .with_time_period(100)
        .build()
        .unwrap();
    assert!(agg.book().is_none());
}
