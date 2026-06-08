# T6 — Use-cases (primitif screenshot)

Tranche **T6** (origine *genetic-trading*). Cible **v0.3.0**, additif & rétro-compatible.
Format : *ID · comportement · critères · tests qui le couvrent · issue*.

> Non-goal tenu : on n'expose que de l'état **déjà calculé** ; jamais d'interprétation.
> Invariants : coût **à la demande** (hot path inchangé) ; historique **opt-in** (empreinte
> mémoire inchangée par défaut).

---

## t6.1 — Order flow d'une barre en formation (issue #31)

Nœud d'archi : `symbol-aggregator` + `aggressor/orderflow`.

| ID | Comportement | Critères d'acceptation | Tests |
|----|--------------|------------------------|-------|
| **UC-T6-1** | `forming_orderflow(label)` renvoie l'order flow **courant** de la barre en formation (footprint, delta, CVD courant, TradeCount, VWAP). | reflète tous les trades depuis l'ouverture ; `None` si label inconnu / pas de barre ouverte. | `screenshot_t6.rs::forming_orderflow_reflects_trades_so_far` |
| **UC-T6-2** | Lecture seule : `&self`, idempotent, ne clôture ni ne mute ; cohérent avec l'`OrderFlow` produit à la clôture si appelé juste avant ; CVD courant = cumul fermées + delta courant (pas de double comptage). | appels répétés identiques ; `forming == closed.orderflow` ; CVD correct après clôtures. | `screenshot_t6.rs::forming_is_readonly_and_consistent_with_close` |
| **UC-T6-3** | `forming_bar(label)` : barre en formation complète (`OHLCV` courant + order flow courant), marquée `partial`. Multi-frames par label. | `forming_bar` OHLCV/partial corrects ; interrogeable par label indépendamment. | `screenshot_t6.rs::forming_orderflow_reflects_trades_so_far`, `::forming_orderflow_multi_frame` |

---

## t6.2 — Historique FIFO + `snapshot()` (issue #32)

Nœud d'archi : `symbol-aggregator`.

| ID | Comportement | Critères d'acceptation | Tests |
|----|--------------|------------------------|-------|
| **UC-T6-4** | Historique **opt-in** : sans `with_history`/`with_period_lenses_history`, aucune rétention (`history()` = `None`, empreinte mémoire inchangée). | `history()` = `None` sans activation ; `snapshot()` expose quand même la barre en formation, `closed` vide. | `screenshot_t6.rs::history_is_opt_in` |
| **UC-T6-5** | Ring buffer **FIFO borné à `depth`** par période (la plus ancienne tombe), du plus ancien au plus récent. | `history()` ≤ `depth` ; ordre ancien→récent ; profondeur par période ou globale. | `screenshot_t6.rs::history_fifo_bounded_to_depth` |
| **UC-T6-6** | `snapshot() -> Vec<FrameSnapshot>` : par frame, `[≤X barres fermées] + [barre en formation]` ; multi-frames simultanés. | un `FrameSnapshot` par période ; `closed` + `forming` corrects ; frames indépendants. | `screenshot_t6.rs::snapshot_multi_frame_closed_plus_forming` |
| **UC-T6-7** | Déterminisme & cohérence : l'historique == les barres reçues par les abonnés à la clôture. | `history()` == séquence notifiée (`on_bar_close`). | `screenshot_t6.rs::history_matches_subscriber_closes` |
