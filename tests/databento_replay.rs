//! Test d'intégration **optionnel** sur de vraies données DataBento (fiche `UC-T0-1`, R).
//!
//! Activé seulement avec la feature `databento` ET la variable d'environnement
//! `TRADE_AGG_DATA_DIR` pointant vers une racine structurée :
//! `<root>/<SYMBOL>/trades/glbx-mdp3-*.trades.dbn.zst`.
//! Sinon, le test est **skippé** (pas d'échec).
#![cfg(feature = "databento")]

use std::env;
use std::fs;
use std::path::PathBuf;

use trade_aggregator::canonical::{Granularity, Instrument};
use trade_aggregator::databento::{first_instrument_id, replay_trades_file};
use trade_aggregator::{Bar, SymbolAggregator};

// Compteur de barres partagé (l'agrégateur prend possession de l'abonné).
use std::cell::RefCell;
use std::rc::Rc;
#[derive(Clone)]
struct SharedCounter(Rc<RefCell<usize>>);
impl trade_aggregator::Subscriber for SharedCounter {
    fn on_bar_close(&mut self, _period: &str, _bar: &Bar) {
        *self.0.borrow_mut() += 1;
    }
}

#[test]
fn replay_real_nq_trades_if_available() {
    let Ok(root) = env::var("TRADE_AGG_DATA_DIR") else {
        eprintln!("SKIP: TRADE_AGG_DATA_DIR non défini");
        return;
    };
    let symbol = env::var("TRADE_AGG_SYMBOL").unwrap_or_else(|_| "NQ".to_string());
    let dir = PathBuf::from(&root).join(&symbol).join("trades");

    // Premier fichier *.trades.dbn.zst du dossier.
    let Some(file) = fs::read_dir(&dir).ok().and_then(|rd| {
        rd.filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.to_string_lossy().ends_with(".trades.dbn.zst"))
            .min()
    }) else {
        eprintln!("SKIP: aucun *.trades.dbn.zst dans {}", dir.display());
        return;
    };

    // Un fichier mêle plusieurs échéances : on agrège la première rencontrée.
    let instrument_id = first_instrument_id(&file)
        .expect("lecture DBN")
        .expect("au moins un trade");

    let counter = SharedCounter(Rc::new(RefCell::new(0)));
    let mut agg = SymbolAggregator::builder(
        Instrument {
            id: instrument_id,
            tick_size: 1,
        },
        Granularity::L1,
    )
    .with_time_period(60_000_000_000) // barres 1 minute (ns)
    .build()
    .unwrap();
    agg.subscribe(Box::new(counter.clone()));

    let n = replay_trades_file(&file, &mut agg, Some(200_000)).expect("replay DBN");

    assert!(
        n > 0,
        "des trades doivent être lus depuis {}",
        file.display()
    );
    assert!(
        *counter.0.borrow() > 0,
        "au moins une barre doit être produite"
    );
    eprintln!(
        "OK: {} ({} trades, {} barres, instrument_id={})",
        file.display(),
        n,
        counter.0.borrow(),
        instrument_id
    );
}

// UC-T2-17 : reconstruction du book depuis MBP-10 réel (gated TRADE_AGG_DATA_DIR).
#[test]
fn reconstruct_book_from_real_mbp10_if_available() {
    use trade_aggregator::databento::{first_mbp10_instrument_id, replay_mbp10_file};

    let Ok(root) = env::var("TRADE_AGG_DATA_DIR") else {
        eprintln!("SKIP: TRADE_AGG_DATA_DIR non défini");
        return;
    };
    let symbol = env::var("TRADE_AGG_SYMBOL").unwrap_or_else(|_| "NQ".to_string());
    let dir = PathBuf::from(&root).join(&symbol).join("mbp-10");
    let Some(file) = fs::read_dir(&dir).ok().and_then(|rd| {
        rd.filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.to_string_lossy().ends_with(".mbp-10.dbn.zst"))
            .min()
    }) else {
        eprintln!("SKIP: aucun *.mbp-10.dbn.zst dans {}", dir.display());
        return;
    };

    let id = first_mbp10_instrument_id(&file)
        .expect("lecture DBN")
        .expect("au moins un message MBP-10");
    let (n, crossed) = replay_mbp10_file(&file, id, Some(200_000)).expect("replay MBP-10");

    assert!(n > 0, "des messages MBP-10 doivent être lus");
    // Un book bien reconstruit ne doit quasiment jamais être croisé.
    let ratio = crossed as f64 / n as f64;
    assert!(
        ratio < 0.01,
        "book croisé sur {crossed}/{n} messages (ratio {ratio:.4}) — reconstruction suspecte"
    );
    eprintln!(
        "OK: {} ({n} messages MBP-10, {crossed} croisés, instrument {id})",
        file.display()
    );
}

// UC-T5-1 : replay **fusionné** trades + MBP-10 dans un seul agrégateur (issue #17).
// Vérifie l'ordre event-time (aucun désordre), la synchro du book et le déterminisme.
#[test]
fn replay_merged_trades_and_mbp10_if_available() {
    use trade_aggregator::BookSide;
    use trade_aggregator::databento::replay_merged;

    let Ok(root) = env::var("TRADE_AGG_DATA_DIR") else {
        eprintln!("SKIP: TRADE_AGG_DATA_DIR non défini");
        return;
    };
    let symbol = env::var("TRADE_AGG_SYMBOL").unwrap_or_else(|_| "NQ".to_string());

    let pick = |sub: &str, suffix: &str| -> Option<PathBuf> {
        let dir = PathBuf::from(&root).join(&symbol).join(sub);
        fs::read_dir(&dir).ok().and_then(|rd| {
            rd.filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.to_string_lossy().ends_with(suffix))
                .min()
        })
    };
    let (Some(trades), Some(book)) = (
        pick("trades", ".trades.dbn.zst"),
        pick("mbp-10", ".mbp-10.dbn.zst"),
    ) else {
        eprintln!("SKIP: fichiers trades + mbp-10 requis pour le replay fusionné");
        return;
    };

    // L'agrégateur filtre par instrument : on prend l'échéance vue dans les trades.
    let instrument_id = first_instrument_id(&trades)
        .expect("lecture DBN")
        .expect("au moins un trade");

    let run = || {
        let mut agg = SymbolAggregator::builder(
            Instrument {
                id: instrument_id,
                tick_size: 1,
            },
            Granularity::L2,
        )
        .with_time_period(60_000_000_000)
        .with_passive()
        .build()
        .unwrap();
        let n = replay_merged(&trades, &book, &mut agg, Some(200_000)).expect("replay fusionné");
        let bb = agg.book().unwrap().best_bid();
        let ba = agg.book().unwrap().best_ask();
        let total = agg.book().unwrap().total_qty(BookSide::Bid);
        (n, agg.out_of_order_count(), bb, ba, total)
    };

    let first = run();
    let second = run();

    assert!(first.0 > 0, "des événements doivent être fusionnés");
    assert_eq!(first.1, 0, "flux fusionné trié → aucun désordre temporel");
    assert!(
        first.2.is_some() && first.3.is_some(),
        "le book doit être peuplé après le replay fusionné"
    );
    assert_eq!(first, second, "le replay fusionné doit être déterministe");
    eprintln!(
        "OK merged: {} évts, book best_bid={:?} best_ask={:?}",
        first.0, first.2, first.3
    );
}
