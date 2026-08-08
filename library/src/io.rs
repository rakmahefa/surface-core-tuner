//! Source I/O : statistiques disque depuis /sys/block/*/stat.
//!
//! Calcule les IOPS et débits en lecture/écriture par delta.

use crate::core::*;
use crate::paths;
use std::time::Instant;

/// Champs du fichier stat pour un disque.
///
/// Format : read_ios read_merges read_sectors read_ticks
///          write_ios write_merges write_sectors write_ticks
///          in_flight io_ticks time_in_queue
struct DiskCounters {
    read_ios: u64,
    read_sectors: u64,
    write_ios: u64,
    write_sectors: u64,
}

/// Calcule les IOPS et débits par delta temporel.
pub struct DiskStats {
    name: String,
    path: String,
    prev: Option<DiskCounters>,
    prev_time: Option<Instant>,
}

impl DiskStats {
    /// `dev` : nom du disque (ex. "nvme0n1").
    pub fn new(dev: &str) -> Self {
        Self {
            name: dev.to_string(),
            path: format!("{}/{dev}/stat", paths::SYS_BLOCK),
            prev: None,
            prev_time: None,
        }
    }

    fn parse_stat(&self) -> Result<DiskCounters> {
        let content =
            std::fs::read_to_string(&self.path).map_err(|e| format!("{}: {}", self.path, e))?;
        let parts: Vec<u64> = content
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();

        Ok(DiskCounters {
            read_ios: parts.first().copied().unwrap_or(0),
            read_sectors: parts.get(2).copied().unwrap_or(0),
            write_ios: parts.get(4).copied().unwrap_or(0),
            write_sectors: parts.get(6).copied().unwrap_or(0),
        })
    }
}

impl Source for DiskStats {
    fn read(&mut self) -> Result<Vec<Sample>> {
        let cur = self.parse_stat()?;
        let now = Instant::now();
        let mut samples = Vec::new();

        let label = |suffix: &str| format!("{} {}", self.name, suffix);

        if let (Some(prev), Some(prev_time)) = (self.prev.as_ref(), self.prev_time) {
            let dt = now.duration_since(prev_time).as_secs_f64();
            if dt > 0.0 {
                let read_iops = (cur.read_ios.saturating_sub(prev.read_ios)) as f64 / dt;
                let write_iops = (cur.write_ios.saturating_sub(prev.write_ios)) as f64 / dt;
                let read_bps =
                    (cur.read_sectors.saturating_sub(prev.read_sectors)) as f64 * 512.0 / dt; // secteurs → bytes
                let write_bps =
                    (cur.write_sectors.saturating_sub(prev.write_sectors)) as f64 * 512.0 / dt;

                samples.push(Sample {
                    kind: "io".to_string(),
                    label: label("Read IOPS"),
                    value: Value::Num(read_iops),
                    unit: Unit::Count,
                });
                samples.push(Sample {
                    kind: "io".to_string(),
                    label: label("Write IOPS"),
                    value: Value::Num(write_iops),
                    unit: Unit::Count,
                });
                samples.push(Sample {
                    kind: "io".to_string(),
                    label: label("Read B/s"),
                    value: Value::Num(read_bps),
                    unit: Unit::Bytes,
                });
                samples.push(Sample {
                    kind: "io".to_string(),
                    label: label("Write B/s"),
                    value: Value::Num(write_bps),
                    unit: Unit::Bytes,
                });
            }
        }

        self.prev = Some(cur);
        self.prev_time = Some(now);

        // Toujours ajouter les compteurs cumulés
        samples.push(Sample {
            kind: "io".to_string(),
            label: label("Total Reads"),
            value: Value::Num(self.prev.as_ref().map(|p| p.read_ios as f64).unwrap_or(0.0)),
            unit: Unit::Count,
        });
        samples.push(Sample {
            kind: "io".to_string(),
            label: label("Total Writes"),
            value: Value::Num(
                self.prev
                    .as_ref()
                    .map(|p| p.write_ios as f64)
                    .unwrap_or(0.0),
            ),
            unit: Unit::Count,
        });

        Ok(samples)
    }
}
