//! Moteur de données : produit un [`UiState`] par tick.
//!
//! - **Réel** (défaut, natif) : `library::build_sampler()` lit sysfs/procfs,
//!   alimente le `Store` (dernière valeur + historique), puis [`data::snapshot`]
//!   projette le tout dans l'état typé de l'UI.
//! - **Démo** (`--features demo`, ou toute cible wasm) : mêmes types, mêmes
//!   chemins de code, valeurs synthétiques plausibles pour une Surface Pro 7
//!   (i5-1035G4). Sert à prévisualiser l'UI sans élévation et à produire des
//!   captures reproductibles.

use crate::core::{Sample, Store, Unit, Value};
use crate::data::{self, UiState};

/// Fréquence d'échantillonnage (Hz) — identique à l'ancienne GUI egui.
pub const TICK_HZ: f64 = 5.0;
/// Profondeur d'historique conservée (30 min × 5 Hz).
const HISTORY_CAP: usize = 30 * 60 * 5;

pub enum Engine {
    #[cfg(not(any(feature = "demo", target_family = "wasm")))]
    Real(RealEngine),
    #[allow(dead_code)] // construit sous `--features demo` ou cible wasm
    Demo(DemoEngine),
}

impl Engine {
    pub fn new() -> Self {
        #[cfg(any(feature = "demo", target_family = "wasm"))]
        return Engine::Demo(DemoEngine::new());
        #[cfg(not(any(feature = "demo", target_family = "wasm")))]
        Engine::Real(RealEngine::new())
    }

    /// Échantillonne toutes les sources et retourne l'état pour l'UI.
    pub fn tick(&mut self) -> UiState {
        match self {
            #[cfg(not(any(feature = "demo", target_family = "wasm")))]
            Engine::Real(real) => real.tick(),
            Engine::Demo(demo) => demo.tick(),
        }
    }
}

// ────────────────────────────────────────────
//  Moteur réel (sysfs/procfs)
// ────────────────────────────────────────────

#[cfg(not(any(feature = "demo", target_family = "wasm")))]
pub struct RealEngine {
    sampler: crate::core::Sampler,
    store: Store,
    governors: Vec<String>,
}

#[cfg(not(any(feature = "demo", target_family = "wasm")))]
impl RealEngine {
    fn new() -> Self {
        Self {
            sampler: crate::library::build_sampler(),
            store: Store::new(HISTORY_CAP),
            governors: Vec::new(),
        }
    }

    fn tick(&mut self) -> UiState {
        let samples = self.sampler.poll_all();
        self.store.push_batch(samples);
        data::snapshot(&self.store, &mut self.governors)
    }
}

// ────────────────────────────────────────────
//  Moteur démo — Surface Pro 7 (i5-1035G4) simulée
// ────────────────────────────────────────────

/// Générateurs : fonctions pures du temps simulé (secondes), partagées entre
/// le pré-remplissage des graphes et les ticks pour un tracé continu.
mod gen {
    /// Bruit déterministe doux (somme de sinus irrationnels) — reproductible,
    /// sans dépendance à un RNG.
    pub fn wobble(t: f64, seed: f64) -> f64 {
        (t * 0.21 + seed).sin() * 0.6 + (t * 0.47 + seed * 2.7).sin() * 0.4
    }

    /// Charge CPU0 : plancher 4 %, vague lente, pics périodiques.
    pub fn cpu_load(t: f64) -> f64 {
        let base = 8.0 + 6.0 * (t * 0.05).sin();
        let burst = 55.0 * (-((t - 20.0) / 14.0).powi(2)).exp().max(0.0)
            + 35.0 * (-((t - 155.0) / 10.0).powi(2)).exp().max(0.0)
            + 25.0 * (-((t - 255.0) / 18.0).powi(2)).exp().max(0.0);
        (base + burst + 4.0 * wobble(t, 1.3)).clamp(1.0, 99.0)
    }

    pub fn cpu_load_core(t: f64, core: f64) -> f64 {
        cpu_load(t + core * 3.1).clamp(1.0, 99.5)
    }

    /// Fréquence d'un cœur : suit sa charge (400 → 2667 MHz).
    pub fn cpu_freq(t: f64, core: f64) -> f64 {
        let load = cpu_load_core(t, core);
        let f = 400.0 + (2667.0 - 400.0) * (load / 100.0).powf(0.8);
        f + 25.0 * wobble(t, core)
    }

    /// Température package : suit la charge moyenne, inertie lente.
    pub fn cpu_temp(t: f64) -> f64 {
        let load = (0..8).map(|c| cpu_load_core(t, c as f64)).sum::<f64>() / 8.0;
        42.0 + 28.0 * (load / 100.0) + 1.5 * wobble(t, 7.0)
    }

