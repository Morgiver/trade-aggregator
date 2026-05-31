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
