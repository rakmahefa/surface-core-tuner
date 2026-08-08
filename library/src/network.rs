//! Source réseau : débits RX/TX par delta.
//!
//! Calcule le throughput à partir des compteurs
//! rx_bytes / tx_bytes de l'interface sans fil.

use crate::core::*;
use std::time::Instant;

/// Calcule les débits réseau à partir des compteurs bytes.
pub struct NetworkThroughput {
    base: String,
    prev_rx: Option<u64>,
    prev_tx: Option<u64>,
    prev_time: Option<Instant>,
}

impl NetworkThroughput {
    /// `base` : répertoire `statistics` de l'interface
    /// (voir `paths::wifi_iface()`).
    pub fn new(base: String) -> Self {
        Self {
            base,
            prev_rx: None,
            prev_tx: None,
            prev_time: None,
        }
    }
}

impl Source for NetworkThroughput {
    fn read(&mut self) -> Result<Vec<Sample>> {
        let rx = read_counter(&format!("{}/rx_bytes", self.base));
        let tx = read_counter(&format!("{}/tx_bytes", self.base));
        let now = Instant::now();
        let mut samples = Vec::new();

        if let (Some(prev_rx), Some(prev_tx), Some(prev_time)) =
            (self.prev_rx, self.prev_tx, self.prev_time)
        {
            let dt = now.duration_since(prev_time).as_secs_f64();
            if dt > 0.0 {
                if let (Some(rx), Some(tx)) = (rx, tx) {
                    let rx_rate = (rx.saturating_sub(prev_rx)) as f64 / dt;
                    let tx_rate = (tx.saturating_sub(prev_tx)) as f64 / dt;

                    samples.push(Sample {
                        kind: "network".to_string(),
                        label: "RX Rate".to_string(),
                        value: Value::Num(rx_rate),
                        unit: Unit::KiBps,
                    });
                    samples.push(Sample {
                        kind: "network".to_string(),
                        label: "TX Rate".to_string(),
                        value: Value::Num(tx_rate),
                        unit: Unit::KiBps,
                    });
                }
            }
        }

        if let Some(rx) = rx {
            self.prev_rx = Some(rx);
        }
        if let Some(tx) = tx {
            self.prev_tx = Some(tx);
        }
        self.prev_time = Some(now);

        Ok(samples)
    }
}

fn read_counter(path: &str) -> Option<u64> {
    crate::core::read_trimmed(path).ok()?.parse().ok()
}
