# extension — Point d'extension réactif

> Feuille. Parent : [`README.md`](README.md). Pilier **P5** (extensibilité).
>
> **Rôle** : exposer les données agrégées pour que **n'importe qui branche ses calculs**.
> La crate **expose**, n'interprète pas.

## Deux axes (cf. [`../vision/piliers.md`](../vision/piliers.md))

**Axe push / pull**
- *Push* : un trait `Subscriber` (l'utilisateur implémente `on_bar_close` / `on_bar_update`)
  et/ou des **channels** (`crossbeam`, `tokio::broadcast`) pour découpler / multi-consommateurs.
- *Pull* : exposer les bars comme un `Iterator` / `Stream`, ou un **état interrogeable**
  (snapshot à la demande).

**Axe granularité**
- `on_bar_close` (barre fermée) ;
- `on_bar_update` (barre en formation — footprint qui se remplit).

## Garanties
- **Alignement** : aggressor et passive émis sur les **mêmes bornes** de `Period`.
- **Coût** : trait générique monomorphisé pour le hot path ; les adaptateurs (channel,
  `Stream`) sont opt-in (cf. [`transverse/`](transverse/README.md)).

## Forme esquissée (détail → Phase 7)
Un **point d'extension unique** (le trait `Subscriber`) + adaptateurs optionnels vers
channel / `Stream`. « Prévoir push **et** pull » sans dupliquer le cœur.

## Frontière
Ici s'arrête la crate : au-delà du point d'extension, c'est le **consommateur** (layer
d'indicateurs, viz, stratégie) — un autre projet.

---

## Fiches features (Phase 5)

> Atomisation du thème F ([`../vision/features.md`](../vision/features.md)).

- **`EXT-1`** — Trait `Subscriber::on_bar_close` · **P0** · *un abonné reçoit chaque bar fermée.*
- **`EXT-2`** — `Subscriber::on_bar_update` (intra-bar) · **P1** · *un abonné reçoit les mises à jour de la bar en formation.*
- **`EXT-3`** — Dispatch générique zero-cost (hot path) · **P1** · *abonnement sans `dyn` ni alloc dans le hot path.*
- **`EXT-4`** — Adaptateur channel (push, multi-consommateurs) · **P1** · *fan-out vers plusieurs consommateurs découplés.*
- **`EXT-5`** — Adaptateur `Iterator`/`Stream` (pull) · **P1** · *consommation pull, composable.*
- **`EXT-6`** — État interrogeable (snapshot à la demande) · **P2** · *lecture de l'état courant sans abonnement.*
- **`EXT-7`** — Garantie d'alignement des deux côtés · **P1** · *aggressor et passive exposés sur mêmes bornes.*
- **`EXT-8`** — Cycle de vie des abonnés ((dés)abonnement) · **P2** · *on peut abonner/désabonner un subscriber proprement.* *(passe de relecture)*
