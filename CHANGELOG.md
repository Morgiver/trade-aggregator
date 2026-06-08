# Changelog

Format inspiré de [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/).
On consigne les **changements notables** (fin de phase, tranche réalisée, décision/ADR, breaking) — pas chaque commit.

## [0.3.0] — 2026-06-08

**Primitif « screenshot » (tranche T6)** — exposer l'état multi-frame complet au **tick
courant** pour des observations ML tick-by-tick (*genetic-trading*). Additif &
**rétro-compatible**. Non-goal tenu (on n'expose que de l'état déjà calculé). Invariants :
coût **à la demande** (hot path inchangé) ; historique **opt-in** (empreinte mémoire
inchangée par défaut).

### Added
- **Order flow d'une barre en formation** (#31) :
  `SymbolAggregator::forming_orderflow(label)` / `forming_bar(label)` — snapshot
  **lecture-seule** (`&self`) des lentilles vivantes (footprint, delta, CVD courant,
  TradeCount, VWAP) sans clôturer la barre ; CVD courant = cumul fermées + delta courant.
- **Historique FIFO + screenshot** (#32) : ring buffer borné **opt-in** des dernières barres
  fermées par période (`Builder::with_history(depth)` /
  `with_period_lenses_history(period, lenses, depth)`) ;
  `SymbolAggregator::history(label)` ; `snapshot() -> Vec<FrameSnapshot>`
  (`[≤X fermées] + [barre en formation]` par frame).

## [0.2.0] — 2026-06-08

**Itération consommateur (tranche T5)** — enrichissements nés du premier vrai consommateur
du point d'extension (*genetic-trading* : observations par barre, agressif + passif
synchronisés). Additif & **rétro-compatible**. Non-goals tenus (agrège/expose, n'interprète
pas). ~58 tests (unit + intégration, dont replay réel gated).

### Added
- **Replay fusionné event-time** (#17) : `replay_merged(trades, book, agg, limit)` — k-way
  merge par `ts` de deux fichiers DBN (trades + MBP-10/MBO) dans un **seul**
  `SymbolAggregator` ; départage déterministe « carnet avant trade ».
  `SymbolAggregator::ingest_book_snapshot` + `PassiveAggregator::replace_book`.
- **Snapshot du carnet à la clôture de barre** (#18) :
  `Subscriber::on_bar_close_with_book(period, bar, book)` — carnet échantillonné au `ts` de
  clôture (impl par défaut déléguant à `on_bar_close`, rétro-compatible).
- **Agrégations pures** (#19) : lentilles `TradeCount` (buy/sell) et `Vwap`, activables via
  `LensKind`, exposées dans `OrderFlow`.
- **Vue footprint à largeur fixe** (#20) : `Footprint::window(anchor, tick_size, half_width)`
  → `2·half_width+1` cellules indexées par offset de tick.
- **Renko sur grille** (#21) : `RenkoBrickPeriod` — briques alignées sur grille, sauts
  multi-briques, borne d'excursion explicite (`2·brick−1`). `RenkoPeriod` conservé.
- **Helper DX** (#22) : `replay_to_bars(path, agg, limit) -> Vec<Bar>` (feature `databento`).

## [0.1.0] — 2026-05-31

Première version fonctionnelle de bout en bout — **roadmap (T0→T4) épuisée**.

### Phase 7 — Réalisation (tranches)
- **T0 — Walking skeleton** : `trades → barres temporelles → on_bar_close`, en replay.
- **T1 — Cœur agressif** : order flow (Footprint, Delta/CVD, VolumeProfile→POC/Value Area),
  périodes (Time/Aligned/Tick/Volume/Dollar/Range/Renko), point d'extension
  (`on_bar_update`, `ChannelSink`, `FnSubscriber`).
- **T2 — Côté passif** : reconstruction du carnet (`OrderBook` L2), `LiquidityProfile`
  périodique (churn, pondéré-temps, déséquilibre, snapshots), mapping DataBento MBP-10
  (validé sur NQ réel).
- **T3 — Différenciation** : imbalance bars (tick/volume/dollar), run bars, TPO/Market
  Profile (POC temps, Value Area, single prints, Initial Balance), Point & Figure,
  détection de désordre temporel.
- **T4 — Robustesse & perf** : reconstruction L3→L2 fidèle (`MboBook`), bornes mémoire
  (`prune_to_depth`), benchmark hot path (~23 M trades/s).

### Périmètre
Crate Rust (édition 2024), agrégation déterministe (live = replay, event-time), côté
agressif + passif, mapping **DataBento** isolé derrière la feature `databento`. ~42 tests.
Non-goals tenus : pas d'indicateurs/interprétation, pas de connecteurs réseau.

## [Non publié]

### Added
- Amorçage du projet : repo privé, squelette `docs/ideation/`, board Project #18, issue Phase 1.
- **Phase 1 — Découverte** : idée cadrée dans `docs/ideation/idea.md` (modèle
  SymbolAggregator / Aggressor / Passive, entrée DataBento L3/MBO, frontière de scope
  agrégation-vs-interprétation, live+replay event-time). Issue #1 close.
- **Phase 2 — Vision** : `docs/vision/` complet (produit, positionnement, 5 piliers,
  scope IN/OUT, features priorisées P0→P3, tranches macro T0→T4, risques & questions).
  Issue #2 close.
- **Phase 3 — Domaine** : `docs/domain/` (glossaire EN/FR + concepts & relations
  Mermaid). Vocabulaire stable. Issue #3 close.
- **Phase 4 — Architecture** : `docs/architecture/` complète (descente C4 : racine,
  canonical, symbol-aggregator, aggressor/orderflow, passive, extension, transverse).
  Chaque feature a un toit. Issue #4 close.
- **Phase 5 — Structuration** : 98 fiches atomiques co-localisées dans
  `docs/architecture/` (aucune feature orpheline). Issue #5 close.
- **Phase 6 — Priorisation** : `docs/roadmap.md` (tranches T0→T4 ordonnées). Issue #6
  close. **Partie documentaire (Phases 1→6) terminée.**
- Démarrage de la **Phase 7 — Réalisation** (issue #7).
- **Tranche T0 — walking skeleton** : crate Rust (édition 2024) — modèle canonique,
  `TimePeriod`, `Bar`/OHLCV, `SymbolAggregator` (routage, fan-out, fail-fast, flush),
  `Subscriber`, et mapping **DataBento** (`dbn`, feature `databento`). Pipeline
  `trades → barres temporelles → on_bar_close` en replay. Tests : 7 d'intégration
  synthétiques + 1 réel optionnel (validé sur NQ : 8703 trades → 60 barres 1-min).
