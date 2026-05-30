# Idée — trade-aggregator

> Phase 1 — Découverte. Document **libre** : on décrit, on ne décrète pas.
> Se découpera en fichiers / `recherche/` seulement si le volume le justifie.

## Pitch

Une **librairie (crate) Rust** capable d'**agréger des trades en temps réel** sous
différentes formes **périodiques**, où une « période » peut être définie par des
natures variées de données. Deux piliers fonctionnels + un pilier d'extensibilité :
**order flow** complet, **TPO** (Time Price Opportunity), et un **point d'extension
réactif** permettant à n'importe qui de brancher SES calculs temps réel sur les données
agrégées (le layer d'indicateurs lui-même = un autre projet).

## Features en vrac

### Types d'agrégation périodique (à lister exhaustivement — 1ʳᵉ tâche)
- **Temporelle** : TimeFrame (s, min, h, jour, mois, année) ; *aligned* (borné sur
  l'horloge) ; sessions / RTH.
- **Volumétrique** : volume échangé ; *dollar / notional / turnover bars* (n unités de
  **valeur**, pas de quantité).
- **Activité** : *tick bars* (n trades).
- **Limites / prix** : range fixe de ticks/points/écart de prix ; *Renko* ; *Point &
  Figure*.
- **Information-driven** (López de Prado, *AFML* ch. 2) : *imbalance bars*
  (tick/volume/dollar), *run bars*. → aucune impl Rust trouvée = carte de
  différenciation.
- **Hybrides** : barres composites (ex. temps **ou** volume, le premier atteint).
- **(à explorer)** périodes définies par des **événements d'orderbook** (cf. question
  ouverte sur le book).

### Order flow (dans les périodes, quel que soit leur type)
- **Footprint** : volume **bid vs ask par niveau de prix** dans la barre.
- Dérivés : *delta*, *cumulative delta (CVD)*, *POC*, *Value Area (VAH/VAL)*,
  imbalances diagonales.
- **Avec orderbook** (selon scope retenu) : absorption, détection icebergs/refills,
  liquidité au POC, book imbalance, profondeur au moment des trades.

### TPO / Market Profile
- Distribution du **temps** passé à chaque prix (lettres TPO), *Value Area 70 %*,
  *POC*, *single prints*, *Initial Balance*.
- Footprint = lentille **volume** ; TPO = lentille **temps** → complémentaires sur la
  même période.

### Point d'extension réactif (ex-« layer d'indicateurs », reformulé)
- Fournir tout ce qu'il faut pour brancher des calculs temps réel **externes** sur les
  données agrégées. La crate expose ; elle ne calcule pas d'indicateurs.
- **Axe 1 — push/pull** : callbacks (`FnMut`), trait observer (`OnBar`/`Subscriber`),
  channels (`mpsc`/`crossbeam`/`tokio::broadcast`) ; `Iterator`/`Stream` ; état
  interrogeable (snapshot pull).
- **Axe 2 — granularité** : `on_bar_close` ET `on_bar_update` (barre en formation).
- **Axe perf** : cœur générique monomorphisé (zero-cost) vs `Box<dyn>`. Reco : un point
  d'extension unique (trait `Sink`/`Observer`) + adaptateurs (channel, `Stream`).
- Autres : fan-out multi-subscribers ; mode *tee* (live + enregistrement).

## Entrée de données

- **Sources** : **DataBento** (crate Rust officielle [`dbn`](https://docs.rs/dbn/) —
  schémas **MBO** = orderbook complet, **Trades** = tape avec **aggressor side**
  Ask/Bid/None, MBP-1/10, TBBO) **ou** exchanges crypto fournissant orderbook + tape
  complets (Bybit, Binance `aggTrade.m`, Coinbase).
- **Côté agresseur** : fourni par les sources → inférence (Lee-Ready) reléguée à un
  *fallback* pour les cas `None`.
- **Live + replay** : même API. Pattern = `process(event)` sur des **événements
  horodatés** ; le temps vient des données (event-time) → déterministe, testable. Le
  dataset DataBento de Morgan = golden dataset des tests.

## Non-goals

- **Pas** de couche layer d'indicateurs (= autre projet).
- **Pas** de connecteurs exchange / websocket / REST (réseau, auth, reconnection).
- **Pas** de réutilisation ni d'inspiration de `trade_aggregation` ni des crates TA
  existantes (yata/kand/mantis-ta) — on fait à notre manière.

## Questions ouvertes

1. **Rôle de l'orderbook complet** : (a) enrichir l'order flow des barres de trades,
   (b) objet de premier plan qu'on agrège/profile aussi (voire familles d'agrégation
   basées book), ou les deux ? → **question structurante**.
2. **Normaliseurs vs trait d'entrée** : la crate fournit-elle des adapters de format
   (DataBento via `dbn`, schémas Binance/Bybit/Coinbase) vers un modèle interne unifié,
   ou juste un trait d'entrée que l'utilisateur mappe ? (réseau exclu dans tous les cas)
3. **Mono vs multi-symboles** : reco = brique `Aggregator` mono-instrument composable en
   multi (un par symbole) plutôt qu'un cœur multi monolithique. À valider.
4. **Mécanisme d'exposition** : point d'extension unique + adaptateurs — à confirmer.
