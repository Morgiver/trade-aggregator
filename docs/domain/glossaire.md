# Glossaire — trade-aggregator

> Vocabulaire métier **stable et non ambigu**. Convention : **concepts & code en
> anglais**, **définitions en français**. Ces concepts seront mappés sur des types Rust
> en Phase 4 (Architecture) — ici on définit le *sens*, pas la *structure*.
>
> 🔤 Piège FR/EN : en **anglais** *aggregate / aggregation / aggregator* prennent **deux
> g** ; *aggressor* prend **deux g et deux s**. En **français** *agréger / agrégation /
> agrégateur* et *agresseur* prennent **un seul g** (et un seul s).

## Entrée & données brutes

- **MarketEvent** — événement de marché horodaté en entrée. Deux familles : `Trade` et
  `BookUpdate`.
- **Trade** — transaction exécutée : un agresseur consomme de la liquidité. Porte
  timestamp, prix, taille, `AggressorSide`.
- **AggressorSide** — côté qui **initie** le trade : `Buy` (l'agresseur achète, lève
  l'ask) ou `Sell` (l'agresseur vend, frappe le bid). ⚠️ **Distinct** du côté du book
  (`Bid`/`Ask`). Mappé depuis DataBento à la frontière.
- **BookUpdate** — événement modifiant le carnet : ajout / annulation / modification d'un
  ordre passif (et fill).
- **Instrument** — l'actif tradé et ses paramètres : tick size, price increment,
  lot/contract size, multiplicateur, devise.
- **Granularity** — richesse de la donnée d'entrée : **L1** (BBO / top), **L2** (MBP, par
  prix), **L3** (MBO, par ordre). Déclarée à la création ; conditionne les agrégations
  possibles.

## Orchestration

- **SymbolAggregator** — concept **racine** pour un symbole : **lie** l'`AggressorAggregator`
  et le `PassiveAggregator`, porte l'`Instrument`, et **route** chaque `MarketEvent` vers
  le bon côté.
- **AggressorAggregator** — agrège le flux **agressif** (les `Trade`) en `Bar`, selon une
  ou plusieurs `Period`.
- **PassiveAggregator** — **maintient** l'`OrderBook` et agrège son état en
  `LiquidityProfile`.

## Agrégation agressive

- **Period** — *règle* qui décide quand une `Bar` se ferme. Familles : Time, Tick,
  Volume, Dollar/Notional, Range, Renko, Imbalance, Run, hybride.
- **Bar** — *résultat* agrégé d'une `Period` sur le flux agressif. Porte l'`OHLCV` et,
  selon configuration, `Footprint` / `Delta` / `POC` / `ValueArea` / `TPO`. (La lecture
  OHLC d'une `Bar` = une « candle » ; pas un concept distinct.)
- **OHLCV** — open / high / low / close + volume d'une `Bar`.

## Order flow (lentilles d'une `Bar`)

- **Footprint** — répartition du volume échangé **par niveau de prix et par côté**
  (Bid vs Ask) dans une `Bar`. Lentille **volume**.
- **Delta** — volume agressif acheteur − vendeur (sur une `Bar` ou un niveau).
- **CumulativeDelta (CVD)** — somme courante des `Delta` à travers les `Bar`.
- **POC (Point of Control)** — niveau de prix où le profil est maximal (volume ou temps).
- **ValueArea (VAH / VAL)** — fourchette de prix concentrant ~70 % de l'activité, bornée
  par Value Area High et Value Area Low.
- **TPO (Time Price Opportunity) / MarketProfile** — profil de distribution du **temps**
  passé à chaque niveau de prix sur une `Bar`. Lentille **temps** (vs `Footprint` =
  volume).

## Agrégation passive

- **OrderBook** — état du carnet : niveaux `Bid` et `Ask`, chacun avec un prix et une
  quantité (et, en L3, les ordres individuels).
- **Bid / Ask** — côtés du carnet : `Bid` = acheteurs passifs, `Ask` = vendeurs passifs.
- **LiquidityProfile** — agrégation **périodique** de l'état du book : profil de liquidité
  pondéré-temps, snapshots, churn (add/cancel), depth, déséquilibre bid/ask.

## Sortie

- **ExtensionPoint / Subscriber** — mécanisme par lequel un consommateur externe reçoit
  les données agrégées — sur fermeture (`BarClose`) ou mise à jour (`BarUpdate`) — pour y
  brancher **ses** calculs. La crate expose, ne calcule pas d'indicateur.
