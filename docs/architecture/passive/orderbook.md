# passive/orderbook — Reconstruction du carnet

> Feuille (proche du code). Parent : [`README.md`](README.md).
>
> **Rôle** : maintenir l'**état instantané** de l'`OrderBook` à partir du flux de
> `BookUpdate`. C'est le **prérequis** de tout profil passif.

## Événements traités

| Event | Effet sur le book |
|---|---|
| **Add** | insère un ordre / ajoute de la quantité à un niveau de prix |
| **Cancel** | retire un ordre / réduit la quantité d'un niveau |
| **Modify** | change quantité et/ou prix d'un ordre |
| **Fill / Trade** | réduit la quantité passive consommée par une agression |
| **Snapshot / Clear** | réinitialise le book à un état connu (resynchronisation) |

## L2 vs L3

- **L2 (MBP)** : on tient une **quantité agrégée par niveau de prix**.
- **L3 (MBO)** : on tient les **ordres individuels** (id, file) ; le L2 s'en **dérive** par
  agrégation. On garde le L3 quand on l'a (richesse), on dégrade proprement en L2 sinon.

## Difficultés (risques connus — cf. [`../../vision/risques-questions.md`](../../vision/risques-questions.md))

- **Amorçage** : snapshot initial puis application incrémentale.
- **Trous de séquence / events désordonnés** : détecter (numéros de séquence) et
  **resynchroniser** (re-snapshot).
- **Cohérence** : un cancel sur un niveau inexistant, un croisement bid/ask → stratégie de
  tolérance à définir.
- *Référence* : guide officiel DataBento « Constructing the LOB » (on fait **maison** sur
  cette base).

## Forme esquissée (détail → Phase 7)

- `bids` / `asks` : maps **triées par prix** (ordre décroissant côté bid, croissant côté
  ask). Niveau = quantité agrégée (L2) **ou** liste d'ordres (L3).
- Sorties : meilleur bid/ask, profondeur N niveaux, snapshot complet à la demande.
- ⚠️ Structure de données = enjeu **perf** (cf. [`../transverse/README.md`](../transverse/README.md)).

## Frontière

La reconstruction **produit un état** ; elle n'en tire **aucune** conclusion (interprétation
= hors scope). Ce que les profils en font → [`liquidity-profile.md`](liquidity-profile.md).

---

## Fiches features (Phase 5)
- **`OB-1`** — Traitement `Add` · **P1** · *un ajout insère/incrémente le niveau.*
- **`OB-2`** — Traitement `Cancel` · **P1** · *une annulation retire/décrémente.*
- **`OB-3`** — Traitement `Modify` · **P1** · *une modif change quantité/prix.*
- **`OB-4`** — Traitement `Fill`/`Trade` · **P1** · *un fill réduit la quantité passive.*
- **`OB-5`** — `Snapshot`/`Clear` (amorçage & resync) · **P1** · *le book repart d'un état connu.*
- **`OB-6`** — Détection de trous de séquence · **P2** · *un gap déclenche une resynchronisation.*
- **`OB-7`** — Niveaux agrégés L2 (`prix → quantité`) · **P1** · *book L2 correct.*
- **`OB-8`** — Ordres individuels L3 (+ dérivation L2) · **P1** · *book L3 maintenu, L2 dérivable.*
- **`OB-9`** — Requêtes (best bid/ask, depth N, snapshot) · **P1** · *état interrogeable à la demande.*
- **`OB-10`** — Intégrité (book croisé bid≥ask, niveau négatif) · **P2** · *une incohérence est détectée et traitée (resync), jamais ignorée silencieusement.* *(passe de relecture)*
