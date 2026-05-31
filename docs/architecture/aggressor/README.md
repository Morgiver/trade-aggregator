# aggressor/ — AggressorAggregator

> Nœud **riche** de l'archi. Parent : [`../README.md`](../README.md). Concepts :
> [`../../domain/glossaire.md`](../../domain/glossaire.md).
>
> **Rôle** : agréger le flux **agressif** (les `Trade`) en `Bar`, selon une ou plusieurs
> `Period` en parallèle, et calculer l'**order flow** de chaque `Bar`.

## Vue d'ensemble

```mermaid
graph TD
    T["Trade (porte AggressorSide)"] --> P["Period(s)<br/>(règles de fermeture)"]
    P -->|ouvre / ferme| B["Bar (en formation)"]
    T --> B
    B -->|alimente| FP["Footprint (volume×prix×côté)"]
    B --> VP["VolumeProfile → POC / ValueArea"]
    B --> TPO["TPO (temps×prix)"]
    B -.->|delta de la Bar| D["Delta"]
    D --> CVD["CVD (état inter-Bar)"]
    B -->|BarUpdate / BarClose| EX["→ extension"]
```

## Sous-composants

### `Period` — la règle de fermeture
Décide **quand** la `Bar` courante se ferme. Famille homogène de règles (un même rôle,
plusieurs variantes) : Time, Tick, Volume, Dollar/Notional, Range, Renko, Imbalance,
Run, hybride. **Plusieurs `Period` tournent en parallèle** sur le même flux (multi-charts).
- *État* : chaque `Period` maintient son propre seuil/compteur (temps écoulé, volume
  cumulé, range courant, déséquilibre cumulé pour les imbalance bars…).
- *Forme esquissée* : un trait `Period` du type `on_trade(&mut self, &Trade) -> Boundary`
  (continue / ferme-et-rouvre), + `reset`.

### `Bar` — l'accumulateur
Unité en **formation** puis **fermée**. Accumule l'`OHLCV` et **délègue** aux lentilles
order flow. Émet un `BarUpdate` à chaque `Trade` intégré, un `BarClose` à la fermeture.

### Lentilles order flow (attachées à la `Bar`)
- **Footprint** : `prix → (volume Bid, volume Ask)`, alimenté par chaque `Trade` selon son
  `AggressorSide`. Lentille **volume**.
- **VolumeProfile → POC / ValueArea** : distribution du volume par prix → `POC` (max),
  `ValueArea` (~70 %). *(Footprint et VolumeProfile partagent « volume par prix ».)*
- **TPO** : distribution du **temps** par prix → POC/VA en version temps.
- **Delta / CVD** : `Delta` = volume agressif Buy − Sell de la `Bar` ; **`CVD`** = somme
  courante des deltas → **état porté par l'agrégateur** (inter-`Bar`), pas par la `Bar`.

## Composabilité

Les lentilles sont **optionnelles et composables** : à la création d'une `Period`, on
choisit les lentilles voulues (footprint seul, ou footprint + TPO, etc.). Forme esquissée :
un trait commun `BarComponent` (`on_trade`, `on_close`).
> ⚠️ Ressemble *en surface* à l'idée modulaire de `trade_aggregation`, mais : (a) on fait
> **maison**, (b) nos lentilles sont des **profils par niveau de prix**, pas des composants
> scalaires.

## Capacités / granularité

Le flux agressif (les `Trade` avec `AggressorSide`) suffit à **tout** l'order flow ici —
ce nœud n'a **pas** besoin du book. La granularité L1/L2/L3 n'impacte donc l'aggressor
que via la disponibilité de l'`AggressorSide` (fallback si `None`, cf. transverse).

## Descente

Les lentilles order flow sont riches → re-décomposées dans **[`orderflow/`](orderflow/)** :

| Sous-nœud | Fichier |
|---|---|
| Vue d'ensemble + trait `BarComponent` | [`orderflow/README.md`](orderflow/README.md) |
| Footprint | [`orderflow/footprint.md`](orderflow/footprint.md) |
| VolumeProfile → POC / ValueArea | [`orderflow/volume-profile.md`](orderflow/volume-profile.md) |
| TPO | [`orderflow/tpo.md`](orderflow/tpo.md) |
| Delta / CVD | [`orderflow/delta-cvd.md`](orderflow/delta-cvd.md) |

`Period` et `Bar` restent décrits ici (feuilles de ce nœud). Le détail des types vient en
**Phase 7**.

---

## Fiches features (Phase 5)

> Atomisation du thème C ([`../../vision/features.md`](../../vision/features.md)). Les
> lentilles order flow (thème D) sont fichées dans [`orderflow/`](orderflow/README.md).

### `Period` — types (la plus petite unité = un type de Period)
- **`AGG-P0`** — Trait `Period` (`on_trade → Boundary` : Continue | CloseAndOpen) · **P0** · *contrat commun de fermeture.*
- **`AGG-P1`** — `TimePeriod` (n unités de temps) · **P0** · *bar close au franchissement de durée.*
- **`AGG-P2`** — `AlignedTimePeriod` (bornée sur l'horloge) · **P1** · *bornes alignées sur minute/heure.*
- **`AGG-P3`** — `SessionPeriod` (RTH / sessions) · **P2** · *bornes = ouverture/clôture de session.*
- **`AGG-P4`** — `TickPeriod` (n trades) · **P1** · *close tous les n ticks.*
- **`AGG-P5`** — `VolumePeriod` (n volume) · **P1** · *close à n volume cumulé.*
- **`AGG-P6`** — `DollarPeriod` (n notional) · **P1** · *close à n valeur échangée.*
- **`AGG-P7`** — `RangePeriod` (range de prix fixe) · **P1** · *close au franchissement de range.*
- **`AGG-P8`** — `RenkoPeriod` · **P1** · *brique au mouvement de prix.*
- **`AGG-P9`** — `PointFigurePeriod` · **P2** · *colonnes X/O.*
- **`AGG-P10`** — `TickImbalancePeriod` · **P2** · *close au seuil de déséquilibre de ticks.*
- **`AGG-P11`** — `VolumeImbalancePeriod` · **P2** · *idem, volume.*
- **`AGG-P12`** — `DollarImbalancePeriod` · **P2** · *idem, notional.*
- **`AGG-P13`** — `RunPeriod` (tick/volume/dollar runs) · **P2** · *close sur séquence directionnelle.*
- **`AGG-P14`** — `HybridPeriod` (composite, premier atteint) · **P2** · *close au 1ᵉʳ critère franchi.*

### `Bar` — l'accumulateur
- **`AGG-B1`** — État `Bar` (forming → closed) · **P0** · *cycle de vie d'une bar.*
- **`AGG-B2`** — Calcul `OHLCV` · **P0** · *open/high/low/close/volume corrects.*
- **`AGG-B3`** — Émission `BarUpdate` (intra-bar) · **P1** · *un event en cours notifie la bar en formation.*
- **`AGG-B4`** — Émission `BarClose` · **P0** · *la fermeture notifie la bar finalisée.*
