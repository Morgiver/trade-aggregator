# orderflow/tpo — TPO / Market Profile

> Feuille. Parent : [`README.md`](README.md).

**Rôle** : distribution du **temps passé à chaque niveau de prix** sur une `Bar` (lentille
**temps**, vs `volume-profile` = lentille volume). C'est le Market Profile de Steidlmayer.

## Accumulation
- Le temps de la `Bar` est découpé en **brackets** (sous-périodes, ex. lettres TPO).
- Pour chaque bracket, on marque les niveaux de prix **touchés** (par les `Trade` du
  bracket).
- `on_close` : POC (temps) = prix le plus visité ; ValueArea (temps) = ~70 % des TPO ;
  *single prints* = niveaux touchés par un seul bracket.

## Note de conception
- Découpage en brackets = paramètre (souvent lié à une sous-unité de temps).
- Partage l'axe « prix » avec footprint/volume-profile mais compte du **temps**, pas du
  volume → accumulateur distinct.

## Granularité
Comme les autres lentilles agressives : `Trade` suffisent. Indépendant du book.

---

## Fiches features (Phase 5)
- **`TPO-1`** — Découpage en brackets (sous-périodes) · **P2** · *le temps de la bar est segmenté.*
- **`TPO-2`** — Marquage des niveaux touchés par bracket · **P2** · *profil temps×prix construit.*
- **`TPO-3`** — `POC` / `ValueArea` (version temps) · **P2** · *POC/VA temps corrects.*
- **`TPO-4`** — `Single prints` · **P2** · *niveaux touchés par un seul bracket identifiés.*
- **`TPO-5`** — `Initial Balance` · **P2** · *IB (n premiers brackets) exposée.*
