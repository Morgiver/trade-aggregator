# Changelog

Format inspiré de [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/).
On consigne les **changements notables** (fin de phase, tranche réalisée, décision/ADR, breaking) — pas chaque commit.

## [Non publié]

### Added
- Amorçage du projet : repo privé, squelette `docs/ideation/`, board Project #18, issue Phase 1.
- **Phase 1 — Découverte** : idée cadrée dans `docs/ideation/idea.md` (modèle
  SymbolAggregator / Aggressor / Passive, entrée DataBento L3/MBO, frontière de scope
  agrégation-vs-interprétation, live+replay event-time). Issue #1 close.
- **Phase 2 — Vision** : `docs/vision/` complet (produit, positionnement, 5 piliers,
  scope IN/OUT, features priorisées P0→P3, tranches macro T0→T4, risques & questions).
  Issue #2 close.
- **Phase 3 — Domaine** : `docs/domain/` (glossaire EN/FR + concepts & relations
  Mermaid). Vocabulaire stable. Issue #3 close.
- **Phase 4 — Architecture** : `docs/architecture/` complète (descente C4 : racine,
  canonical, symbol-aggregator, aggressor/orderflow, passive, extension, transverse).
  Chaque feature a un toit. Issue #4 close.
- **Phase 5 — Structuration** : 98 fiches atomiques co-localisées dans
  `docs/architecture/` (aucune feature orpheline). Issue #5 close.
- **Phase 6 — Priorisation** : `docs/roadmap.md` (tranches T0→T4 ordonnées). Issue #6
  close. **Partie documentaire (Phases 1→6) terminée.**
- Démarrage de la **Phase 7 — Réalisation** (issue #7) — tranche **T0 walking skeleton**
  sur branche `tranche/T0-walking-skeleton` : use-cases posés.
