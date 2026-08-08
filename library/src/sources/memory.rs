//! Sources mémoire : /proc/meminfo et pression mémoire.
//!
//! /proc/meminfo contient toutes les métriques RAM/Swap
//! dans un format clé: valeur en kB.

use crate::core::*;
use crate::paths;

/// Parse `/proc/meminfo` et extrait les métriques clés.
///
/// Les valeurs sont en kB → converties en bytes (×1024).
/// Calcule aussi Used = Total - Available.
pub struct MemInfo {
    /// (préfixe ligne, label, unité)
    keys: Vec<(&'static str, &'static str, Unit)>,
}

impl MemInfo {
    pub fn new() -> Self {
        Self {
            keys: vec![
                ("MemTotal", "Total", Unit::Bytes),
                ("MemAvailable", "Available", Unit::Bytes),
                ("MemFree", "Free", Unit::Bytes),
                ("Buffers", "Buffers", Unit::Bytes),
                ("Cached", "Cached", Unit::Bytes),
                ("SwapTotal", "Swap Total", Unit::Bytes),
                ("SwapFree", "Swap Free", Unit::Bytes),
                ("SwapCached", "Swap Cached", Unit::Bytes),
            ],
        }
    }

    fn parse_kb(&self, content: &str) -> Vec<Sample> {
        let mut samples = Vec::new();
        for &(prefix, label, ref unit) in &self.keys {
            if let Some(line) = content.lines().find(|l| l.starts_with(prefix)) {
                if let Some(val_str) = line.split_whitespace().nth(1) {
                    if let Ok(kb) = val_str.parse::<f64>() {
                        samples.push(Sample {
                            kind: "memory".to_string(),
                            label: label.to_string(),
                            value: Value::Num(kb * 1024.0), // kB → bytes
                            unit: unit.clone(),
                        });
                    }
                }
            }
        }

        // Used = Total - Available
        let total = samples
            .iter()
            .find(|s| s.label == "Total")
            .and_then(|s| s.value.as_num())
            .unwrap_or(0.0);
        let avail = samples
            .iter()
            .find(|s| s.label == "Available")
            .and_then(|s| s.value.as_num())
            .unwrap_or(0.0);
        samples.push(Sample {
            kind: "memory".to_string(),
            label: "Used".to_string(),
            value: Value::Num(total - avail),
            unit: Unit::Bytes,
        });

        // Used%
        if total > 0.0 {
            samples.push(Sample {
                kind: "memory".to_string(),
                label: "Used%".to_string(),
                value: Value::Num((total - avail) / total * 100.0),
                unit: Unit::Percent,
            });
        }

        // Swap Used = SwapTotal - SwapFree
        let swap_total = samples
            .iter()
            .find(|s| s.label == "Swap Total")
            .and_then(|s| s.value.as_num())
            .unwrap_or(0.0);
        let swap_free = samples
            .iter()
            .find(|s| s.label == "Swap Free")
            .and_then(|s| s.value.as_num())
            .unwrap_or(0.0);
        samples.push(Sample {
            kind: "memory".to_string(),
            label: "Swap Used".to_string(),
            value: Value::Num(swap_total - swap_free),
            unit: Unit::Bytes,
        });

        samples
    }
}

impl Source for MemInfo {
    fn read(&mut self) -> Result<Vec<Sample>> {
        let content = std::fs::read_to_string(paths::PROC_MEMINFO)
            .map_err(|e| format!("{}: {}", paths::PROC_MEMINFO, e))?;
        Ok(self.parse_kb(&content))
    }
}

// ── Pressure Stall ──────────────────────────

/// Parse `/proc/pressure/{cpu,memory,io}`.
///
/// Format : `some avg10=0.00 avg60=0.00 avg300=0.00 total=0`
pub struct Pressure {
    kind: &'static str,
    path: String,
}

impl Pressure {
    pub fn new(kind: &'static str, path: &str) -> Self {
        Self {
            kind,
            path: path.to_string(),
        }
    }

    fn parse_pressure(&self, content: &str) -> Vec<Sample> {
        let mut samples = Vec::new();
        for line in content.lines() {
            if line.is_empty() {
                continue;
            }
            let prefix = line.split_whitespace().next().unwrap_or("");
            for field in &["avg10", "avg60", "avg300"] {
                if let Some(val) = line
                    .split_whitespace()
                    .find(|s| s.starts_with(field))
                    .and_then(|s| s.split('=').nth(1))
                    .and_then(|s| s.parse::<f64>().ok())
                {
                    samples.push(Sample {
                        kind: self.kind.to_string(),
                        label: format!("PSI {} {}", prefix, field),
                        value: Value::Num(val),
                        unit: Unit::Percent,
                    });
                }
            }
        }
        samples
    }
}

impl Source for Pressure {
    fn read(&mut self) -> Result<Vec<Sample>> {
        let content =
            std::fs::read_to_string(&self.path).map_err(|e| format!("{}: {}", self.path, e))?;
        Ok(self.parse_pressure(&content))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MEMINFO: &str = "\
MemTotal:       16293828 kB
MemFree:         3000000 kB
MemAvailable:    8000000 kB
Buffers:          500000 kB
Cached:          4000000 kB
SwapTotal:       2097152 kB
SwapFree:        1500000 kB
SwapCached:       100000 kB
";

    #[test]
    fn meminfo_parses_core_values() {
        let m = MemInfo::new();
        let samples = m.parse_kb(MEMINFO);

        let total = samples
            .iter()
            .find(|s| s.label == "Total")
            .and_then(|s| s.value.as_num())
            .unwrap();
        let used = samples
            .iter()
            .find(|s| s.label == "Used")
            .and_then(|s| s.value.as_num())
            .unwrap();
        let used_pct = samples
            .iter()
            .find(|s| s.label == "Used%")
            .and_then(|s| s.value.as_num())
            .unwrap();

        assert_eq!(total, 16293828.0 * 1024.0);
        assert_eq!(used, (16293828.0 - 8000000.0) * 1024.0);
        let expected_pct = (16293828.0 - 8000000.0) / 16293828.0 * 100.0;
        assert!((used_pct - expected_pct).abs() < 1e-9);
    }

    #[test]
    fn meminfo_parses_swap() {
        let m = MemInfo::new();
        let samples = m.parse_kb(MEMINFO);
        let swap_used = samples
            .iter()
            .find(|s| s.label == "Swap Used")
            .and_then(|s| s.value.as_num())
            .unwrap();
        assert_eq!(swap_used, (2097152.0 - 1500000.0) * 1024.0);
    }

    #[test]
    fn pressure_parses_some_and_full() {
        let p = Pressure::new("cpu", "/proc/pressure/cpu");
        let content =
            "some avg10=0.38 avg60=0.42 avg300=0.30 total=12345\nfull avg10=0.00 avg60=0.01 avg300=0.00 total=678\n";
        let samples = p.parse_pressure(content);

        assert_eq!(samples.len(), 6);
        assert!(samples.iter().all(|s| s.unit == Unit::Percent));

        let s10 = samples
            .iter()
            .find(|s| s.label == "PSI some avg10")
            .and_then(|s| s.value.as_num())
            .unwrap();
        let f60 = samples
            .iter()
            .find(|s| s.label == "PSI full avg60")
            .and_then(|s| s.value.as_num())
            .unwrap();
        assert!((s10 - 0.38).abs() < 1e-9);
        assert!((f60 - 0.01).abs() < 1e-9);
    }
}
