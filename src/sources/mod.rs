//! Sources génériques réutilisables : ScalarMetric, TextMetric.
//! + fonction `build_sampler()` qui enregistre toutes les sources.

use crate::core::*;
use crate::paths;

pub mod battery;
pub mod cpu;
pub mod gpu;
pub mod io;
pub mod memory;
pub mod network;
pub mod thermal;

// ────────────────────────────────────────────
//  ScalarMetric — source scalaire générique
// ────────────────────────────────────────────

/// Lit un fichier sysfs/procfs, parse un f64, applique un scale.
///
/// Couvre ~70% des métriques : fréquences, températures,
/// voltages, puissances, compteurs...
pub struct ScalarMetric {
    kind: &'static str,
    path: String,
    label: String,
    unit: Unit,
    /// `valeur = raw × scale`
    scale: f64,
}

impl ScalarMetric {
    pub fn new(kind: &'static str, path: &str, label: &str, unit: Unit, scale: f64) -> Self {
        Self {
            kind,
            path: path.to_string(),
            label: label.to_string(),
            unit,
            scale,
        }
    }
}

impl Source for ScalarMetric {
    fn read(&mut self) -> Result<Vec<Sample>> {
        let v = match read_f64(&self.path) {
            Ok(v) => v,
            Err(_) => return Ok(vec![]), // fichier inexistant → silencieux
        };
        Ok(vec![Sample {
            kind: self.kind.to_string(),
            label: self.label.clone(),
            value: Value::Num(v * self.scale),
            unit: self.unit.clone(),
        }])
    }
}

// ────────────────────────────────────────────
//  TextMetric — source textuelle générique
// ────────────────────────────────────────────

/// Lit un fichier sysfs et retourne son contenu comme texte.
pub struct TextMetric {
    kind: &'static str,
    path: String,
    label: String,
}

impl TextMetric {
    pub fn new(kind: &'static str, path: &str, label: &str) -> Self {
        Self {
            kind,
            path: path.to_string(),
            label: label.to_string(),
        }
    }
}

impl Source for TextMetric {
    fn read(&mut self) -> Result<Vec<Sample>> {
        let s = match read_trimmed(&self.path) {
            Ok(s) => s,
            Err(_) => return Ok(vec![]),
        };
        Ok(vec![Sample {
            kind: self.kind.to_string(),
            label: self.label.clone(),
            value: Value::Text(s),
            unit: Unit::Raw,
        }])
    }
}

// ────────────────────────────────────────────
//  Fabrique : construit toutes les sources
// ────────────────────────────────────────────

