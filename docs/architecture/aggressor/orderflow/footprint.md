# orderflow/footprint — Footprint

> Feuille. Parent : [`README.md`](README.md). Concept :
> [`../../../domain/glossaire.md`](../../../domain/glossaire.md).

**Rôle** : pour une `Bar`, répartir le **volume échangé par niveau de prix et par côté**
(`Bid` vs `Ask`). Lentille **volume**.

## Accumulation
- `on_trade` : `cells[trade.price][trade.aggressor_side] += trade.size`
  (Buy → colonne Ask consommée / Sell → colonne Bid consommée, selon convention à fixer
  en Phase 7).
- Structure : `prix → (volume_buy, volume_sell)`.

## Sorties
- La grille footprint de la `Bar` (volume Buy/Sell par prix).
- Base de calcul du `Delta` (cf. [`delta-cvd.md`](delta-cvd.md)) et des imbalances
  diagonales (donnée brute — l'interprétation reste au consommateur).

## Granularité
N'a besoin que des `Trade` (+ `AggressorSide`). Indépendant du book.

---

## Fiches features (Phase 5)
- **`FP-1`** — Accumulation `prix → (vol_buy, vol_sell)` sur `on_trade` · **P1** · *chaque trade incrémente la bonne cellule.*
- **`FP-2`** — Grille footprint exposée à la clôture · **P1** · *la bar porte sa grille volume×prix×côté.*
- **`FP-3`** — Imbalances diagonales (donnée brute) · **P2** · *ratios bid/ask diagonaux disponibles (calcul d'interprétation laissé au consommateur).*
