//! `SymbolAggregator` : racine d'exécution par symbole (fiches `SYM-*`).
//!
//! Route les `MarketEvent` vers les périodes, fan-out, compose les lentilles order flow,
//! ferme les barres et notifie les abonnés. Déterministe (event-time). T0 : côté agressif.

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
}

/// Constructeur (fiches `SYM-5`/`SYM-6`) avec **fail-fast** sur la granularité.
pub struct Builder {
    instrument: Instrument,
    granularity: Granularity,
    specs: Vec<(Box<dyn Period>, Vec<LensKind>)>,
    passive: bool,
    passive_window: Option<i64>,
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
        self.specs.push((period, lenses));
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
        for (p, _) in &self.specs {
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
        let slots = self
            .specs
            .into_iter()
            .map(|(p, lens_kinds)| {
                let label = p.label();
                Slot {
                    period: p,
                    label,
                    lens_kinds,
                    lenses: Vec::new(),
                    cvd: Cvd::new(),
                    current: None,
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
                        notify(&mut self.subscribers, &slot.label, &bar);
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
                notify(&mut self.subscribers, &slot.label, &bar);
            }
        }
        if let Some(passive) = &mut self.passive {
            passive.finish();
        }
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

fn notify(subscribers: &mut [Box<dyn Subscriber>], label: &str, bar: &Bar) {
    for sub in subscribers.iter_mut() {
        sub.on_bar_close(label, bar);
    }
}

fn notify_update(subscribers: &mut [Box<dyn Subscriber>], label: &str, bar: &Bar) {
    for sub in subscribers.iter_mut() {
        sub.on_bar_update(label, bar);
    }
}
