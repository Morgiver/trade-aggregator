# Tests documentés — T1 Cœur agressif

> Phase 7, étape 2. Critères d'acceptation par use-case. **U** = unitaire, **I** = intégration.
> Rédigés lot par lot (A d'abord).

## Lot A — Order flow

- **`UC-T1-1`** (U) — `BarComponent` : un type implémentant `on_trade`/`on_close` se compile et s'utilise via le trait.
- **`UC-T1-3`** Footprint (U) — après 3 trades (Buy@100×2, Sell@100×5, Buy@101×1), les cellules valent `{100:(2,5), 101:(1,0)}`. Un côté `Unknown` n'est attribué ni Buy ni Sell.
- **`UC-T1-4`** VolumeProfile + POC (U) — volumes par prix corrects ; `poc()` = prix de volume max (ex. 100 si 100 a le plus de volume).
- **`UC-T1-5`** Value Area (U) — sur un profil connu, la value area (~70 %) couvre les niveaux attendus autour du POC ; le seuil est paramétrable.
- **`UC-T1-6`** Delta (U) — `Σ Buy − Σ Sell` (ex. Buy 3 + Sell 5 → −2) ; `Unknown` compte 0.
- **`UC-T1-7`** CVD (U) — cumul des deltas de barres successives (ex. −2 puis +5 → −2 puis 3).

## Lot B — Périodes *(à détailler au lot B)*
## Lot C — Extension *(à détailler au lot C)*
