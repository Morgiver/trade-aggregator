# Use-cases — T0 Walking skeleton

> Phase 7, étape 1. **Comportements atomiques** de T0 (*un comportement = une action*).
> Chaque UC référence les fiches qu'il réalise (cf. [`../../architecture/`](../../architecture/README.md)).
> Les **tests documentés** (critères d'acceptation) suivront en étape 2 ; le **code + tests** en étape 3.
>
> Périmètre T0 : `trades (DataBento) → SymbolAggregator → barres temporelles → on_bar_close`, **en replay**.

## Décodage de l'entrée

### `UC-T0-1` — Décoder un trade DataBento en `Trade` canonique
Donner un record *trades* DBN → obtenir un `Trade { ts, price, size, aggressor_side }`.
*Fiches* : `CAN-1`, `CAN-8`, `CAN-11`, `CAN-12`.

### `UC-T0-2` — Représenter l'absence de côté agresseur
Un trade DBN avec côté `None` → `Trade` avec `aggressor_side` = inconnu (pas d'échec).
*Fiches* : `CAN-3`.

## Construction & configuration

### `UC-T0-3` — Créer un `SymbolAggregator` (Instrument + Granularity + une `TimePeriod`)
Construire l'agrégateur configuré pour des barres de durée *n* (ex. 1 min).
*Fiches* : `CAN-4`, `CAN-5`, `CAN-6`, `SYM-1`, `SYM-5`, `SYM-6`, `AGG-P0`, `AGG-P1`.

### `UC-T0-4` — Refuser une configuration incompatible (fail-fast)
Demander une agrégation non supportée par la `Granularity` déclarée → **erreur à la construction**.
*Fiches* : `CAN-7`, `SYM-8`, `TR-6`.

## Agrégation

### `UC-T0-5` — Intégrer un trade dans la barre en formation
`process(trade)` ouvre (si besoin) et alimente la barre courante (OHLCV).
*Fiches* : `SYM-2`, `SYM-4`, `AGG-B1`, `AGG-B2`, `TR-3`, `TR-4`.

### `UC-T0-6` — Fermer la barre au franchissement de la période et émettre `on_bar_close`
Un trade dont le timestamp dépasse la borne → la barre courante se ferme (OHLCV finalisé)
et l'événement `BarClose` est émis ; une nouvelle barre s'ouvre.
*Fiches* : `AGG-P1`, `AGG-B4`, `EXT-1`.

### `UC-T0-7` — Finaliser la barre en formation en fin de flux (flush)
À la fin du replay, la dernière barre (partielle) est fermée proprement.
*Fiches* : `SYM-11`.

## Sortie & déterminisme

### `UC-T0-8` — Un `Subscriber` reçoit chaque barre fermée
Abonner un subscriber → il reçoit, dans l'ordre, chaque barre fermée via `on_bar_close`.
*Fiches* : `EXT-1`.

### `UC-T0-9` — Rejeu déterministe
Rejouer le même fichier de trades deux fois → **séquence de barres identique** (event-time,
aucune horloge système).
*Fiches* : `TR-3`, `SYM-1`.

---

## Couverture des fiches T0

`CAN-1` ✓ · `CAN-3` ✓ · `CAN-4` ✓ · `CAN-5` ✓ · `CAN-6` ✓ · `CAN-7` ✓ · `CAN-8` ✓ ·
`CAN-11` ✓ · `CAN-12` ✓ · `SYM-1` ✓ · `SYM-2` ✓ · `SYM-4` ✓ · `SYM-5` ✓ · `SYM-6` ✓ ·
`SYM-8` ✓ · `SYM-11` ✓ · `AGG-P0` ✓ · `AGG-P1` ✓ · `AGG-B1` ✓ · `AGG-B2` ✓ · `AGG-B4` ✓ ·
`EXT-1` ✓ · `TR-3` ✓ · `TR-4` ✓ · `TR-6` ✓ — **toutes les fiches T0 sont couvertes.**
