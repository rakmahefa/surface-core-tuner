//! Chemins des interfaces noyau (sysfs/procfs) — point de vérité unique
//! pour toute la configuration matérielle.
//!
//! Les indices hwmon et le numéro de carte DRM ne sont pas stables
//! entre deux boots (ordre de probe des drivers) : ils sont résolus
//! par nom, une seule fois par processus (cache `OnceLock`).

use std::collections::HashMap;
use std::sync::OnceLock;

const HWMON_BASE: &str = "/sys/class/hwmon";
const DRM_BASE: &str = "/sys/class/drm";
const NET_BASE: &str = "/sys/class/net";

// ── Chemins stables ─────────────────────────

pub const SYS_CPU: &str = "/sys/devices/system/cpu";
pub const SYS_BLOCK: &str = "/sys/block";
pub const PROC_STAT: &str = "/proc/stat";
pub const PROC_MEMINFO: &str = "/proc/meminfo";
pub const PRESSURE_CPU: &str = "/proc/pressure/cpu";
pub const PRESSURE_MEMORY: &str = "/proc/pressure/memory";
pub const PRESSURE_IO: &str = "/proc/pressure/io";
pub const BATTERY: &str = "/sys/class/power_supply/BAT1";
pub const AC: &str = "/sys/class/power_supply/ADP1";
pub const DISK_NVME: &str = "nvme0n1";
pub const DISK_SD: &str = "sda";

/// Nombre de threads CPU (4 cœurs / 8 threads — i5-1035G4).
pub const CPU_THREADS: usize = 8;

/// Répertoire cpufreq d'un cœur (`/sys/devices/system/cpu/cpuN/cpufreq`).
pub fn cpufreq(cpu: usize) -> String {
    format!("{SYS_CPU}/cpu{cpu}/cpufreq")
}

// ── Résolution par nom (indices instables) ──

/// Répertoire hwmon d'un driver, résolu via son fichier `name`.
///
/// Les drivers multi-instance sont suffixés (`iwlwifi_1`, ...) :
/// un nom `foo` matche aussi `foo_N`.
pub fn hwmon(name: &str) -> Option<String> {
    static CACHE: OnceLock<HashMap<String, String>> = OnceLock::new();
    let map = CACHE.get_or_init(|| {
        let mut m = HashMap::new();
        if let Ok(entries) = std::fs::read_dir(HWMON_BASE) {
            for e in entries.flatten() {
                let dir = e.path();
                if let Ok(n) = std::fs::read_to_string(dir.join("name")) {
                    m.insert(n.trim().to_string(), dir.to_string_lossy().to_string());
                }
            }
        }
        m
    });
    map.get(name).cloned().or_else(|| {
        map.iter()
            .find(|(n, _)| {
                n.strip_prefix(name)
                    .is_some_and(|rest| rest.starts_with('_'))
            })
            .map(|(_, path)| path.clone())
    })
}

/// Répertoire DRM du GPU — celui qui expose `gt_cur_freq_mhz`.
pub fn gpu_dir() -> Option<String> {
    static DIR: OnceLock<Option<String>> = OnceLock::new();
    DIR.get_or_init(|| {
        std::fs::read_dir(DRM_BASE)
            .ok()?
            .flatten()
            .find(|e| e.path().join("gt_cur_freq_mhz").exists())
            .map(|e| e.path().to_string_lossy().to_string())
    })
    .clone()
}

/// Nom de l'interface réseau sans fil (présence de `wireless/`).
pub fn wifi_iface() -> Option<String> {
    static IFACE: OnceLock<Option<String>> = OnceLock::new();
    IFACE
        .get_or_init(|| {
            std::fs::read_dir(NET_BASE)
                .ok()?
                .flatten()
                .find(|e| e.path().join("wireless").exists())
                .map(|e| e.file_name().to_string_lossy().to_string())
        })
        .clone()
}
