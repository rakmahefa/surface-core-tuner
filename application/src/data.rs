//! État typé consommé par l'UI.
//!
//! La library produit des `Sample` génériques (clé `"kind/label"`), parfaits
//! pour le stockage mais peu pratiques dans du rsx. Chaque tick, le moteur
//! ([`crate::engine`]) projette ce flot stringly-typé dans des structures
//! typées — `Option<f64>` / `Option<String>` par métrique — que les pages
//! consomment directement. La `View` brute est conservée pour la logique des
//! profils (`is_active`, `from_store`), qui reste dans `nyvoara-core`.

use crate::core::{ReadLatest, Store, View};

// ────────────────────────────────────────────
//  État par sous-système
// ────────────────────────────────────────────

/// Lecture d'un cœur logique.
#[derive(Clone, Default)]
pub struct CoreState {
    pub load: Option<f64>,
    pub freq: Option<f64>,
    pub governor: Option<String>,
    pub min: Option<f64>,
    pub max: Option<f64>,
}

#[derive(Clone, Default)]
pub struct CpuState {
    pub cores: Vec<CoreState>,
    pub hw_min: Option<f64>,
    pub hw_max: Option<f64>,
    /// Gouverneurs disponibles — constante noyau.
    pub governors: Vec<String>,
    pub package_temp: Option<f64>,
    pub core_temps: [Option<f64>; 4],
    pub psi_some: Option<f64>,
    pub psi_full: Option<f64>,
}

impl CpuState {
    /// Charge moyenne des cœurs lisibles.
    pub fn avg_load(&self) -> Option<f64> {
        let loads: Vec<f64> = self.cores.iter().filter_map(|c| c.load).collect();
        (!loads.is_empty()).then(|| loads.iter().sum::<f64>() / loads.len() as f64)
    }
}

#[derive(Clone, Default)]
pub struct GpuState {
    /// Vrai dès qu'au moins une métrique GPU est lue (carte DRM présente).
    pub present: bool,
    pub busy: Option<f64>,
    pub rc6: Option<f64>,
    pub cur_freq: Option<f64>,
    pub act_freq: Option<f64>,
    /// Réglages actuels (RP1/RP0).
    pub min_freq: Option<f64>,
    pub max_freq: Option<f64>,
    /// Bornes matérielles.
    pub rp1_min: Option<f64>,
    pub rp0_max: Option<f64>,
    pub boost: Option<f64>,
    pub rps_cur: Option<f64>,
    pub rps_min: Option<f64>,
    pub rps_max: Option<f64>,
    pub rps_boost: Option<f64>,
}

#[derive(Clone, Default)]
pub struct MemState {
    pub total: Option<f64>,
    pub used: Option<f64>,
    pub used_pct: Option<f64>,
    pub available: Option<f64>,
    pub free: Option<f64>,
    pub buffers: Option<f64>,
    pub cached: Option<f64>,
    pub swap_total: Option<f64>,
    pub swap_used: Option<f64>,
    pub swap_free: Option<f64>,
    pub swap_cached: Option<f64>,
    pub psi_some: Option<f64>,
    pub psi_full: Option<f64>,
}

#[derive(Clone, Default)]
pub struct BatState {
    pub present: bool,
    pub capacity: Option<f64>,
    pub power: Option<f64>,
    pub voltage: Option<f64>,
    pub energy_now: Option<f64>,
    pub energy_full: Option<f64>,
    pub energy_design: Option<f64>,
    pub wear: Option<f64>,
    pub cycles: Option<f64>,
    pub status: Option<String>,
    pub manufacturer: Option<String>,
    pub technology: Option<String>,
    /// 1 = secteur branché.
    pub ac_online: Option<f64>,
}

/// Lecture d'un disque.
#[derive(Clone, Default)]
pub struct DevIo {
    pub name: String,
    pub present: bool,
    pub read_iops: Option<f64>,
    pub write_iops: Option<f64>,
    pub read_bps: Option<f64>,
    pub write_bps: Option<f64>,
    pub total_reads: Option<f64>,
    pub total_writes: Option<f64>,
}

#[derive(Clone, Default)]
pub struct IoState {
    pub devices: Vec<DevIo>,
    pub psi_some: Option<f64>,
    pub psi_full: Option<f64>,
}

#[derive(Clone, Default)]
pub struct NetState {
    pub rx_rate: Option<f64>,
    pub tx_rate: Option<f64>,
    pub rx_bytes: Option<f64>,
    pub tx_bytes: Option<f64>,
    pub wifi_temp: Option<f64>,
}

// ────────────────────────────────────────────
//  Séries temporelles (graphes)
// ────────────────────────────────────────────

/// Série prête à dessiner : valeurs homothétiques, de la plus ancienne (0)
/// à la plus récente, fenêtre `window_secs`.
#[derive(Clone, Default, PartialEq)]
pub struct Series {
    /// Valeurs ordonnées, plus ancienne d'abord. Vidé = pas encore de données.
    pub points: Vec<f64>,
    /// Fenêtre couverte (secondes) — sert au libellé « last N min ».
    pub window_secs: f64,
    /// Borne Y si fixe (pourcentages), sinon auto-déduite des points.
    pub y_max: Option<f64>,
}

