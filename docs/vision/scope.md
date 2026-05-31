# Scope — trade-aggregator

> Garde-fou anti-creep. La ligne directrice tient en une phrase :
> **agréger & structurer la donnée de marché brute (IN) vs l'interpréter (OUT).**
>
> Sources et raisonnement : voir [`../ideation/idea.md`](../ideation/idea.md) et
> l'issue [#1 — Découverte](https://github.com/Morgiver/trade-aggregator/issues/1).

## La frontière en une ligne

`trade-aggregator` transforme un flux de **données de marché brutes** (L3 / MBO :
tape + book) en **données agrégées structurées**, alignées temporellement et exposées
en temps réel. Il **n'interprète pas** ces données.

---

## IN — ce qu'on construit

### A. Cœur d'agrégation
- **`SymbolAggregator`** : racine par symbole, porte l'**instrument definition** (tick
  size, price increment, lot/contract size, multiplicateur, devise) et route un flux
  d'événements horodatés unique vers les deux côtés.
- **`AggressorAggregator`** (tape) : N agrégations **périodiques en parallèle**.
- **`PassiveAggregator`** (book) : **reconstruction maison du carnet** (sur la base du
  guide DataBento) + agrégation périodique de l'état du book.

### B. Types de périodes (agressif)
- Temporelles (timeframes, aligned, sessions), activité (tick / volume / dollar bars),
  prix (range, Renko, P&F), information-driven (imbalance / run bars), hybrides.

### C. Lentilles order flow & profils
- **Footprint** (volume bid/ask par niveau de prix), delta, CVD, POC, Value Area.
- **TPO / Market Profile** (lentille temps).
- **Profils de liquidité passifs** : profil pondéré-temps, snapshots, churn add/cancel,
  depth, déséquilibre moyen.

### D. Entrée & exécution
- **Un seul format d'entrée canonique** (maison) : la crate est **source-agnostic**, elle
  ne connaît qu'un modèle d'événements normalisé. *(décision Morgan : un seul format
  dans le scope ; les adapters par venue sont hors scope — cf. OUT.)*
- **Mapping DataBento** (via la crate officielle `dbn`, MBO/L3, side agresseur fourni)
  fourni comme module **isolé / feature-gated** : DataBento est un *format* (pas un
  connecteur réseau) et notre source de référence + golden dataset de test.
- **Granularité déclarée à la création** de l'agrégateur (L3/MBO, L2/MBP, L1/BBO) ; le
  computing **s'adapte** aux capacités disponibles. Demander une agrégation incompatible
  avec la granularité déclarée → **erreur à la construction** (fail-fast). *(décision
  Morgan.)*
- **Live + replay** sur la même API : `process(event)` sur des événements horodatés ;
  le temps vient des **données** (event-time) → déterministe, testable.

### E. Exposition
- **Point d'extension réactif** : pour que n'importe qui branche SES calculs temps réel
  sur les données agrégées (push/pull · `on_bar_close` / `on_bar_update`), les deux
  côtés **alignés temporellement**.

---

## OUT — hors scope (et pourquoi)

| Hors scope | Raison |
|---|---|
| **Layer d'indicateurs** (calcul d'indicateurs techniques) | = interprétation → **autre projet**. |
| **Métriques cross-aggregator** : absorption, détection icebergs/refills | = interprétation. La crate **aligne** les deux côtés ; le consommateur calcule. |
| **Connecteurs / adapters par venue** (Binance, Bybit, Coinbase…) + réseau / websocket / REST | un seul format canonique IN ; chaque venue = un adapter dédié → relève d'un **scope « connecteurs »** distinct (projet compagnon éventuel). |
| **Réutilisation / inspiration** de `trade_aggregation` ou des crates TA (yata/kand/mantis-ta) | décision : on fait **maison**. |
| **Stockage / persistance** de la donnée brute ou agrégée | non requis ; à la charge du consommateur. |
| **Backtesting / exécution d'ordres / stratégies** | clairement un autre étage. |

---

## Arbitrages tranchés (31/05/2026)

- **Normaliseurs multi-sources** → ✅ **un seul format canonique** dans le scope ; les
  adapters par venue sont **OUT** (scope « connecteurs »). Mapping DataBento fourni mais
  isolé (format de référence + test).
- **Granularité d'entrée** → ✅ **déclarée à la création** de l'agrégateur ; le computing
  s'adapte aux capacités ; agrégation incompatible = **erreur fail-fast**.
