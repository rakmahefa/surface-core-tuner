//! Source GPU : taux d'occupation calculé depuis RC6 residency.
//!
//! GPU busy% = 100 × (1 - Δrc6_ms / Δtime_ms).

use crate::core::*;
use std::time::Instant;

/// Calcule le busy% du GPU à partir du compteur RC6
/// (temps passé en deep sleep).
///
/// Nécessite 2 lectures consécutives pour calculer le delta.
pub struct GpuBusy {
    rc6_path: String,
    prev_rc6: Option<u64>,
    prev_time: Option<Instant>,
}

impl GpuBusy {
    /// `base` : répertoire DRM du GPU (voir `paths::gpu_dir()`).
    pub fn new(base: String) -> Self {
        Self {
            rc6_path: format!("{base}/gt/gt0/rc6_residency_ms"),
            prev_rc6: None,
            prev_time: None,
        }
    }
}

impl Source for GpuBusy {
    fn read(&mut self) -> Result<Vec<Sample>> {
        let raw = match std::fs::read_to_string(&self.rc6_path) {
            Ok(s) => s,
            Err(_) => return Ok(vec![]),
        };
        let rc6: u64 = match raw.trim().parse() {
            Ok(n) => n,
            Err(_) => return Ok(vec![]),
        };
        let now = Instant::now();

        let busy = match (self.prev_rc6, self.prev_time) {
            (Some(prev_rc6), Some(prev_time)) => {
                let dt_ms = now.duration_since(prev_time).as_secs_f64() * 1000.0;
                let drc6 = rc6.saturating_sub(prev_rc6) as f64;
                if dt_ms > 0.0 {
                    (1.0 - drc6 / dt_ms) * 100.0
                } else {
                    0.0
                }
            }
            _ => 0.0,
        };

        self.prev_rc6 = Some(rc6);
        self.prev_time = Some(now);

        Ok(vec![
            Sample {
                kind: "gpu".to_string(),
                label: "Busy".to_string(),
                value: Value::Num(busy.clamp(0.0, 100.0)),
                unit: Unit::Percent,
            },
            Sample {
                kind: "gpu".to_string(),
                label: "RC6 Residency".to_string(),
                value: Value::Num(rc6 as f64),
                unit: Unit::Ms,
            },
        ])
    }
}
