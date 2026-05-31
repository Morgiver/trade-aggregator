# Tests documentés — T3 Différenciation

> Phase 7, étape 2. **U** = unitaire.

## Lot A — Barres information-driven
- **`UC-T3-1`** (U) — TickImbalance : ferme quand `|Σ signe| ≥ seuil`.
- **`UC-T3-2`** (U) — VolumeImbalance : `Σ signe·size`.
- **`UC-T3-3`** (U) — DollarImbalance : `Σ signe·price·size`.
- **`UC-T3-4`** (U) — Run : ferme quand une série directionnelle atteint `seuil` trades.
- Protocole decide-before-add ; `Unknown` compte 0. Seuils fixes (dynamiques = T4+).

## Lot B — TPO / Market Profile *(à détailler)*
## Lot C — Finitions *(à détailler)*
