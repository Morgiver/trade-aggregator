# transverse/ — Préoccupations transverses

> Ce qui **traverse tous les nœuds** de l'archi. Parent : [`../README.md`](../README.md).
> Rattaché surtout aux piliers **P4** (event-time) et au transverse **performance**.

## Performance / low-latency
- **Hot path zero-alloc** : intégrer un `Trade` / un `BookUpdate` ne doit pas allouer.
  Buffers réutilisés, structures pré-dimensionnées.
- **Génériques monomorphisés** plutôt que `Box<dyn>` dans le hot path (les lentilles, les
  `Period`, le point d'extension). Le `dyn` est toléré aux **frontières** (config, sortie).
- Le `OrderBook` et les profils sont les points chauds → choix de structures (maps triées,
  arènes) à benchmarker (tranche **T4**).

## Temps (event-time)
- **Une seule horloge** : le timestamp des `MarketEvent`. **Aucune** horloge système dans
  le cœur → live = replay, déterministe.
- Type d'horodatage à fixer (ex. ns depuis epoch UTC) — Phase 7.
- **Ordre** : les events sont supposés ordonnés par temps ; le désordre est une **anomalie**
  (cf. erreurs).

## Erreurs
- **Fail-fast à la construction** : agrégation incompatible avec la `Granularity` déclarée
  → erreur typée (idéalement à la compilation).
- **Tolérance en flux** : trous de séquence / désordre / anomalies de book → stratégie de
  **resynchronisation** (re-snapshot), pas de panic dans le hot path.
- **Frontière de mapping** : les erreurs de décodage source (DataBento) sont des `Result`
  à la frontière, jamais dans le cœur.

## Observabilité (léger, *later*)
- Hooks de métriques **optionnels** (events traités, bars émises, resync) — sans coût quand
  désactivés. Non prioritaire (P3).
