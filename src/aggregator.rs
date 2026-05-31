//! `SymbolAggregator` : racine d'exécution par symbole (fiches `SYM-*`).
//!
//! Route les `MarketEvent` vers les périodes, fan-out, compose les lentilles order flow,
//! ferme les barres et notifie les abonnés. Déterministe (event-time). T0 : côté agressif.

use crate::bar::Bar;
use crate::canonical::{Granularity, Instrument, MarketEvent, Trade};
use crate::error::ConfigError;
use crate::extension::Subscriber;
use crate::orderflow::{Cvd, LensInstance, LensKind, OrderFlow};
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
        })
    }
}

/// Agrégateur d'un symbole.
pub struct SymbolAggregator {
    instrument: Instrument,
    granularity: Granularity,
    slots: Vec<Slot>,
    subscribers: Vec<Box<dyn Subscriber>>,
}

impl SymbolAggregator {
    /// Démarre un constructeur pour un `Instrument` à la `Granularity` déclarée.
    pub fn builder(instrument: Instrument, granularity: Granularity) -> Builder {
        Builder {
            instrument,
            granularity,
            specs: Vec::new(),
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

    /// Point d'entrée unique — live **et** replay (fiche `SYM-1`).
    pub fn process(&mut self, event: &MarketEvent) {
        match event {
            // Routage : un trade alimente le côté agressif (fiche `SYM-2`).
            MarketEvent::Trade(t) => self.on_trade(t),
        }
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
                    slot.current
                        .as_mut()
                        .expect("barre courante absente après ouverture")
                        .add(t);
                    for lens in &mut slot.lenses {
                        lens.on_trade(t);
                    }
                }
                Boundary::CloseAndOpen { start, end } => {
                    if slot.current.is_some() {
                        let of = slot.snapshot_orderflow();
                        let mut bar = slot.current.take().unwrap();
                        bar.orderflow = of;
                        notify(&mut self.subscribers, &slot.label, &bar);
                    }
                    // Ouvre la nouvelle barre et ses lentilles fraîches.
                    slot.lenses = slot.fresh_lenses();
                    let mut bar = Bar::open(start, end, t);
                    bar.partial = false;
                    for lens in &mut slot.lenses {
                        lens.on_trade(t);
                    }
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
    }
}

fn notify(subscribers: &mut [Box<dyn Subscriber>], label: &str, bar: &Bar) {
    for sub in subscribers.iter_mut() {
        sub.on_bar_close(label, bar);
    }
}
