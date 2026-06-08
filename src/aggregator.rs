//! `SymbolAggregator` : racine d'exécution par symbole (fiches `SYM-*`).
//!
//! Route les `MarketEvent` vers les périodes, fan-out, compose les lentilles order flow,
//! ferme les barres et notifie les abonnés. Déterministe (event-time). T0 : côté agressif.

use std::collections::VecDeque;

use crate::bar::Bar;
use crate::canonical::{BookUpdate, Granularity, Instrument, MarketEvent, Trade, Ts};
use crate::error::ConfigError;
use crate::extension::Subscriber;
use crate::orderflow::{Cvd, LensInstance, LensKind, OrderFlow};
use crate::passive::{LiquidityProfile, OrderBook, PassiveAggregator};
use crate::period::{Boundary, Period, TimePeriod};

/// Une période enregistrée + ses lentilles + sa barre en formation.
struct Slot {
    period: Box<dyn Period>,
    label: String,
    lens_kinds: Vec<LensKind>,
    /// Lentilles vivantes de la barre courante.
    lenses: Vec<LensInstance>,
    /// État inter-barres du cumulative delta (alimenté si une lentille `Delta` est active).
    cvd: Cvd,
    current: Option<Bar>,
    /// Historique FIFO borné des dernières barres fermées (issue #32) — `None` si désactivé
    /// (opt-in : aucune rétention par défaut).
    history: Option<VecDeque<Bar>>,
    /// Profondeur max de l'historique (0 = désactivé).
    history_depth: usize,
}

impl Slot {
    fn fresh_lenses(&self) -> Vec<LensInstance> {
        self.lens_kinds
            .iter()
            .map(|&k| LensInstance::from_kind(k))
            .collect()
    }

    /// Construit l'`OrderFlow` de la barre courante (snapshot des lentilles + CVD).
    fn snapshot_orderflow(&mut self) -> OrderFlow {
        let mut of = OrderFlow::default();
        for lens in &mut self.lenses {
            if let Some(bar_delta) = lens.snapshot_into(&mut of) {
                of.cvd = Some(self.cvd.push_bar_delta(bar_delta));
            }
        }
        of
    }

    /// Pousse une barre fermée dans l'historique FIFO borné (issue #32). Sans effet si
    /// l'historique est désactivé. La plus ancienne tombe quand la profondeur est atteinte.
    fn push_history(&mut self, bar: &Bar) {
        if let Some(h) = self.history.as_mut() {
            if h.len() == self.history_depth {
                h.pop_front();
            }
            h.push_back(bar.clone());
        }
    }

    /// `OrderFlow` **courant** de la barre en formation, en **lecture seule** (issue #31) :
    /// snapshot des lentilles vivantes **sans** les muter ni commiter le CVD. Le CVD courant
    /// = cumul des barres déjà fermées + delta de la barre en cours.
    fn forming_orderflow(&self) -> OrderFlow {
        let mut of = OrderFlow::default();
        for lens in &self.lenses {
            if let Some(bar_delta) = lens.snapshot_ref(&mut of) {
                of.cvd = Some(self.cvd.value() + bar_delta);
            }
        }
        of
    }
}

/// État d'un frame au tick courant (issue #32) : `[X dernières barres fermées] + [barre en
/// formation]`. Renvoyé par `SymbolAggregator::snapshot`.
#[derive(Debug, Clone, PartialEq)]
pub struct FrameSnapshot {
    /// Libellé de la période (cf. `Period::label`).
    pub label: String,
    /// Barres fermées retenues, de la plus ancienne à la plus récente (`≤ depth`). Vide si
    /// l'historique n'est pas activé pour ce frame.
    pub closed: Vec<Bar>,
    /// Barre en formation (OHLCV + order flow courants), `None` si aucune barre ouverte.
    pub forming: Option<Bar>,
}

/// Spécification d'une période au builder : `(période, lentilles, profondeur d'historique
/// override)` — `None` → profondeur globale.
type PeriodSpec = (Box<dyn Period>, Vec<LensKind>, Option<usize>);

/// Constructeur (fiches `SYM-5`/`SYM-6`) avec **fail-fast** sur la granularité.
pub struct Builder {
    instrument: Instrument,
    granularity: Granularity,
    specs: Vec<PeriodSpec>,
    passive: bool,
    passive_window: Option<i64>,
    /// Profondeur d'historique appliquée par défaut à toutes les périodes (issue #32).
    default_history_depth: usize,
}

