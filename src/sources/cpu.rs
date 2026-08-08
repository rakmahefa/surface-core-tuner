//! Source CPU : charge processeur depuis /proc/stat.
//!
//! Calcule le pourcentage d'utilisation par cœur
//! à partir des compteurs ticks (user, nice, system, idle, ...).

use crate::core::*;
use crate::paths;
use std::collections::HashMap;

/// Compteurs pour un cœur.
#[derive(Default, Clone, Copy)]
struct CpuCounters {
    user: u64,
    nice: u64,
    system: u64,
    idle: u64,
    iowait: u64,
    irq: u64,
    softirq: u64,
    steal: u64,
}

impl CpuCounters {
    fn total(&self) -> u64 {
        self.user
            + self.nice
            + self.system
            + self.idle
            + self.iowait
            + self.irq
            + self.softirq
            + self.steal
    }
}

/// Charge CPU % entre deux lectures de compteurs :
/// `(Δtotal - Δidle) / Δtotal`.
fn load_pct(prev: &CpuCounters, cur: &CpuCounters) -> f64 {
    let d_total = cur.total().saturating_sub(prev.total());
    let d_idle = cur.idle.saturating_sub(prev.idle);
    if d_total > 0 {
        100.0 * (d_total - d_idle) as f64 / d_total as f64
    } else {
        0.0
    }
}

/// Charge CPU par cœur.
///
/// Principe : on lit `/proc/stat`, on mémorise les compteurs,
/// au prochain appel on calcule le delta.
pub struct CpuLoad {
    prev: HashMap<String, CpuCounters>,
    first: bool,
}

impl CpuLoad {
    pub fn new() -> Self {
        Self {
            prev: HashMap::new(),
            first: true,
        }
    }

    fn parse_proc_stat(&self) -> Result<HashMap<String, CpuCounters>> {
        let content = std::fs::read_to_string(paths::PROC_STAT)
            .map_err(|e| format!("{}: {}", paths::PROC_STAT, e))?;

        let mut map = HashMap::new();
        for line in content.lines() {
            if !line.starts_with("cpu") {
                break;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 5 {
                continue;
            }
            let name = parts[0].to_string();
            if name == "cpu" {
                continue; // skip aggregate
            }
            let counters = CpuCounters {
                user: parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0),
                nice: parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0),
                system: parts.get(3).and_then(|s| s.parse().ok()).unwrap_or(0),
                idle: parts.get(4).and_then(|s| s.parse().ok()).unwrap_or(0),
                iowait: parts.get(5).and_then(|s| s.parse().ok()).unwrap_or(0),
                irq: parts.get(6).and_then(|s| s.parse().ok()).unwrap_or(0),
                softirq: parts.get(7).and_then(|s| s.parse().ok()).unwrap_or(0),
                steal: parts.get(8).and_then(|s| s.parse().ok()).unwrap_or(0),
            };
            map.insert(name, counters);
        }
        Ok(map)
    }
}

impl Source for CpuLoad {
    fn read(&mut self) -> Result<Vec<Sample>> {
        let current = self.parse_proc_stat()?;
        let mut samples = Vec::new();

        if self.first {
            self.prev = current.clone();
            self.first = false;
            // Premier cycle : pas de delta, on retourne 0
            for name in current.keys() {
                samples.push(Sample {
                    kind: "cpu".to_string(),
                    label: format!("{} Load", name.to_uppercase()),
                    value: Value::Num(0.0),
                    unit: Unit::Percent,
                });
            }
            return Ok(samples);
        }

        for (name, cur) in &current {
            if let Some(prev) = self.prev.get(name) {
                samples.push(Sample {
                    kind: "cpu".to_string(),
                    label: format!("{} Load", name.to_uppercase()),
                    value: Value::Num(load_pct(prev, cur)),
                    unit: Unit::Percent,
                });
            }
        }

        self.prev = current;
        Ok(samples)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn counters(user: u64, idle: u64) -> CpuCounters {
        CpuCounters {
            user,
            idle,
            ..Default::default()
        }
    }

    #[test]
    fn load_100_pct_when_no_idle_delta() {
        assert!((load_pct(&counters(100, 0), &counters(200, 0)) - 100.0).abs() < 1e-9);
    }

    #[test]
    fn load_0_pct_when_only_idle() {
        assert_eq!(load_pct(&counters(0, 100), &counters(0, 200)), 0.0);
    }

    #[test]
    fn load_half_when_half_idle() {
        // Δtotal = 100, Δidle = 50 → 50 %
        assert!((load_pct(&counters(0, 100), &counters(50, 150)) - 50.0).abs() < 1e-9);
    }

    #[test]
    fn load_0_when_no_delta() {
        let c = counters(10, 10);
        assert_eq!(load_pct(&c, &c), 0.0);
    }
}
