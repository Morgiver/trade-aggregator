# Tests documentés — T2 Côté passif

> Phase 7, étape 2. Critères d'acceptation par use-case. **U** = unitaire, **I** = intégration.

## Lot A — BookUpdate + reconstruction OrderBook
- **`UC-T2-1`** (compilation) — `BookUpdate` + `MarketEvent::BookUpdate` ; `ts()` couvre les deux variantes.
- **`UC-T2-2`** (U) — Add/Cancel/Modify maintiennent les niveaux L2 ; Cancel total retire le niveau.
- **`UC-T2-3`** (U) — best_bid (max Bid), best_ask (min Ask), depth(n).
- **`UC-T2-4`** (U) — `clear()` réinitialise (resync).
- **`UC-T2-5`** (U) — Cancel sous zéro → `Err(NegativeLevel)`, niveau borné à 0 ; book croisé détecté (`is_crossed`).
- **`UC-T2-6`** (I) — un `MarketEvent::BookUpdate` est routé vers le passif ; `agg.book()` reflète l'état.
- **`UC-T2-7`** (I) — `with_passive()` en L1 → `Err(IncompatibleGranularity{required:L2})`.

## Lot B — Profils de liquidité *(à détailler)*
## Lot C — Mapping DataBento book *(à détailler)*
