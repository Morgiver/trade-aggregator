# Tests documentés — T3 Différenciation

> Phase 7, étape 2. **U** = unitaire.

## Lot A — Barres information-driven
- **`UC-T3-1`** (U) — TickImbalance : ferme quand `|Σ signe| ≥ seuil`.
- **`UC-T3-2`** (U) — VolumeImbalance : `Σ signe·size`.
- **`UC-T3-3`** (U) — DollarImbalance : `Σ signe·price·size`.
- **`UC-T3-4`** (U) — Run : ferme quand une série directionnelle atteint `seuil` trades.
- Protocole decide-before-add ; `Unknown` compte 0. Seuils fixes (dynamiques = T4+).

## Lot B — TPO / Market Profile
- **`UC-T3-5..8`** (U + I) — `Tpo` (lentille, `LensKind::Tpo{bracket_ns, ib_brackets}`) : brackets, profil temps×prix, POC temps, Value Area ~70 %, single prints, Initial Balance. Validé sur scénario déterministe + intégration via l'aggregator (attaché à `Bar.orderflow.tpo`).
## Lot C — Finitions
- **`UC-T3-9`** (U) — `PointFigurePeriod` : renversement de colonne au-delà de `reversal×box_size`.
- **`UC-T3-10`** (I) — détection de désordre temporel (`TR-5`) : un event au `ts` plus ancien incrémente `out_of_order_count()`, sans rejet ni panic.
