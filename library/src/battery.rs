//! Métriques batterie dérivées.
//!
//! Wear% = (1 - energy_full / energy_full_design) × 100.
//! Les énergies sont déjà lues par ScalarMetric : le dérivateur les
//! récupère du lot, sans relire les fichiers sysfs.

use crate::core::*;

/// Dérive l'usure de la batterie depuis les échantillons du lot.
pub fn derive_wear(samples: &[Sample]) -> Vec<Sample> {
    let full = find(samples, "Energy Full").and_then(|s| s.value.as_num());
    let design = find(samples, "Energy Design").and_then(|s| s.value.as_num());

    match (full, design) {
        (Some(full), Some(design)) if design > 0.0 => vec![Sample {
            kind: "battery".to_string(),
            label: "Wear".to_string(),
            value: Value::Num(((1.0 - full / design) * 100.0).clamp(0.0, 100.0)),
            unit: Unit::Percent,
        }],
        _ => vec![],
    }
}

fn find<'a>(samples: &'a [Sample], label: &str) -> Option<&'a Sample> {
    samples
        .iter()
        .find(|s| s.kind == "battery" && s.label == label)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(label: &str, value: f64) -> Sample {
        Sample {
            kind: "battery".to_string(),
            label: label.to_string(),
            value: Value::Num(value),
            unit: Unit::Wh,
        }
    }

    #[test]
    fn wear_derived_from_full_and_design() {
        // 22.25 Wh actuels / 43.2 Wh design → ~48.5 % d'usure
        let samples = vec![sample("Energy Full", 22.25), sample("Energy Design", 43.2)];
        let wear = derive_wear(&samples);
        assert_eq!(wear.len(), 1);
        let w = wear[0].value.as_num().unwrap();
        assert!((w - (1.0 - 22.25 / 43.2) * 100.0).abs() < 1e-9);
        assert_eq!(wear[0].unit, Unit::Percent);
    }

    #[test]
    fn wear_empty_when_energy_missing() {
        assert!(derive_wear(&[sample("Energy Full", 22.25)]).is_empty());
        assert!(derive_wear(&[]).is_empty());
    }
}
