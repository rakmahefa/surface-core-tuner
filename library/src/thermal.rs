//! Source thermal : zones de température (hwmon).
//!
//! Agrège les températures ACPI et NVMe.
//! (coretemp est dans `cpu.rs`, la puce WiFi dans `network` —
//! chaque capteur n'est lu qu'une fois par cycle.)

use crate::core::*;
use crate::paths;

/// Lit les zones thermiques hwmon.
///
/// Déclaratif : on définit les chemins et labels,
/// le struct les lit tous à chaque `read()`.
/// Un capteur absent est silencieusement ignoré.
pub struct ThermalZones {
    zones: Vec<Zone>,
}

struct Zone {
    path: String,
    label: String,
}

impl ThermalZones {
    pub fn new() -> Self {
        let mut zones = Vec::new();

        // ACPI (9 zones)
        if let Some(dir) = paths::hwmon("acpitz") {
            for i in 1..=9 {
                zones.push(Zone {
                    path: format!("{dir}/temp{i}_input"),
                    label: format!("ACPI Zone {i}"),
                });
            }
        }

        // NVMe (composite + 2 capteurs)
        if let Some(dir) = paths::hwmon("nvme") {
            zones.push(Zone {
                path: format!("{dir}/temp1_input"),
                label: "NVMe Composite".to_string(),
            });
            zones.push(Zone {
                path: format!("{dir}/temp2_input"),
                label: "NVMe Sensor 2".to_string(),
            });
            zones.push(Zone {
                path: format!("{dir}/temp3_input"),
                label: "NVMe Sensor 3".to_string(),
            });
        }

        Self { zones }
    }
}

impl Source for ThermalZones {
    fn read(&mut self) -> Result<Vec<Sample>> {
        let mut samples = Vec::new();
        for zone in &self.zones {
            if let Ok(raw) = std::fs::read_to_string(&zone.path) {
                if let Ok(temp) = raw.trim().parse::<f64>() {
                    samples.push(Sample {
                        kind: "thermal".to_string(),
                        label: zone.label.clone(),
                        value: Value::Num(temp * 0.001), // millidegrés → °C
                        unit: Unit::Celsius,
                    });
                }
            }
        }
        Ok(samples)
    }
}
