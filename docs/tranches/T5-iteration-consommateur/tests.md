# T5 — Tests (itération consommateur)

Cartographie des tests par étape (cf. `use-cases.md` pour le mapping UC → test).
Validation **locale** : `cargo test` + `cargo test --features databento`
(+ `TRADE_AGG_DATA_DIR` pour les tests réels gated) + `cargo clippy --all-targets`.

| Étape | Fichier de tests | Couverture |
|-------|------------------|------------|
| t5.1 (#17) | `tests/merged_t5.rs` | ingestion snapshot, synchro `book()`, désordre temporel, no-op sans passif (synthétique) |
| t5.1 (#17) | `tests/databento_replay.rs::replay_merged_*` | replay fusionné réel : ordre event-time, book peuplé, déterminisme (gated) |
| t5.2 (#18) | `tests/merged_t5.rs` | book au ts de clôture ; rétro-compat abonné `on_bar_close` seul |
| t5.3 (#19) | `src/orderflow.rs` (unit) | `TradeCount` (Unknown ignoré), `Vwap` (valeur, ∈[low,high], vide=None) |
| t5.3 (#19) | `tests/order_flow_wiring.rs` | activation via `LensKind`, attachement à la barre |
| t5.4 (#20) | `src/orderflow.rs` (unit) | `Footprint::window` : largeur, ancre, débordements, cellules manquantes |
| t5.5 (#22) | `tests/databento_replay.rs::replay_to_bars_*` | équivalence avec câblage `ChannelSink` manuel (gated) |
| t5.6 (#21) | `src/period.rs` (unit) | `RenkoBrickPeriod` : grille, saut multi-briques, borne d'excursion |

**Règle de couverture tenue** : rien de codé sans use-case écrit ; rien sans test qui le
couvre. Les tests réels (DataBento) sont **gated** (`TRADE_AGG_DATA_DIR`) et **skippés**
sans données — jamais d'échec, pas de données commitées.
