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
