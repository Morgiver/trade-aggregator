# Tests documentés — T4 Robustesse & performance

> Phase 7, étape 2. **U** = unitaire, **B** = benchmark (ignoré par défaut).

## Lot A — L3→L2 fidèle
- **`UC-T4-1/2`** (U) — `MboBook` : Add/Cancel/Modify par `order_id` ; L2 dérivé = somme des ordres par niveau ; `Modify` (taille ou prix) ne corrompt pas le niveau.

## Lot B — Bornes mémoire & durcissement
- **`UC-T4-3`** (U) — `OrderBook::prune_to_depth(n)` ne garde que les `n` meilleurs niveaux par côté (`TR-10`).
- **`UC-T4-4`** — `clear()` / intégrité déjà couverts (T2) ; resync = re-snapshot.

## Lot C — Perf
- **`UC-T4-6`** (B) — benchmark débit d'agrégation (trades/s) sur flux synthétique.
- **`UC-T4-5`** (décision) — dispatch : le hot path par trade évite l'allocation (`TR-1`) ; les lentilles sont en dispatch statique (enum, pas `dyn`). La monomorphisation totale (génériser `Period`/`Subscriber`) est un **non-goal assumé** : la souplesse multi-périodes/multi-abonnés prime, et le coût `dyn` (1 appel virtuel par close/update, pas par trade pour la sortie) est négligeable — confirmé par le benchmark.