impl Builder {
    /// Ajoute une période sans lentille.
    pub fn with_period(self, period: Box<dyn Period>) -> Self {
        self.with_period_and_lenses(period, Vec::new())
    }

    /// Ajoute une période avec ses lentilles order flow (fiche `OF-COMP`).
    pub fn with_period_and_lenses(
        mut self,
        period: Box<dyn Period>,
        lenses: Vec<LensKind>,
    ) -> Self {
        self.specs.push((period, lenses, None));
        self
    }

    /// Historique FIFO global (issue #32) : retient les `depth` dernières barres fermées de
    /// **chaque** période (sauf override par période). Opt-in : sans cet appel, aucune
    /// rétention (empreinte mémoire inchangée). `depth = 0` désactive.
    pub fn with_history(mut self, depth: usize) -> Self {
        self.default_history_depth = depth;
        self
    }

    /// Ajoute une période avec lentilles **et** une profondeur d'historique propre (issue
    /// #32), prioritaire sur la profondeur globale.
    pub fn with_period_lenses_history(
        mut self,
        period: Box<dyn Period>,
        lenses: Vec<LensKind>,
        depth: usize,
    ) -> Self {
        self.specs.push((period, lenses, Some(depth)));
        self
    }

    /// Raccourci : barre temporelle de `interval_ns` (fiche `AGG-P1`), sans lentille.
    pub fn with_time_period(self, interval_ns: i64) -> Self {
        self.with_period(Box::new(TimePeriod::new(interval_ns)))
    }

    /// Active le côté passif (reconstruction du carnet) — fiche `PAS-1`.
    /// Exige une granularité ≥ L2 (sinon `build()` échoue, fiche `PAS-3`).
    pub fn with_passive(mut self) -> Self {
        self.passive = true;
        self
    }

    /// Active le côté passif **avec profils de liquidité périodiques** de `window_ns`
    /// (fenêtres alignées sur l'horloge, fiches `LP-*`/`PAS-2`).
    pub fn with_liquidity_profile(mut self, window_ns: i64) -> Self {
        self.passive = true;
        self.passive_window = Some(window_ns);
        self
    }

    /// Valide la configuration et construit l'agrégateur.
    ///
    /// Échoue (fiches `SYM-8`/`CAN-7`/`TR-6`) si une période exige une granularité
    /// supérieure à celle déclarée.
    pub fn build(self) -> Result<SymbolAggregator, ConfigError> {
        for (p, _, _) in &self.specs {
            let required = p.min_granularity();
            if required > self.granularity {
                return Err(ConfigError::IncompatibleGranularity {
                    required,
                    declared: self.granularity,
                });
            }
        }
        // Le côté passif exige au moins du L2 (fiche `PAS-3`).
        if self.passive && self.granularity < Granularity::L2 {
            return Err(ConfigError::IncompatibleGranularity {
                required: Granularity::L2,
                declared: self.granularity,
            });
        }
        let default_depth = self.default_history_depth;
        let slots = self
            .specs
            .into_iter()
            .map(|(p, lens_kinds, depth_override)| {
                let label = p.label();
                let depth = depth_override.unwrap_or(default_depth);
                Slot {
                    period: p,
                    label,
                    lens_kinds,
                    lenses: Vec::new(),
                    cvd: Cvd::new(),
                    current: None,
                    history: (depth > 0).then(|| VecDeque::with_capacity(depth)),
                    history_depth: depth,
                }
            })
            .collect();
        Ok(SymbolAggregator {
            instrument: self.instrument,
            granularity: self.granularity,
            slots,
            subscribers: Vec::new(),
            passive: self.passive.then(|| match self.passive_window {
                Some(w) => PassiveAggregator::with_window(w),
                None => PassiveAggregator::new(),
            }),
            last_ts: None,
            out_of_order: 0,
        })
    }
}

/// Agrégateur d'un symbole.
pub struct SymbolAggregator {
    instrument: Instrument,
    granularity: Granularity,
    slots: Vec<Slot>,
    subscribers: Vec<Box<dyn Subscriber>>,
    /// Côté passif (carnet), présent si activé via `with_passive` (fiche `PAS-1`).
    passive: Option<PassiveAggregator>,
    /// Détection de désordre temporel (fiche `TR-5`).
    last_ts: Option<Ts>,
    out_of_order: u64,
}

