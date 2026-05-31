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

---

## Fiches features (Phase 5)

> Atomisation du thème G + préoccupations transverses.

- **`TR-1`** — Hot path zero-alloc · **P1** · *intégrer un event n'alloue pas.*
- **`TR-2`** — Génériques monomorphisés (pas de `dyn` dans le hot path) · **P1** · *dispatch statique des lentilles/Period/extension.*
- **`TR-3`** — Horloge unique = event-time · **P0** · *aucune horloge système dans le cœur.*
- **`TR-4`** — Type d'horodatage (ns epoch UTC) · **P0** · *timestamp commun à tous les events.*
- **`TR-5`** — Hypothèse d'ordre + détection désordre · **P2** · *event hors ordre signalé comme anomalie.*
- **`TR-6`** — Erreurs fail-fast à la construction · **P0** · *config invalide → erreur typée.*
- **`TR-7`** — Tolérance/resync en flux · **P2** · *anomalie de book → resync, pas de panic.*
- **`TR-8`** — Erreurs de mapping aux frontières (`Result`) · **P1** · *décodage source faillible hors du cœur.*
- **`TR-9`** — Hooks de métriques optionnels · **P3** · *observabilité sans coût quand désactivée.*
- **`TR-10`** — Bornes mémoire (profondeur du book, plage de prix des profils) · **P2** · *structures bornées ; pas de croissance mémoire non maîtrisée en flux long.* *(passe de relecture)*
