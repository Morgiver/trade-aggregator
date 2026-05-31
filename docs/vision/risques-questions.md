# Risques & questions ouvertes — trade-aggregator

> Ce qu'on garde à l'œil, et ce qu'on laisse explicitement à trancher en
> **Phase 3 — Domaine** et **Phase 4 — Architecture**. Sources :
> [`piliers.md`](piliers.md), [`scope.md`](scope.md).

## Risques

### Techniques
| Risque | Mitigation |
|---|---|
| **Reconstruction du carnet délicate** (add/cancel/modify/fill, snapshots, gaps, resync) — bugs subtils. | Guide officiel DataBento, **golden dataset**, tests déterministes par replay. |
| **Tension richesse ↔ low-latency** : footprint & profils de liquidité sont gourmands en structures/alloc. | Structures pensées dès l'archi (transverse perf), esprit zero-alloc dans le hot path. |
| **Côté agresseur `None`** (DataBento ou crypto) → footprint/delta dégradés. | Fallback d'inférence (tick rule) **optionnel** ; sinon barre marquée « side inconnu ». |
| **Alignement temporel Aggressor ⟷ Passive** malgré des cadences d'events différentes. | Garanti par event-time + bornes de barre communes ; à tester explicitement. |
| **Format canonique trop/pas assez général** (couvrir DataBento sans fuite d'abstraction). | Partir de DataBento (notre source réelle), **ne pas sur-généraliser** (YAGNI). |

### Scope & produit
| Risque | Mitigation |
|---|---|
| **Creep vers l'interprétation** (« juste un petit signal… »). | Pilier **P2** (agréger ≠ interpréter) en garde-fou permanent. |
| **Frontière floue sur des cas limites** (POC, Value Area = agrégation, mais d'autres seront ambigus). | Arbitrage au cas par cas contre la règle : *statistique de la distribution = IN ; conclusion = OUT*. |
| **Niche d'adoption** (public Rust quant restreint). | Acceptable : projet d'abord **brique fondation pour Morgan** ; adoption externe = bonus. |

### Externes
| Risque | Mitigation |
|---|---|
| **Dépendance au format DataBento** (évolution de DBN). | Mapping **isolé / feature-gated** ; le cœur ne dépend que du format canonique. |

## Questions ouvertes

### Pour la Phase 3 — Domaine (vocabulaire)
- Termes à fixer : *barre* / *candle* / *période* ? *Aggressor* / *Passive* gardés tels quels ?
- Le « côté » : *Bid/Ask* vs *Buy/Sell* vs *Aggressor side* — choisir **un** terme.
- Glossaire à définir : Trade, BookDelta, Footprint (cell), Delta/CVD, POC, Value Area,
  TPO, Profile de liquidité, Instrument…

### Pour la Phase 4 — Architecture
- **Format canonique** : enum d'événements unique vs traits ? types maison vs exposition de `dbn` ?
- **Point d'extension** : trait générique (zero-cost) vs `dyn` ? un trait ou plusieurs ?
- **Représentation du book** : structure (BTreeMap par prix ? arène ?) — arbitrage perf.
- **Temps** : type d'horodatage, garanties d'ordre des events.
- **Granularité / capacités** : encodage **type-level** (fail-fast à la compilation) vs runtime ?
- **Multi-symbole** : confirmer `SymbolAggregator` mono + couche d'orchestration au-dessus.