impl SymbolAggregator {
    /// Démarre un constructeur pour un `Instrument` à la `Granularity` déclarée.
    pub fn builder(instrument: Instrument, granularity: Granularity) -> Builder {
        Builder {
            instrument,
            granularity,
            specs: Vec::new(),
            passive: false,
            passive_window: None,
            default_history_depth: 0,
        }
    }

    /// L'instrument agrégé.
    pub fn instrument(&self) -> Instrument {
        self.instrument
    }

    /// La granularité déclarée.
    pub fn granularity(&self) -> Granularity {
        self.granularity
    }

    /// Enregistre un abonné (fiche `EXT-1`).
    pub fn subscribe(&mut self, sub: Box<dyn Subscriber>) {
        self.subscribers.push(sub);
    }

    /// Nombre d'événements arrivés en **désordre temporel** (fiche `TR-5`).
    pub fn out_of_order_count(&self) -> u64 {
        self.out_of_order
    }

    /// Point d'entrée unique — live **et** replay (fiche `SYM-1`).
    pub fn process(&mut self, event: &MarketEvent) {
        self.note_ts(event.ts());
        match event {
            // Routage : un trade alimente le côté agressif (fiche `SYM-2`).
            MarketEvent::Trade(t) => self.on_trade(t),
            // Routage : un book update alimente le côté passif (fiche `SYM-3`).
            MarketEvent::BookUpdate(b) => self.on_book_update(b),
        }
    }

    /// Met à jour la détection de désordre temporel (fiche `TR-5`) — on ne rejette pas
    /// l'event. Partagé par `process` et `ingest_book_snapshot`.
    fn note_ts(&mut self, ts: Ts) {
        match self.last_ts {
            Some(prev) if ts < prev => self.out_of_order += 1,
            _ => self.last_ts = Some(ts),
        }
    }

    /// Ingère un **snapshot complet** du carnet à l'instant `ts` (fiche `SYM-3` étendue).
    ///
    /// Pour les flux par **snapshot** (MBP-10) où chaque message donne l'état du book à
    /// `t` (et non un delta) : remplace le carnet passif tel quel. Sans effet si le côté
    /// passif n'est pas actif. N'alimente **pas** le churn des profils de liquidité
    /// (celui-ci exige des deltas MBO) ; sert à garder `book()` synchronisé avec le tape
    /// pour l'échantillonnage par barre (cf. `on_bar_close_with_book`). Filtrage par
    /// instrument à la charge de l'appelant.
    pub fn ingest_book_snapshot(&mut self, ts: Ts, book: OrderBook) {
        self.note_ts(ts);
        if let Some(passive) = &mut self.passive {
            passive.replace_book(book);
        }
    }

    fn on_book_update(&mut self, b: &BookUpdate) {
        if b.instrument_id != self.instrument.id {
            return;
        }
        if let Some(passive) = &mut self.passive {
            // L'anomalie d'intégrité est tolérée (book borné) — cf. `OB-10`/`TR-7`.
            let _ = passive.apply(b);
        }
    }

    /// Carnet courant, si le côté passif est actif (fiche `EXT-6` — état interrogeable).
    pub fn book(&self) -> Option<&OrderBook> {
        self.passive.as_ref().map(|p| p.book())
    }

    /// Order flow **courant** de la barre en formation d'un frame (issue #31), sans la
    /// clôturer ni muter l'état (`&self`). `None` si le label est inconnu ou si aucune barre
    /// n'est ouverte. Reflète **tous les trades depuis l'ouverture** de la barre courante ;
    /// cohérent avec l'`OrderFlow` produit à la clôture si appelé juste avant. Le coût
    /// (snapshot) n'est payé qu'à l'appel → hot path inchangé.
    pub fn forming_orderflow(&self, period_label: &str) -> Option<OrderFlow> {
        let slot = self.slots.iter().find(|s| s.label == period_label)?;
        slot.current.as_ref()?;
        Some(slot.forming_orderflow())
    }

    /// Barre **en formation** complète d'un frame (issue #31) : `OHLCV` courant + order flow
    /// courant, marquée `partial` (incomplète par nature). `None` si label inconnu / pas de
    /// barre ouverte.
    pub fn forming_bar(&self, period_label: &str) -> Option<Bar> {
        let slot = self.slots.iter().find(|s| s.label == period_label)?;
        let mut bar = slot.current.clone()?;
        bar.orderflow = slot.forming_orderflow();
        bar.partial = true;
        Some(bar)
    }

