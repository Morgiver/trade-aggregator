//! `SymbolAggregator` : racine d'exécution par symbole (fiches `SYM-*`).
//!
//! Route les `MarketEvent` vers les périodes, fan-out, ferme les barres et notifie les
//! abonnés. Déterministe (event-time). T0 : côté agressif seulement.

use crate::bar::Bar;
use crate::canonical::{Granularity, Instrument, MarketEvent, Trade};
use crate::error::ConfigError;
use crate::extension::Subscriber;
use crate::period::{Boundary, Period, TimePeriod};

/// Une période enregistrée + sa barre en formation.
struct Slot {
    period: Box<dyn Period>,
    label: String,
    current: Option<Bar>,
}

/// Constructeur (fiche `SYM-5`/`SYM-6`) avec **fail-fast** sur la granularité.
pub struct Builder {
    instrument: Instrument,
    granularity: Granularity,
    periods: Vec<Box<dyn Period>>,
}

impl Builder {
    /// Ajoute une période quelconque.
    pub fn with_period(mut self, period: Box<dyn Period>) -> Self {
        self.periods.push(period);
        self
    }

    /// Raccourci : ajoute une barre temporelle de `interval_ns` (fiche `AGG-P1`).
    pub fn with_time_period(self, interval_ns: i64) -> Self {
        self.with_period(Box::new(TimePeriod::new(interval_ns)))
    }

    /// Valide la configuration et construit l'agrégateur.
    ///
    /// Échoue (fiche `SYM-8`/`CAN-7`/`TR-6`) si une période exige une granularité
    /// supérieure à celle déclarée.
    pub fn build(self) -> Result<SymbolAggregator, ConfigError> {
        for p in &self.periods {
            let required = p.min_granularity();
            if required > self.granularity {
                return Err(ConfigError::IncompatibleGranularity {
                    required,
                    declared: self.granularity,
                });
            }
        }
        let slots = self
            .periods
            .into_iter()
            .map(|p| {
                let label = p.label();
                Slot {
                    period: p,
                    label,
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
            periods: Vec::new(),
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
                }
                Boundary::CloseAndOpen { start, end } => {
                    if let Some(bar) = slot.current.take() {
                        notify(&mut self.subscribers, &slot.label, &bar);
                    }
                    slot.current = Some(Bar::open(start, end, t));
                }
            }
        }
    }

    /// Finalise les barres en formation en fin de flux (fiche `SYM-11`).
    /// Les barres émises sont marquées `partial`.
    pub fn finish(&mut self) {
        for slot in &mut self.slots {
            if let Some(mut bar) = slot.current.take() {
                bar.partial = true;
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
