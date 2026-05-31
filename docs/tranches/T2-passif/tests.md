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

## Lot B — Profils de liquidité
- **`UC-T2-8…13`** (I) — `with_liquidity_profile(window)` produit des `LiquidityProfile` par fenêtre alignée : churn add/cancel (`LP-3`), profil pondéré-temps par côté (`LP-1`/`LP-4`), déséquilibre (`LP-5`), snapshots open/close (`LP-2`), drain pull (`EXT-6`), dernière fenêtre `partial` au `finish()` (`LP-6`).
## Lot C — Mapping DataBento book
- **`UC-T2-15`** (U/R) — `book_from_mbp10` reconstruit l'`OrderBook` depuis les 10 niveaux d'un message MBP-10.
- **`UC-T2-14`** — `book_update_from_mbo` mappe MBO → `BookUpdate` (A/C/M, côté ; None pour T/F/R/N). *Note : reconstruction L3→L2 fidèle (suivi order_id) = raffinement ultérieur.*
- **`UC-T2-17`** (R, gated) — replay réel d'un `*.mbp-10.dbn.zst` : book reconstruit non croisé (validé sur NQ).
