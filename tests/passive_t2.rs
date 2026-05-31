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

fn bupd(ts: i64, action: BookAction, side: BookSide, price: i64, size: u64) -> MarketEvent {
    MarketEvent::BookUpdate(BookUpdate {
        ts,
        action,
        side,
        price,
        size,
        order_id: None,
        instrument_id: 1,
    })
}

// UC-T2-8..13 : profil de liquidité périodique (fenêtre alignée).
#[test]
fn liquidity_profile_window() {
    use BookAction::Add;
    use BookSide::{Ask, Bid};

    let mut agg = SymbolAggregator::builder(INSTR, Granularity::L2)
        .with_liquidity_profile(100)
        .build()
        .unwrap();

    agg.process(&bupd(0, Add, Bid, 100, 10)); // fenêtre [0,100) ouvre (book vide)
    agg.process(&bupd(50, Add, Ask, 101, 10));
    agg.process(&bupd(100, Add, Bid, 100, 5)); // ferme [0,100), ouvre [100,200)
    agg.finish(); // ferme [100,200) (partielle)

    let profs = agg.drain_liquidity_profiles();
    assert_eq!(profs.len(), 2);

    let p0 = &profs[0];
    assert_eq!((p0.start, p0.end), (0, 100));
    assert_eq!(p0.add_volume, 20); // LP-3 churn
    assert_eq!(p0.cancel_volume, 0);
    // LP-1/LP-4 pondéré-temps : bid 10 sur [0,100) = 10.0 ; ask 10 sur [50,100) = 5.0.
    assert_eq!(p0.tw_bid, 10.0);
    assert_eq!(p0.tw_ask, 5.0);
    // LP-5 déséquilibre = (10-5)/15.
    assert!((p0.imbalance() - (5.0 / 15.0)).abs() < 1e-9);
    // LP-2 snapshots : ouverture vide, clôture avec les deux côtés.
    assert_eq!(p0.open.best_bid(), None);
    assert_eq!(p0.close.best_ask(), Some((101, 10)));
    assert!(!p0.partial);

    assert!(profs[1].partial, "dernière fenêtre fermée par finish()");
}
