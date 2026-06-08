# trade-aggregator

> Crate **Rust** d'agrégation de données de marché — version **0.3.0**.

Transforme un flux de données de marché brutes (tape + carnet) en **données agrégées
riches** — order flow agressif et profils de liquidité passifs — en **temps réel** et en
**replay**, sous un modèle **déterministe** (event-time). La crate **agrège et expose** ;
elle **n'interprète pas** (pas d'indicateurs, pas de signaux).

## Ce qu'elle fait

- **Côté agressif** : périodes variées (time, aligned, tick, volume, dollar, range, renko,
  **renko sur grille**, **imbalance**, **run**, point & figure) → barres portant l'**order
  flow** (footprint + **fenêtre à largeur fixe**, delta/CVD, **TradeCount**, **VWAP**,
  volume profile → POC/Value Area, **TPO/Market Profile**).
- **Côté passif** : reconstruction du **carnet** (L2, et L3→L2 fidèle via `MboBook`) +
  **profils de liquidité** périodiques (pondéré-temps, churn, depth, déséquilibre).
- **Entrée** : un format canonique unique ; mapping **DataBento** (`dbn` — trades, MBP-10,
  MBO) isolé derrière la feature `databento`, dont un **replay fusionné event-time**
  (`replay_merged` : trades + carnet dans un seul agrégateur).
- **Sortie** : point d'extension réactif (`Subscriber` : `on_bar_close` /
  `on_bar_close_with_book` (carnet échantillonné à la clôture) / `on_bar_update`,
  `ChannelSink`, closures, helper `replay_to_bars`) — branchez vos propres calculs.
- **Interrogation à la demande** (primitif *screenshot* tick-by-tick) : order flow de la
  barre **en formation** (`forming_orderflow` / `forming_bar`), **historique FIFO** opt-in
  des X dernières barres fermées par frame (`with_history`), et `snapshot()` de l'état
  multi-frame complet (`[≤X fermées] + [en formation]`).

## Démarrage

```rust
use trade_aggregator::{SymbolAggregator, Instrument, Granularity, MarketEvent, Trade, AggressorSide};
use trade_aggregator::orderflow::LensKind;

let instr = Instrument { id: 42, tick_size: 25 };
let mut agg = SymbolAggregator::builder(instr, Granularity::L1)
    .with_period_and_lenses(
        Box::new(trade_aggregator::TimePeriod::new(60_000_000_000)), // barres 1 min
        vec![LensKind::Footprint, LensKind::Delta],
    )
    .build()
    .unwrap();
// agg.subscribe(...) ; agg.process(&MarketEvent::Trade(...)) ; agg.finish();
```

## Tests

```
cargo test                      # tests unitaires + intégration
cargo test --features databento # + mapping DataBento
TRADE_AGG_DATA_DIR=<racine> cargo test --features databento   # + replay sur données réelles
cargo test --release --test bench -- --ignored --nocapture    # benchmark hot path
```

Layout attendu pour le replay réel : `<racine>/<SYMBOLE>/<schéma>/glbx-mdp3-*.<schéma>.dbn.zst`.

## Documentation

Conçue **doc-first** (cf. [`docs/`](docs/)) : [vision](docs/vision/README.md) ·
[domaine](docs/domain/glossaire.md) · [architecture](docs/architecture/README.md) ·
[roadmap](docs/roadmap.md). Suivi sur le [Project #18](https://github.com/users/Morgiver/projects/18).

## Non-goals

Pas d'indicateurs/signaux (interprétation = autre projet), pas de connecteurs exchange
(réseau), pas de stockage/backtest.