/// Crée et retourne un Sampler avec toutes les sources
/// de métriques enregistrées.
pub fn build_sampler() -> Sampler {
    let mut s = Sampler::new();

    // ── CPU ──────────────────────────────────
    // Fréquences par cœur (cur/min/max)
    for cpu in 0..paths::CPU_THREADS {
        let cpufreq = paths::cpufreq(cpu);
        add_scalar!(
            s,
            "cpu",
            &format!("{cpufreq}/scaling_cur_freq"),
            &format!("CPU{cpu} Freq"),
            Unit::Mhz,
            0.001 // kHz → MHz
        );
        add_scalar!(
            s,
            "cpu",
            &format!("{cpufreq}/scaling_min_freq"),
            &format!("CPU{cpu} Min"),
            Unit::Mhz,
            0.001
        );
        add_scalar!(
            s,
            "cpu",
            &format!("{cpufreq}/scaling_max_freq"),
            &format!("CPU{cpu} Max"),
            Unit::Mhz,
            0.001
        );
    }

    // Gouverneurs : disponibles (cpu0) + courant par cœur
    s.add(Box::new(TextMetric::new(
        "cpu",
        &format!("{}/scaling_available_governors", paths::cpufreq(0)),
        "Available Governors",
    )));
    for cpu in 0..paths::CPU_THREADS {
        s.add(Box::new(TextMetric::new(
            "cpu",
            &format!("{}/scaling_governor", paths::cpufreq(cpu)),
            &format!("CPU{cpu} Governor"),
        )));
    }
    add_scalar!(
        s,
        "cpu",
        &format!("{}/cpu0/cpufreq/cpuinfo_min_freq", paths::SYS_CPU),
        "HW Min Freq",
        Unit::Mhz,
        0.001
    );
    add_scalar!(
        s,
        "cpu",
        &format!("{}/cpu0/cpufreq/cpuinfo_max_freq", paths::SYS_CPU),
        "HW Max Freq",
        Unit::Mhz,
        0.001
    );

    // Températures CPU (package + 4 cœurs, coretemp)
    if let Some(dir) = paths::hwmon("coretemp") {
        add_scalar!(
            s,
            "cpu",
            &format!("{dir}/temp1_input"),
            "Package Temp",
            Unit::Celsius,
            0.001
        );
        for core in 0..4 {
            add_scalar!(
                s,
                "cpu",
                &format!("{dir}/temp{}_input", core + 2),
                &format!("Core{core} Temp"),
                Unit::Celsius,
                0.001
            );
        }
    }

    // Charge CPU (/proc/stat)
    s.add(Box::new(cpu::CpuLoad::new()));
    s.add(Box::new(memory::Pressure::new("cpu", paths::PRESSURE_CPU)));

    // ── GPU ──────────────────────────────────
    if let Some(gpu) = paths::gpu_dir() {
        add_scalar!(
            s,
            "gpu",
            &format!("{gpu}/gt_cur_freq_mhz"),
            "Cur Freq",
            Unit::Mhz,
            1.0
        );
        add_scalar!(
            s,
            "gpu",
            &format!("{gpu}/gt_act_freq_mhz"),
            "Act Freq",
            Unit::Mhz,
            1.0
        );
        add_scalar!(
            s,
            "gpu",
            &format!("{gpu}/gt_min_freq_mhz"),
            "Min Freq",
            Unit::Mhz,
            1.0
        );
        add_scalar!(
            s,
            "gpu",
            &format!("{gpu}/gt_max_freq_mhz"),
            "Max Freq",
            Unit::Mhz,
            1.0
        );
        add_scalar!(
            s,
            "gpu",
            &format!("{gpu}/gt_boost_freq_mhz"),
            "Boost Freq",
            Unit::Mhz,
            1.0
        );
        // Plages matérielles RP0/RP1 (bornes des sliders)
        add_scalar!(
            s,
            "gpu",
            &format!("{gpu}/gt_RP0_freq_mhz"),
            "RP0 Max",
            Unit::Mhz,
            1.0
        );
        add_scalar!(
            s,
            "gpu",
            &format!("{gpu}/gt_RP1_freq_mhz"),
            "RP1 Min",
            Unit::Mhz,
            1.0
        );
        // RPS frequencies (sous-répertoire gt0)
        let gt0 = &format!("{gpu}/gt/gt0");
        add_scalar!(
            s,
            "gpu",
            &format!("{gt0}/rps_cur_freq_mhz"),
            "RPS Cur Freq",
            Unit::Mhz,
            1.0
        );
        add_scalar!(
            s,
            "gpu",
            &format!("{gt0}/rps_min_freq_mhz"),
            "RPS Min Freq",
            Unit::Mhz,
            1.0
        );
        add_scalar!(
            s,
            "gpu",
            &format!("{gt0}/rps_max_freq_mhz"),
            "RPS Max Freq",
            Unit::Mhz,
            1.0
        );
        add_scalar!(
            s,
            "gpu",
            &format!("{gt0}/rps_boost_freq_mhz"),
            "RPS Boost Freq",
            Unit::Mhz,
            1.0
        );

        // GPU busy % (dérivé de RC6)
        s.add(Box::new(gpu::GpuBusy::new(gpu)));
    }

    // ── Memory ────────────────────────────────
    s.add(Box::new(memory::MemInfo::new()));
    s.add(Box::new(memory::Pressure::new(
        "memory",
        paths::PRESSURE_MEMORY,
    )));

    // ── Battery ───────────────────────────────
    let bat = paths::BATTERY;

    add_scalar!(
        s,
        "battery",
        &format!("{bat}/capacity"),
        "Capacity",
        Unit::Percent,
        1.0
    );
    add_scalar!(
        s,
        "battery",
        &format!("{bat}/power_now"),
        "Power",
        Unit::W,
        0.000001
    );
    add_scalar!(
        s,
        "battery",
        &format!("{bat}/voltage_now"),
        "Voltage",
        Unit::V,
        0.000001
    );
    add_scalar!(
        s,
        "battery",
        &format!("{bat}/energy_now"),
        "Energy Now",
        Unit::Wh,
        0.000001
    );
    add_scalar!(
        s,
        "battery",
        &format!("{bat}/energy_full"),
        "Energy Full",
        Unit::Wh,
        0.000001
    );
    add_scalar!(
        s,
        "battery",
        &format!("{bat}/energy_full_design"),
        "Energy Design",
        Unit::Wh,
        0.000001
    );
    add_scalar!(
        s,
        "battery",
        &format!("{bat}/cycle_count"),
        "Cycles",
        Unit::Cycles,
        1.0
    );
    s.add(Box::new(TextMetric::new(
        "battery",
        &format!("{bat}/status"),
        "Status",
    )));
    s.add(Box::new(TextMetric::new(
        "battery",
        &format!("{bat}/manufacturer"),
        "Manufacturer",
    )));
    s.add(Box::new(TextMetric::new(
        "battery",
        &format!("{bat}/technology"),
        "Technology",
    )));

    // Usure (dérivée des énergies déjà lues — pas de relecture fichier)
    s.derive(battery::derive_wear);

    // AC adapter
    add_scalar!(
        s,
        "battery",
        &format!("{}/online", paths::AC),
        "AC Online",
        Unit::Count,
        1.0
    );

    // ── Thermal ───────────────────────────────
    s.add(Box::new(thermal::ThermalZones::new()));

    // ── I/O ───────────────────────────────────
    s.add(Box::new(io::DiskStats::new(paths::DISK_NVME)));
    s.add(Box::new(io::DiskStats::new(paths::DISK_SD)));
    s.add(Box::new(memory::Pressure::new("io", paths::PRESSURE_IO)));

    // ── Network ───────────────────────────────
    if let Some(iface) = paths::wifi_iface() {
        let net = format!("/sys/class/net/{iface}/statistics");
        add_scalar!(
            s,
            "network",
            &format!("{net}/rx_bytes"),
            "RX Bytes",
            Unit::Bytes,
            1.0
        );
        add_scalar!(
            s,
            "network",
            &format!("{net}/tx_bytes"),
            "TX Bytes",
            Unit::Bytes,
            1.0
        );
        s.add(Box::new(network::NetworkThroughput::new(net)));

        // Température de la puce WiFi (iwlwifi)
        if let Some(dir) = paths::hwmon("iwlwifi") {
            add_scalar!(
                s,
                "network",
                &format!("{dir}/temp1_input"),
                "WiFi Temp",
                Unit::Celsius,
                0.001
            );
        }
    }

    s
}

/// Macro pour ajouter une ScalarMetric en une ligne.
macro_rules! add_scalar {
    ($sampler:expr, $kind:expr, $path:expr, $label:expr, $unit:expr, $scale:expr) => {
        $sampler.add(Box::new(ScalarMetric::new(
            $kind, $path, $label, $unit, $scale,
        )));
    };
}
pub(crate) use add_scalar;
