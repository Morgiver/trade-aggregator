# passive/liquidity-profile — Profil de liquidité périodique

> Feuille (proche du code). Parent : [`README.md`](README.md).
>
> **Rôle** : **agréger** l'état de l'`OrderBook` maintenu sur une `Period` en un
> `LiquidityProfile`. *(Maintenir = [`orderbook.md`](orderbook.md) ; ici on résume sur une
> fenêtre.)*

## Sorties produites (sur une `Period`)

- **Profil de liquidité pondéré-temps** : pour chaque niveau de prix, la quantité
  **moyenne pondérée par le temps de présence** sur la fenêtre.
- **Snapshots** d'ouverture / clôture du book.
- **Churn** : volumes d'`Add` / `Cancel` sur la période (activité du book).
- **Depth** : profondeur max/min, quantité cumulée par côté.
- **Déséquilibre** bid/ask moyen.

## Mécanique

- Bornée par la **`Period`** (mêmes bornes que l'aggressor → comparabilité).
- **Event-driven** : à chaque `BookUpdate` dans la fenêtre, on met à jour les accumulateurs ;
  la **pondération temporelle** utilise les timestamps (durée pendant laquelle un niveau a
  tenu une quantité).
- Émet `BarUpdate` en cours de fenêtre, `BarClose` à la fin (cohérent avec l'aggressor).

## Forme esquissée (détail → Phase 7)

- Trait analogue aux lentilles agressives : un accumulateur `on_book_event` / `on_close`,
  **composable** (on choisit les profils voulus).
- État léger dérivé du book courant ; éviter de recopier le book entier à chaque event
  (perf — cf. [`../transverse/README.md`](../transverse/README.md)).

## Frontière

On **résume** la liquidité ; on ne dit pas ce qu'elle *signifie*. L'absorption, la
détection d'icebergs (qui croisent agressif × passif) = **interprétation, hors scope** —
le consommateur les calcule via le point d'extension.

---

## Fiches features (Phase 5)
- **`LP-1`** — Profil de liquidité pondéré-temps · **P2** · *quantité moyenne par niveau, pondérée par durée.*
- **`LP-2`** — Snapshots ouverture/clôture · **P2** · *book figé aux bornes de la période.*
- **`LP-3`** — Churn (volumes add/cancel) · **P2** · *activité du book sur la période.*
- **`LP-4`** — Stats de depth (max/min, cumul par côté) · **P2** · *profondeur résumée.*
- **`LP-5`** — Déséquilibre bid/ask moyen · **P2** · *ratio moyen exposé.*
- **`LP-6`** — Émission `BarUpdate`/`BarClose` (alignée aggressor) · **P2** · *mêmes bornes que l'order flow.*
