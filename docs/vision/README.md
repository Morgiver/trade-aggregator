# Vision — trade-aggregator

> Sortie de la **Phase 2 — Vision**. Du vrac (Découverte) à une vision cadrée.

**En une phrase** : une crate Rust **source-agnostic** qui agrège des données de marché
brutes (tape + book) en **order flow agressif** et **profils de liquidité passifs**, en
temps réel et en replay, sous un modèle déterministe — et qui **expose** ces données pour
que d'autres calculent (elle **n'interprète pas**).

## Les fichiers

| Fichier | Contenu |
|---|---|
| [`produit.md`](produit.md) | Proposition de valeur, public visé. |
| [`positionnement.md`](positionnement.md) | Différenciation vs l'existant, posture. |
| [`piliers.md`](piliers.md) | P1 dualité · P2 agréger≠interpréter · P3 source-agnostic · P4 event-time · P5 extensibilité. |
| [`scope.md`](scope.md) | Scope **IN / OUT** (garde-fou anti-creep). |
| [`features.md`](features.md) | Features par thème, priorisées P0→P3. |
| [`strategie.md`](strategie.md) | Tranches macro T0→T4. |
| [`risques-questions.md`](risques-questions.md) | Risques + questions pour Domaine/Archi. |

Matière amont : [`../ideation/idea.md`](../ideation/idea.md).