    pub fn gpu_busy(t: f64) -> f64 {
        let base = 12.0 + 10.0 * (t * 0.04).sin();
        let burst = 70.0 * (-((t - 90.0) / 20.0).powi(2)).exp().max(0.0);
        (base + burst + 5.0 * wobble(t, 4.2)).clamp(0.5, 99.0)
    }

    pub fn gpu_freq(t: f64) -> f64 {
        300.0 + 750.0 * (gpu_busy(t) / 100.0)
    }

    pub fn mem_used_pct(t: f64) -> f64 {
        (38.0 + 4.0 * (t * 0.013).sin() + 1.5 * wobble(t, 9.1)).clamp(1.0, 99.0)
    }

    /// Débit lecture NVMe : rafales autour de t = 40 et t = 210.
    pub fn io_read_bps(t: f64) -> f64 {
        let burst =
            110.0 * (-((t - 40.0) / 9.0).powi(2)).exp() + 60.0 * (-((t - 210.0) / 14.0).powi(2)).exp();
        let base = 0.4 + 0.8 * (t * 0.09).sin().max(0.0);
        ((base + burst).max(0.05) * 1024.0 * 1024.0) * (1.0 + 0.08 * wobble(t, 3.3))
    }

    pub fn io_write_bps(t: f64) -> f64 {
        0.25 * io_read_bps(t + 4.0) + 1.2 * 1024.0 * 1024.0 * (t * 0.05).sin().max(0.0)
    }

    pub fn net_rx(t: f64) -> f64 {
        // KiB/s : téléchargée continue + vague
        900.0 + 2200.0 * (t * 0.06).sin().max(0.0) + 260.0 * wobble(t, 5.5)
    }

    pub fn net_tx(t: f64) -> f64 {
        120.0 + 380.0 * (t * 0.045).sin().max(0.0) + 60.0 * wobble(t, 2.2)
    }

    pub fn battery_pct(t: f64) -> f64 {
        86.0 - t * 0.0025 + 0.3 * (t * 0.01).sin()
    }

    pub fn power_w(t: f64) -> f64 {
        4.6 + 1.9 * (t * 0.03).sin() + 0.8 * wobble(t, 8.8)
    }
}

/// Constantes matérielles simulées — i5-1035G4 (Surface Pro 7).
mod hw {
    pub const CPU_HW_MIN: f64 = 400.0;
    pub const CPU_HW_MAX: f64 = 2667.0;
    pub const GPU_RP1: f64 = 300.0;
    pub const GPU_RP0: f64 = 1050.0;
    pub const GPU_BOOST: f64 = 1100.0;
    pub const MEM_TOTAL_KIB: f64 = 8_127_536.0; // ~7,75 GiB
}

pub struct DemoEngine {
    store: Store,
    governors: Vec<String>,
    /// Temps simulé (secondes écoulées depuis le démarrage).
    t: f64,
}

impl DemoEngine {
    #[allow(dead_code)] // idem : uniquement demo/wasm
    pub fn new() -> Self {
        let mut store = Store::new(HISTORY_CAP);

        // Pré-remplit 300 s d'historique pour les séries graphées : les
        // graphes sont pleins dès la première frame (captures reproductibles).
        //
        // Sur wasm, `Instant` est basé sur performance.now() (~0 au chargement
        // de la page) : impossible de dater « 300 s dans le passé » (soustraction
        // underflow) ou dans le futur (duration_since paniquerait). Tous les
        // points sont donc datés « maintenant » — l'ordre d'insertion (plus
        // ancienne d'abord) suffit au rendu, et chaque point reste dans la
        // fenêtre de 300 s le temps que les vrais ticks prennent le relais.
        let now = nyvoara_core::Instant::now();
        let series: [(&str, &str, fn(f64) -> f64); 7] = [
            ("cpu", "CPU0 Load", gen::cpu_load),
            ("cpu", "Package Temp", gen::cpu_temp),
            ("gpu", "Busy", gen::gpu_busy),
            ("memory", "Used%", gen::mem_used_pct),
            ("io", "nvme0n1 Read B/s", gen::io_read_bps),
            ("network", "RX Rate", gen::net_rx),
            ("network", "TX Rate", gen::net_tx),
        ];
        const N: usize = data::CHART_MAX_POINTS;
        for (kind, label, f) in series {
            let mut dq = std::collections::VecDeque::with_capacity(N);
            for k in 0..N {
                let t = -data::CHART_WINDOW_SECS + k as f64 * (data::CHART_WINDOW_SECS / N as f64);
                dq.push_back((now, f(t)));
            }
            store
                .history
                .insert(format!("{kind}/{label}"), dq);
        }

        let mut eng = Self { store, governors: Vec::new(), t: 0.0 };
        // Amorce `latest` avec un premier tick.
        let _ = eng.tick();
        eng
    }

