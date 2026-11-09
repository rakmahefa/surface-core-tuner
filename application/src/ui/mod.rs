//! Interface utilisateur — modules.
//!
//! - [`widgets`] : briques partagées (cartes, KPI, barres, graphes SVG,
//!   contrôles) ;
//! - [`shell`] : coquille applicative (barre latérale, entête, barre d'état) ;
//! - [`pages`] : une page par domaine (Overview, Power, CPU, …) ;
//! - [`tray`] : icône de notification et menu contextuel (desktop).

pub mod icons;
pub mod pages;
pub mod shell;
#[cfg(not(target_family = "wasm"))]
pub mod tray;
pub mod widgets;


/// Pages du dashboard — ordre de la barre latérale.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Page {
    Overview,
    Power,
    Cpu,
    Gpu,
    Memory,
    Storage,
    Network,
    Battery,
    Thermal,
}

impl Page {
    pub fn title(self) -> &'static str {
        match self {
            Page::Overview => "Overview",
            Page::Power => "Power Profiles",
            Page::Cpu => "CPU",
            Page::Gpu => "GPU",
            Page::Memory => "Memory",
            Page::Storage => "Storage",
            Page::Network => "Network",
            Page::Battery => "Battery",
            Page::Thermal => "Thermal",
        }
    }

    pub fn subtitle(self) -> &'static str {
        match self {
            Page::Overview => "Live snapshot of the machine",
            Page::Power => "Apply, save and manage power profiles",
            Page::Cpu => "Load, frequencies, governor and thermals",
            Page::Gpu => "Frequencies, utilisation and RC6 residency",
            Page::Memory => "RAM, swap and memory pressure",
            Page::Storage => "Disk throughput and I/O pressure",
            Page::Network => "Throughput and interface metrics",
            Page::Battery => "Charge, power draw and health",
            Page::Thermal => "Thermal zones across the SoC",
        }
    }
}
