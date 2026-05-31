# Architecture — trade-aggregator (racine)

> Niveau racine de la descente **C4** (Contexte → Composants → … → Code). Dérivée des
> [features](../vision/features.md), des [piliers](../vision/piliers.md) et du
> [domaine](../domain/concepts.md). Chaque nœud riche se re-décompose dans son
> sous-dossier ; un nœud simple est une feuille.

## Contexte (niveau 1)

Le système est **une librairie (crate)** : `trade-aggregator`. Deux externes l'entourent.

```mermaid
graph LR
    Src["Source de données<br/>(DataBento dbn · ou adapter externe)"] -->|MarketEvent canonique| TA
    TA["trade-aggregator<br/>(la crate)"] -->|Bars · Profiles<br/>BarClose / BarUpdate| Cons["Consommateur<br/>(indicateurs, viz, stratégies)"]
```

- **Source** : produit des `MarketEvent` au **format canonique** (le mapping DataBento
  vit *dans* la crate, isolé ; les adapters par venue sont *hors* scope).
- **Consommateur** : s'abonne au **point d'extension** et calcule ce qu'il veut (la crate
  n'interprète pas).

## Composants internes (niveau 2)

Dérivés des thèmes de features et des concepts du domaine.

```mermaid
graph TD
    In["canonical<br/>(modèle d'events + Instrument + Granularity + mapping DataBento)"]
    In --> SA["symbol-aggregator<br/>(routage + fan-out + process(event))"]
    SA --> AG["aggressor<br/>(Periods · Bars · order flow)"]
    SA --> PA["passive<br/>(OrderBook · LiquidityProfile)"]
    AG --> EX["extension<br/>(point d'extension réactif)"]
    PA --> EX
    EX --> Out["Consommateur"]
    Tr["transverse<br/>(perf · temps · erreurs)"] -.traverse.-> SA
```

| Composant | Rôle | Richesse |
|---|---|---|
| **canonical** | Contrat d'entrée : `MarketEvent` (Trade/BookUpdate), `Instrument`, `Granularity`, `AggressorSide` + **mapping DataBento** isolé. | feuille |
| **symbol-aggregator** | Orchestration : route les events vers les deux côtés, fan-out vers N `Period`, boucle `process(event)` (live = replay). | feuille |
| **aggressor** | `AggressorAggregator` : `Period` → `Bar` + order flow (`Footprint`, `Delta`/CVD, `POC`, `ValueArea`, `TPO`). | **riche → sous-dossier** |
| **passive** | `PassiveAggregator` : **reconstruction de l'`OrderBook`** + `LiquidityProfile` périodiques. | **riche → sous-dossier** |
| **extension** | Point d'extension réactif : push/pull, `BarClose` / `BarUpdate`, alignement des deux côtés. | feuille |
| **transverse/** | Préoccupations qui traversent tout : performance/zero-alloc, gestion du temps (event-time), erreurs. | dossier transverse |

## Arbre prévu (à valider)

```
docs/architecture/
├── README.md             ← (ici) contexte + composants
├── canonical.md          ← feuille
├── symbol-aggregator.md  ← feuille
├── aggressor/            ← nœud riche (Periods · Bars · order flow)
│   └── README.md
├── passive/              ← nœud riche (reconstruction book · profils)
│   └── README.md
├── extension.md          ← feuille
└── transverse/
    └── README.md         ← perf · temps · erreurs
```

> Découpe **riche vs feuille** = proposition. On descend dans `aggressor/` et `passive/`
> (les deux plus riches) ; le reste reste en feuille tant que le besoin ne l'exige pas
> (YAGNI).