    fn tick(&mut self) -> UiState {
        let t = self.t;
        self.t += 1.0 / TICK_HZ;

        let samples = self.samples(t);
        self.store.push_batch(samples);
        data::snapshot(&self.store, &mut self.governors)
    }

    /// Produit le lot complet d'échantillons « comme la vraie library ».
    fn samples(&self, t: f64) -> Vec<Sample> {
        use crate::paths::CPU_THREADS;
        let mut s = Vec::new();

        let num = |kind: &str, label: &str, v: f64, unit: Unit| Sample {
            kind: kind.into(),
            label: label.into(),
            value: Value::Num(v),
            unit,
        };
        let text = |kind: &str, label: &str, v: &str| Sample {
            kind: kind.into(),
            label: label.into(),
            value: Value::Text(v.into()),
            unit: Unit::Raw,
        };

        // ── CPU ──
        // Profil « Balanced » actif : gouverneur powersave, caps à 100 % HW.
        s.push(text("cpu", "Available Governors", "performance powersave"));
        s.push(num("cpu", "HW Min Freq", hw::CPU_HW_MIN, Unit::Mhz));
        s.push(num("cpu", "HW Max Freq", hw::CPU_HW_MAX, Unit::Mhz));
        for i in 0..CPU_THREADS {
            let fi = i as f64;
            let lbl = |s: &str| format!("CPU{i} {s}");
            s.push(num("cpu", &lbl("Load"), gen::cpu_load_core(t, fi), Unit::Percent));
            s.push(num("cpu", &lbl("Freq"), gen::cpu_freq(t, fi), Unit::Mhz));
            s.push(num("cpu", &lbl("Min"), hw::CPU_HW_MIN, Unit::Mhz));
            s.push(num("cpu", &lbl("Max"), hw::CPU_HW_MAX, Unit::Mhz));
            s.push(text("cpu", &lbl("Governor"), "powersave"));
        }
        s.push(num("cpu", "Package Temp", gen::cpu_temp(t), Unit::Celsius));
        for i in 0..4 {
            s.push(num(
                "cpu",
                &format!("Core{i} Temp"),
                gen::cpu_temp(t) - 1.5 - i as f64 * 0.4,
                Unit::Celsius,
            ));
        }
        s.push(num("cpu", "PSI some avg10", 1.2 + 2.5 * (gen::cpu_load(t) / 100.0), Unit::Percent));
        s.push(num("cpu", "PSI full avg10", 0.05, Unit::Percent));

        // ── GPU ──
        s.push(num("gpu", "Cur Freq", gen::gpu_freq(t), Unit::Mhz));
        s.push(num("gpu", "Act Freq", gen::gpu_freq(t) + 12.0, Unit::Mhz));
        s.push(num("gpu", "Min Freq", hw::GPU_RP1, Unit::Mhz));
        s.push(num("gpu", "Max Freq", hw::GPU_RP0, Unit::Mhz));
        s.push(num("gpu", "Boost Freq", hw::GPU_BOOST, Unit::Mhz));
        s.push(num("gpu", "RP0 Max", hw::GPU_RP0, Unit::Mhz));
        s.push(num("gpu", "RP1 Min", hw::GPU_RP1, Unit::Mhz));
        s.push(num("gpu", "RPS Cur Freq", gen::gpu_freq(t), Unit::Mhz));
        s.push(num("gpu", "RPS Min Freq", hw::GPU_RP1, Unit::Mhz));
        s.push(num("gpu", "RPS Max Freq", hw::GPU_RP0, Unit::Mhz));
        s.push(num("gpu", "RPS Boost Freq", hw::GPU_BOOST, Unit::Mhz));
        s.push(num("gpu", "Busy", gen::gpu_busy(t), Unit::Percent));
        s.push(num("gpu", "RC6 Residency", 100.0 - gen::gpu_busy(t) * 0.9, Unit::Percent));

        // ── Mémoire ──
        let used_pct = gen::mem_used_pct(t);
        let total = hw::MEM_TOTAL_KIB * 1024.0;
        let used = total * used_pct / 100.0;
        let cached = total * 0.22;
        let buffers = total * 0.03;
        let m = |label: &str, v: f64| num("memory", label, v, Unit::Bytes);
        s.push(m("Total", total));
        s.push(m("Used", used));
        s.push(num("memory", "Used%", used_pct, Unit::Percent));
        s.push(m("Available", total - used - cached * 0.4));
        s.push(m("Free", total - used - cached - buffers));
        s.push(m("Buffers", buffers));
        s.push(m("Cached", cached));
        s.push(m("Swap Total", 2_147_483_648.0));
        s.push(m("Swap Used", 214_748_365.0));
        s.push(m("Swap Free", 1_932_735_283.0));
        s.push(m("Swap Cached", 120.0 * 1024.0 * 1024.0));
        s.push(num("memory", "PSI some avg10", 0.4 + 0.8 * (t * 0.02).sin().max(0.0), Unit::Percent));
        s.push(num("memory", "PSI full avg10", 0.0, Unit::Percent));

        // ── Batterie ──
        let cap = gen::battery_pct(t);
        let energy_full = 43.2;
        let energy_design = 50.0;
        let b = |label: &str, v: f64, u: Unit| num("battery", label, v, u);
        s.push(b("Capacity", cap, Unit::Percent));
        s.push(b("Power", gen::power_w(t), Unit::W));
        s.push(b("Voltage", 11.58 + 0.04 * (t * 0.05).sin(), Unit::V));
        s.push(b("Energy Now", energy_full * cap / 100.0, Unit::Wh));
        s.push(b("Energy Full", energy_full, Unit::Wh));
        s.push(b("Energy Design", energy_design, Unit::Wh));
        s.push(b("Wear", (1.0 - energy_full / energy_design) * 100.0, Unit::Percent));
        s.push(b("Cycles", 312.0, Unit::Count));
        s.push(text("battery", "Status", "Discharging"));
        s.push(text("battery", "Manufacturer", "SMP"));
        s.push(text("battery", "Technology", "Li-Po"));
        s.push(b("AC Online", 0.0, Unit::Raw));

        // ── Thermique (zones ACPI + NVMe, comme thermal.rs) ──
        let zones = [
            ("ACPI Zone 1", 46.0),
            ("ACPI Zone 2", 38.0),
            ("ACPI Zone 3", 33.0),
            ("ACPI Zone 5", 44.5),
            ("ACPI Zone 6", 36.0),
            ("ACPI Zone 7", 41.0),
        ];
        for (i, (label, base)) in zones.iter().enumerate() {
            s.push(num("thermal", label, base + 2.0 * gen::wobble(t, i as f64), Unit::Celsius));
        }
        s.push(num("thermal", "NVMe Composite", 43.0 + 6.0 * gen::wobble(t, 1.0), Unit::Celsius));
        s.push(num("thermal", "NVMe Sensor 2", 46.5 + 5.0 * gen::wobble(t, 2.0), Unit::Celsius));
        s.push(num("thermal", "NVMe Sensor 3", 45.0 + 5.0 * gen::wobble(t, 3.0), Unit::Celsius));

        // ── I/O : nvme0n1 présent, sda absent ──
        let rb = gen::io_read_bps(t);
        let wb = gen::io_write_bps(t);
        let io = |label: &str, v: f64, u: Unit| num("io", label, v, u);
        s.push(io("nvme0n1 Read IOPS", rb / 4096.0 * 0.9, Unit::Count));
        s.push(io("nvme0n1 Write IOPS", wb / 4096.0 * 1.4, Unit::Count));
        s.push(io("nvme0n1 Read B/s", rb, Unit::Bytes));
        s.push(io("nvme0n1 Write B/s", wb, Unit::Bytes));
        s.push(io("nvme0n1 Total Reads", 2.31e12 + rb * t, Unit::Bytes));
        s.push(io("nvme0n1 Total Writes", 1.47e12 + wb * t, Unit::Bytes));
        s.push(io("PSI some avg10", 0.3 + 1.8 * (rb / (110.0 * 1024.0 * 1024.0)).max(0.0), Unit::Percent));
        s.push(io("PSI full avg10", 0.0, Unit::Percent));

        // ── Réseau ──
        let n = |label: &str, v: f64, u: Unit| num("network", label, v, u);
        s.push(n("RX Rate", gen::net_rx(t), Unit::KiBps));
        s.push(n("TX Rate", gen::net_tx(t), Unit::KiBps));
        s.push(n("RX Bytes", 4.61e9 + (gen::net_rx(t) * 1024.0) * t, Unit::Bytes));
        s.push(n("TX Bytes", 9.42e8 + (gen::net_tx(t) * 1024.0) * t, Unit::Bytes));
        s.push(n("WiFi Temp", 44.0 + 3.0 * gen::wobble(t, 6.6), Unit::Celsius));

        s
    }
}
