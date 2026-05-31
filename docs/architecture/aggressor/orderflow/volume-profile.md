# orderflow/volume-profile — VolumeProfile → POC / ValueArea

> Feuille. Parent : [`README.md`](README.md).

**Rôle** : distribution du **volume total par niveau de prix** sur une `Bar`, d'où l'on
dérive le `POC` et la `ValueArea`.

## Accumulation
- `on_trade` : `profile[trade.price] += trade.size`.
- `on_close` :
  - **POC** = niveau de prix de volume maximal ;
  - **ValueArea (VAH/VAL)** = plus petite plage autour du POC concentrant **~70 %** du
    volume (seuil paramétrable).

## Relation au Footprint
Le `VolumeProfile` = la projection « volume par prix » du [`footprint`](footprint.md)
(sans la distinction Bid/Ask). À l'implémentation, dérivable du footprint pour éviter un
double comptage.

## Frontière
POC / ValueArea sont des **statistiques de la distribution** → agrégation (IN). Toute
lecture *décisionnelle* (ex. « le POC fait support ») = interprétation (OUT).

---

## Fiches features (Phase 5)
- **`VP-1`** — Accumulation `prix → volume` sur `on_trade` · **P1** · *distribution de volume par prix.*
- **`VP-2`** — `POC` (niveau de volume max) · **P1** · *POC correct à la clôture.*
- **`VP-3`** — `ValueArea` (~70 %, seuil paramétrable) · **P1** · *VAH/VAL corrects autour du POC.*
