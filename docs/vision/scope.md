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
- **Adapter DataBento** via la crate officielle `dbn` (MBO/L3, side agresseur fourni).
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
| **Connecteurs exchange / websocket / REST** (réseau, auth, reconnection) | la crate consomme un flux normalisé, elle ne se connecte pas. |
| **Réutilisation / inspiration** de `trade_aggregation` ou des crates TA (yata/kand/mantis-ta) | décision : on fait **maison**. |
| **Stockage / persistance** de la donnée brute ou agrégée | non requis ; à la charge du consommateur. |
| **Backtesting / exécution d'ordres / stratégies** | clairement un autre étage. |

---

## Zone grise — arbitrages restants (à trancher en Vision)

- **Normaliseurs multi-sources** : au-delà de DataBento, fournit-on des adapters de
  format pour Binance / Bybit / Coinbase, ou seulement un **trait d'entrée** que
  l'utilisateur mappe ? (le réseau reste OUT dans tous les cas)
- **Granularité minimale d'entrée** garantie : exige-t-on du L3/MBO, ou accepte-t-on de
  dégrader proprement avec du L2/MBP (book sans identité d'ordre) ?
