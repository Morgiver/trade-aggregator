# T6 — Tests (primitif screenshot)

Cartographie des tests par étape (cf. `use-cases.md` pour le mapping UC → test).
Validation **locale** : `cargo test` (+ `--features databento`) + `cargo clippy
--all-targets`.

| Étape | Fichier de tests | Couverture |
|-------|------------------|------------|
| t6.1 (#31) | `tests/screenshot_t6.rs` | `forming_orderflow` (trades depuis l'ouverture, None cases), lecture-seule + cohérence avec la clôture (CVD sans double comptage), `forming_bar`, multi-frames |
| t6.2 (#32) | `tests/screenshot_t6.rs` | historique opt-in, FIFO borné à `depth`, `snapshot()` multi-frame (`[≤X fermées]+[forming]`), cohérence avec les barres notifiées aux abonnés |

**Règle de couverture tenue** : rien de codé sans use-case écrit ; rien sans test qui le
couvre. Tous les tests T6 sont **synthétiques** (pas de dépendance aux données réelles) :
les primitifs d'interrogation se valident entièrement au niveau `SymbolAggregator`.
