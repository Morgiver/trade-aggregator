# Idée — trade-aggregator

> Phase 1 — Découverte. Document **libre** : on décrit, on ne décrète pas.
> Se découpera en fichiers / `recherche/` seulement si le volume le justifie.

## Pitch

Une **librairie (crate) Rust** capable d'**agréger des trades en temps réel** sous
différentes formes **périodiques**, où une « période » peut être définie par des
natures variées de données. Trois piliers fonctionnels au-delà de l'agrégation brute :
**order flow** complet, **TPO** (Time Price Opportunity), et un **support
programmatique d'indicateurs** branché sur les données agrégées.

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

### Order flow (dans les périodes, quel que soit leur type)
- **Footprint** : volume **bid vs ask par niveau de prix** dans la barre.
- Dérivés : *delta*, *cumulative delta (CVD)*, *POC*, *Value Area (VAH/VAL)*,
  imbalances diagonales.

### TPO / Market Profile
- Distribution du **temps** passé à chaque prix (lettres TPO), *Value Area 70 %*,
  *POC*, *single prints*, *Initial Balance*.
- Footprint = lentille **volume** ; TPO = lentille **temps** → complémentaires sur la
  même période.

### Layer d'indicateurs programmatique
- Brancher des méthodes qui consomment les données agrégées pour calculer des
  indicateurs techniques. Mécanisme : callbacks, events, channel — à décider.

## Recherche & inspirations

### Prior art direct (⚠️ socle déjà existant)
- **[`trade_aggregation`](https://docs.rs/trade_aggregation)** : fait DÉJÀ le socle —
  trait `AggregationRule` (`TimeRule`, `AlignedTimeRule`, `VolumeRule`, `TickRule`,
  `RelativePriceRule`/Renko), `ModularCandle` + `CandleComponent`, low-latency /
  incrémental. → repositionner trade-aggregator sur ce qu'il N'A PAS (order flow riche,
  TPO, imbalance bars, indicateurs). 3 options : s'appuyer dessus / s'en inspirer et
  réécrire un socle plus ambitieux / le dépasser comme référence.

### Layer indicateurs incrémental (réutiliser ou s'inspirer)
- **[`yata`](https://github.com/amv-dev/yata)** : TA library, support candles, trait
  pour créer ses indicateurs.
- **[`kand`](https://github.com/kand-ta/kand)** : O(1) incrémental, VWAP, Supertrend.
- **[`mantis-ta`](https://crates.io/crates/mantis-ta)** : O(1), **zéro allocation dans
  le hot path** — esprit low-latency.
- `ta`, `rsta` : indicateurs classiques.

### Concepts / références théoriques
- Imbalance & run bars : López de Prado, *Advances in Financial Machine Learning*
  (2018), ch. 2. Réf. d'implémentation Python : `mlfinlab` / `mlfinpy`.
- Footprint / delta / CVD / Market Profile (Steidlmayer) : vocabulaire à fixer en Phase
  Domaine.
- Inférence du côté agresseur sans flag : tick rule / quote rule (Lee-Ready).

## Non-goals (pressentis, à confirmer)

- _Pressenti :_ pas de connecteurs exchange / websocket — la crate consomme un **flux
  de trades normalisé** en entrée (à confirmer avec Morgan).

## Questions ouvertes

1. **Positionnement vs `trade_aggregation`** : dépendre / réécrire / dépasser ?
2. **Donnée d'entrée minimale** : `Trade { ts, price, size, side? }` ? `side` optionnel
   + inféreur (Lee-Ready) ? — conditionne toute la couche order flow.
3. **Frontière du scope** : connecteurs exchange dedans ou dehors ?
4. **Temps réel vs replay** : même API pour live + rejeu d'historique ?
5. **Multi-symboles** : une instance = un instrument, ou N flux gérés ?
6. **Indicateurs** : réutiliser yata/kand/mantis-ta, ou trait `Indicator` maison ?
   Branchement par callbacks / events / channel ?
