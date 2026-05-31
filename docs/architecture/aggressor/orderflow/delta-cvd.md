# orderflow/delta-cvd — Delta / Cumulative Delta

> Feuille. Parent : [`README.md`](README.md).

**Rôle** : mesurer le **déséquilibre agressif** d'une `Bar` (`Delta`) et son **cumul** à
travers les `Bar` (`CVD`).

## Delta (par `Bar`)
- `on_trade` : `delta += signed(trade)` où Buy = `+size`, Sell = `−size`.
- Dérivable du [`footprint`](footprint.md) : `Σ volume_buy − Σ volume_sell`.

## CVD (Cumulative Delta) — état **inter-Bar**
- `CVD_n = CVD_{n−1} + delta_n`.
- ⚠️ **N'est pas porté par la `Bar`** mais par l'**agrégateur** (état courant qui survit
  d'une Bar à l'autre). C'est la seule lentille à mémoire trans-Bar.

## Frontière
On fournit `delta` et `cvd` (chiffres). Les *divergences CVD/prix*, signaux, etc. =
interprétation → consommateur.

---

## Fiches features (Phase 5)
- **`DC-1`** — `Delta` par `Bar` (Σ buy − Σ sell) · **P1** · *delta correct, dérivable du footprint.*
- **`DC-2`** — `CVD` (cumul inter-Bar, état agrégateur) · **P1** · *cumul correct, persistant d'une bar à l'autre.*
- **`DC-3`** — Min/Max delta intra-bar · **P2** · *extrêmes de delta dans la bar exposés.*