impl Series {
    pub fn last(&self) -> Option<f64> {
        self.points.last().copied()
    }

    /// Borne Y effective : fixe si fournie, sinon plafond « propre » déduit
    /// des points (1, 1.2, 1.5, 2, 2.5, 3, 4, 5, 6, 8 × 10^k) — la courbe
    /// occupe l'espace utile sans mentir sur les proportions.
    pub fn y_top(&self) -> f64 {
        self.y_max.unwrap_or_else(|| {
            let m = self.points.iter().copied().fold(0.0_f64, f64::max) * 1.15;
            nice_ceiling(m.max(4.0))
        })
    }
}

/// Plafond lisible le plus proche, par décade.
fn nice_ceiling(v: f64) -> f64 {
    let mag = 10f64.powf(v.log10().floor());
    let norm = v / mag;
    let nice = [
        1.0, 1.2, 1.5, 2.0, 2.5, 3.0, 4.0, 5.0, 6.0, 8.0, 10.0,
    ]
    .iter()
    .find(|n| **n >= norm)
    .copied()
    .unwrap_or(10.0);
    nice * mag
}

/// Toutes les séries affichées par l'UI.
#[derive(Clone, Default)]
pub struct Charts {
    pub cpu_load: Series,
    pub cpu_temp: Series,
    pub gpu_busy: Series,
    pub mem_used: Series,
    pub io_read: Series,
    pub net_rx: Series,
    pub net_tx: Series,
}

// ────────────────────────────────────────────
//  Snapshot global
// ────────────────────────────────────────────

/// Instantané complet de l'état machine, poussé à chaque tick.
#[derive(Clone, Default)]
pub struct UiState {
    /// Snapshot brut — requis par la logique des profils.
    pub view: View,
    pub cpu: CpuState,
    pub gpu: GpuState,
    pub mem: MemState,
    pub bat: BatState,
    pub thermal: Vec<(String, f64)>,
    pub io: IoState,
    pub net: NetState,
    pub charts: Charts,
    /// Profil actif, recalculé par le moteur (cosommateurs : header, Power).
    pub active_profile: Option<String>,
}

/// Fenêtre affichée par les graphes (5 minutes).
pub const CHART_WINDOW_SECS: f64 = 300.0;
/// Nombre maximum de points par série (1 point / 2 s).
pub const CHART_MAX_POINTS: usize = 160;

/// Extrait une série de l'historique du `Store` (dernière fenêtre, valeurs
/// seules, plus ancienne d'abord).
pub fn series_from_store(store: &Store, kind: &str, label: &str, y_max: Option<f64>) -> Series {
    let mut points = Vec::new();
    if let Some(history) = store.history(kind, label) {
        let now = nyvoara_core::Instant::now();
        points = history
            .iter()
            .filter(|(t, _)| now.duration_since(*t).as_secs_f64() < CHART_WINDOW_SECS)
            .map(|(_, v)| *v)
            .collect();
    }
    downsample(points, y_max)
}

/// Réduit une série à `CHART_MAX_POINTS` points (moyennage par blocs).
pub fn downsample(points: Vec<f64>, y_max: Option<f64>) -> Series {
    let window_secs = CHART_WINDOW_SECS;
    if points.len() <= CHART_MAX_POINTS {
        return Series { points, window_secs, y_max };
    }
    let step = points.len() as f64 / CHART_MAX_POINTS as f64;
    let mut out = Vec::with_capacity(CHART_MAX_POINTS);
    let mut i = 0.0;
    while i < points.len() as f64 {
        let lo = i.floor() as usize;
        let hi = ((i + step).ceil() as usize).min(points.len());
        let block = &points[lo..hi.max(lo + 1)];
        out.push(block.iter().sum::<f64>() / block.len() as f64);
        i += step;
    }
    Series { points: out, window_secs, y_max }
}

