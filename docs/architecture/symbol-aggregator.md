# symbol-aggregator — SymbolAggregator

> Feuille. Parent : [`README.md`](README.md). Concept :
> [`../domain/glossaire.md`](../domain/glossaire.md).
>
> **Rôle** : racine d'exécution **par symbole**. Porte l'`Instrument`, **lie** les deux
> agrégateurs et **route** les `MarketEvent`. Pilier **P1** (dualité) + **P4** (event-time).

## Responsabilités
1. **`process(event)`** — point d'entrée unique (live **et** replay).
2. **Routage** : `Trade` → [`aggressor/`](aggressor/README.md) ; `BookUpdate` →
   [`passive/`](passive/README.md).
3. **Fan-out** : pousse chaque event vers **toutes les `Period`** configurées des deux côtés
   (multi-charts).
4. **Cohérence temporelle** : garantit que les deux côtés partagent les mêmes bornes de
   `Period` (alignement, pilier P5).

## Configuration (à la création)
- l'`Instrument` et la `Granularity` ;
- la liste des `Period` × lentilles voulues (agressif) et des profils (passif) ;
- **fail-fast** si une agrégation est incompatible avec la `Granularity` (cf.
  [`transverse/`](transverse/README.md)).

## Multi-symboles
Une instance = **un** symbole. Le multi-symboles se fait **par composition** au-dessus
(un `SymbolAggregator` par symbole), pas dans ce nœud.

---

## Fiches features (Phase 5)

> Atomisation du thème B ([`../vision/features.md`](../vision/features.md)).

- **`SYM-1`** — Point d'entrée `process(event)` · **P0** · *un MarketEvent traité fait avancer l'état.*
- **`SYM-2`** — Routage `Trade` → aggressor · **P0** · *un trade n'atteint que le côté agressif.*
- **`SYM-3`** — Routage `BookUpdate` → passive · **P0** · *un book update n'atteint que le côté passif.*
- **`SYM-4`** — Fan-out vers N `Period` configurées · **P0** · *un event alimente toutes les Period actives.*
- **`SYM-5`** — Config : `Instrument` + `Granularity` · **P0** · *instance créée avec son instrument et sa granularité.*
- **`SYM-6`** — Config : enregistrer `Period` × lentilles (agressif) · **P0** · *on choisit les agrégations agressives.*
- **`SYM-7`** — Config : enregistrer profils (passif) · **P1** · *on choisit les profils passifs.*
- **`SYM-8`** — Fail-fast granularité incompatible · **P0** · *agrégation non supportée → erreur à la construction.*
- **`SYM-9`** — Garantie d'alignement des bornes de `Period` (deux côtés) · **P1** · *aggressor et passive ferment sur les mêmes bornes.*
- **`SYM-10`** — Composition multi-symboles (1 instance/symbole) · **P1** · *N symboles = N instances orchestrées au-dessus.*