    fn on_trade(&mut self, t: &Trade) {
        // Filtrage par instrument : un fichier DataBento mêle plusieurs échéances.
        if t.instrument_id != self.instrument.id {
            return;
        }
        // Fan-out vers toutes les périodes (fiche `SYM-4`).
        for slot in &mut self.slots {
            match slot.period.on_trade(t) {
                Boundary::Continue => {
                    let bar = slot
                        .current
                        .as_mut()
                        .expect("barre courante absente après ouverture");
                    bar.add(t);
                    for lens in &mut slot.lenses {
                        lens.on_trade(t);
                    }
                    notify_update(&mut self.subscribers, &slot.label, bar);
                }
                Boundary::CloseAndOpen {
                    start,
                    end,
                    partial,
                } => {
                    if slot.current.is_some() {
                        let of = slot.snapshot_orderflow();
                        let mut bar = slot.current.take().unwrap();
                        bar.orderflow = of;
                        // Carnet échantillonné au ts de clôture (fiche `EXT-7`, #18).
                        let book = self.passive.as_ref().map(|p| p.book());
                        notify(&mut self.subscribers, &slot.label, &bar, book);
                        // Rétention FIFO de la barre fermée (issue #32).
                        slot.push_history(&bar);
                    }
                    // Ouvre la nouvelle barre et ses lentilles fraîches.
                    slot.lenses = slot.fresh_lenses();
                    let mut bar = Bar::open(start, end, t);
                    bar.partial = partial;
                    for lens in &mut slot.lenses {
                        lens.on_trade(t);
                    }
                    notify_update(&mut self.subscribers, &slot.label, &bar);
                    slot.current = Some(bar);
                }
            }
        }
    }

    /// Finalise les barres en formation en fin de flux (fiche `SYM-11`).
    /// Les barres émises sont marquées `partial`.
    pub fn finish(&mut self) {
        for slot in &mut self.slots {
            if slot.current.is_some() {
                let of = slot.snapshot_orderflow();
                let mut bar = slot.current.take().unwrap();
                bar.partial = true;
                bar.orderflow = of;
                let book = self.passive.as_ref().map(|p| p.book());
                notify(&mut self.subscribers, &slot.label, &bar, book);
                slot.push_history(&bar);
            }
        }
        if let Some(passive) = &mut self.passive {
            passive.finish();
        }
    }

    /// Historique FIFO des dernières barres fermées d'un frame (issue #32), de la plus
    /// ancienne à la plus récente. `None` si le label est inconnu ou si l'historique n'a pas
    /// été activé (`with_history` / `with_period_lenses_history`).
    pub fn history(&self, period_label: &str) -> Option<&VecDeque<Bar>> {
        self.slots
            .iter()
            .find(|s| s.label == period_label)?
            .history
            .as_ref()
    }

    /// **Screenshot** complet de tous les frames à l'instant courant (issue #32) : par
    /// période, les `≤ X` dernières barres fermées (historique FIFO) + la barre en formation
    /// (avec son order flow courant). Les frames sans historique ont `closed` vide ; les
    /// frames sans barre ouverte ont `forming = None`.
    pub fn snapshot(&self) -> Vec<FrameSnapshot> {
        self.slots
            .iter()
            .map(|s| FrameSnapshot {
                label: s.label.clone(),
                closed: s
                    .history
                    .as_ref()
                    .map(|h| h.iter().cloned().collect())
                    .unwrap_or_default(),
                forming: self.forming_bar(&s.label),
            })
            .collect()
    }

    /// Récupère et vide les profils de liquidité fermés (fiches `LP-*`/`EXT-6`).
    /// Vide si le côté passif n'a pas de fenêtre configurée.
    pub fn drain_liquidity_profiles(&mut self) -> Vec<LiquidityProfile> {
        self.passive
            .as_mut()
            .map(|p| p.drain_profiles())
            .unwrap_or_default()
    }
}

fn notify(
    subscribers: &mut [Box<dyn Subscriber>],
    label: &str,
    bar: &Bar,
    book: Option<&OrderBook>,
) {
    for sub in subscribers.iter_mut() {
        sub.on_bar_close_with_book(label, bar, book);
    }
}

fn notify_update(subscribers: &mut [Box<dyn Subscriber>], label: &str, bar: &Bar) {
    for sub in subscribers.iter_mut() {
        sub.on_bar_update(label, bar);
    }
}