/// Projette le `Store` dans l'état typé consommé par l'UI.
pub fn snapshot(store: &Store, governors_cache: &mut Vec<String>) -> UiState {
    let view = View::snapshot(store);
    let num = |label: &str| view.num("cpu", label);
    let gnum = |label: &str| view.num("gpu", label);

    // ── CPU ──
    let n = crate::paths::CPU_THREADS;
    let cores = (0..n)
        .map(|i| CoreState {
            load: view.num("cpu", &format!("CPU{i} Load")),
            freq: view.num("cpu", &format!("CPU{i} Freq")),
            governor: view.text("cpu", &format!("CPU{i} Governor")),
            min: view.num("cpu", &format!("CPU{i} Min")),
            max: view.num("cpu", &format!("CPU{i} Max")),
        })
        .collect();

    if governors_cache.is_empty() {
        if let Some(list) = view.text("cpu", "Available Governors") {
            *governors_cache = list.split_whitespace().map(String::from).collect();
        }
    }

    let cpu = CpuState {
        cores,
        hw_min: num("HW Min Freq"),
        hw_max: num("HW Max Freq"),
        governors: governors_cache.clone(),
        package_temp: num("Package Temp"),
        core_temps: [
            num("Core0 Temp"),
            num("Core1 Temp"),
            num("Core2 Temp"),
            num("Core3 Temp"),
        ],
        psi_some: num("PSI some avg10"),
        psi_full: num("PSI full avg10"),
    };

    // ── GPU ──
    let mnum = |label: &str| view.num("memory", label);
    let bnum = |label: &str| view.num("battery", label);
    let gpu_present = view.by_kind("gpu").iter().any(|s| s.value.as_num().is_some());
    let gpu = GpuState {
        present: gpu_present,
        busy: gnum("Busy"),
        rc6: gnum("RC6 Residency"),
        cur_freq: gnum("Cur Freq"),
        act_freq: gnum("Act Freq"),
        min_freq: gnum("Min Freq"),
        max_freq: gnum("Max Freq"),
        rp1_min: gnum("RP1 Min"),
        rp0_max: gnum("RP0 Max"),
        boost: gnum("Boost Freq"),
        rps_cur: gnum("RPS Cur Freq"),
        rps_min: gnum("RPS Min Freq"),
        rps_max: gnum("RPS Max Freq"),
        rps_boost: gnum("RPS Boost Freq"),
    };

    // ── Mémoire ──
    let mem = MemState {
        total: mnum("Total"),
        used: mnum("Used"),
        used_pct: mnum("Used%"),
        available: mnum("Available"),
        free: mnum("Free"),
        buffers: mnum("Buffers"),
        cached: mnum("Cached"),
        swap_total: mnum("Swap Total"),
        swap_used: mnum("Swap Used"),
        swap_free: mnum("Swap Free"),
        swap_cached: mnum("Swap Cached"),
        psi_some: mnum("PSI some avg10"),
        psi_full: mnum("PSI full avg10"),
    };

    // ── Batterie ──
    let bat_samples = view.by_kind("battery");
    let bat = BatState {
        present: bat_samples.iter().any(|s| s.value.as_num().is_some()),
        capacity: bnum("Capacity"),
        power: bnum("Power"),
        voltage: bnum("Voltage"),
        energy_now: bnum("Energy Now"),
        energy_full: bnum("Energy Full"),
        energy_design: bnum("Energy Design"),
        wear: bnum("Wear"),
        cycles: bnum("Cycles"),
        status: view.text("battery", "Status"),
        manufacturer: view.text("battery", "Manufacturer"),
        technology: view.text("battery", "Technology"),
        ac_online: bnum("AC Online"),
    };

    // ── Thermique (zones ACPI + NVMe), triée par température décroissante —
    // la zone la plus chaude (donc la plus intéressante) s'affiche en tête.
    let mut thermal: Vec<(String, f64)> = view
        .by_kind("thermal")
        .iter()
        .filter_map(|s| s.value.as_num().map(|v| (s.label.clone(), v)))
        .collect();
    thermal.sort_by(|a, b| b.1.total_cmp(&a.1));

    // ── I/O ──
    let ionum = |dev: &str, label: &str| view.num("io", &format!("{dev} {label}"));
    let mut devices = Vec::new();
    for dev in [crate::paths::DISK_NVME, crate::paths::DISK_SD] {
        let present = view
            .by_kind("io")
            .iter()
            .any(|s| s.label.starts_with(dev));
        devices.push(DevIo {
            name: dev.to_string(),
            present,
            read_iops: ionum(dev, "Read IOPS"),
            write_iops: ionum(dev, "Write IOPS"),
            read_bps: ionum(dev, "Read B/s"),
            write_bps: ionum(dev, "Write B/s"),
            total_reads: ionum(dev, "Total Reads"),
            total_writes: ionum(dev, "Total Writes"),
        });
    }
    let io = IoState {
        devices,
        psi_some: view.num("io", "PSI some avg10"),
        psi_full: view.num("io", "PSI full avg10"),
    };

    // ── Réseau ──
    let net = NetState {
        rx_rate: view.num("network", "RX Rate"),
        tx_rate: view.num("network", "TX Rate"),
        rx_bytes: view.num("network", "RX Bytes"),
        tx_bytes: view.num("network", "TX Bytes"),
        wifi_temp: view.num("network", "WiFi Temp"),
    };

    // ── Graphes ──
    let charts = Charts {
        cpu_load: series_from_store(store, "cpu", "CPU0 Load", None),
        cpu_temp: series_from_store(store, "cpu", "Package Temp", None),
        gpu_busy: series_from_store(store, "gpu", "Busy", None),
        mem_used: series_from_store(store, "memory", "Used%", None),
        io_read: series_from_store(store, "io", "nvme0n1 Read B/s", None),
        net_rx: series_from_store(store, "network", "RX Rate", None),
        net_tx: series_from_store(store, "network", "TX Rate", None),
    };

    // ── Profil actif ──
    let active_profile = crate::profiles::load_all()
        .into_iter()
        .find(|p| p.is_active(&view))
        .map(|p| p.name);

    UiState { view, cpu, gpu, mem, bat, thermal, io, net, charts, active_profile }
}
