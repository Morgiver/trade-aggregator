# Use-cases — T4 Robustesse & performance

> Phase 7, étape 1. Dernière tranche. S'appuie sur T0→T3. Sous-découpé en 3 lots.

## Lot A — Reconstruction L3→L2 fidèle

### `UC-T4-1` — `MboBook` (suivi par ordre)
Map `order_id → (side, price, size)`. `Add` insère, `Cancel` retire, `Modify` ajuste
(prix et/ou taille). Le book L2 agrégé reste cohérent (somme des ordres par niveau).

### `UC-T4-2` — Dérivation L2 correcte
Le L2 dérivé du `MboBook` égale la somme des ordres par niveau ; un `Modify` multi-ordres
ne corrompt pas le niveau.

## Lot B — Bornes mémoire & durcissement

### `UC-T4-3` — Profondeur de book bornée (`TR-10`)
Option de limiter le book aux `N` meilleurs niveaux par côté (les niveaux au-delà sont élagués).

### `UC-T4-4` — Resync / cas limites (`TR-7`)
Snapshot/clear remet le book d'aplomb ; séquences incohérentes tolérées sans panic.

## Lot C — Point d'extension générique & benchmarks

### `UC-T4-5` — Dispatch monomorphisé (`EXT-3`/`TR-2`)
Un point d'extension **générique** (sans `Box<dyn>` dans le hot path), rétro-compatible
avec le `Subscriber` dynamique.

### `UC-T4-6` — Benchmarks du hot path
Mesure du débit d'agrégation (trades/s) sur un flux synthétique ; sert de garde-fou perf.

---

## Couverture
`EXT-3` · `TR-2` · `TR-10` · `TR-7` · L3→L2 fidèle — fiches T4 + reports T1/T2.

## Note
Après T4 : **roadmap épuisée**, crate `0.1` fonctionnelle de bout en bout.
