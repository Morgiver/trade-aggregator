# Positionnement — trade-aggregator

> Différenciation vs l'existant + posture. Sources :
> [`../ideation/idea.md`](../ideation/idea.md).

## Face à l'existant

| Acteur | Ce qu'il fait | Notre différence |
|---|---|---|
| **`trade_aggregation`** (Rust) | Candles modulaires (composants scalaires) sur règles d'agrégation (time/volume/tick/Renko). | Order flow **riche** (footprint par niveau de prix, pas des scalaires), côté **passif** (book), TPO, imbalance bars, dualité Aggressor/Passive. On ne le réutilise pas. |
| **Crates TA** (`yata`, `kand`, `mantis-ta`, `ta`) | Calculent des **indicateurs** sur OHLCV. | On produit la **donnée agrégée riche en amont** et on n'impose aucun indicateur. **Complémentaires** : on pourrait brancher `yata` sur notre point d'extension. |
| **`mlfinlab` / `mlfinpy`** (Python) | Imbalance bars & recherche ML, surtout **batch/offline**. | **Rust**, **temps réel + replay**, + order flow & book, low-latency. |
| **Bookmap, Sierra Chart, ATAS, Quantower** | Plateformes order-flow **propriétaires**, GUI, end-user. | **Librairie ouverte et programmable** pour devs ; pas de rendu, pas de lock-in. |

**Trou de marché visé** : il n'existe pas (à notre connaissance) de brique **Rust**,
open, qui unifie **order flow agressif + profils de liquidité passifs** sous un modèle
temps-réel déterministe et programmable.

## Posture

- **Librairie, pas plateforme** : faire *une* chose (agréger) bien, l'exposer, ne pas
  l'interpréter.
- **Source-agnostic** : un format canonique unique ; aucun lock-in à une venue.
- **Maison & maîtrisé** : zéro dépendance aux crates d'agrégation/TA existantes.
- **Déterministe** : event-time, *live = replay* → reproductible et testable.
- **Low-latency / zero-cost** : le hot path prime.
- **Composable (Unix-y)** : un point d'extension propre plutôt qu'un monolithe ; on est
  une brique dans la chaîne d'autres outils.
