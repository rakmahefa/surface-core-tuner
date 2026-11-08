//! Profils de performance — réglages de puissance prédéfinis ou personnalisés.
//!
//! Le module rassemble :
//! - les primitives d'écriture sysfs (gouverneur, fréquences CPU/GPU,
//!   globales ou par cœur), réutilisées par la GUI (sliders) et par
//!   l'application des profils ;
//! - les profils prédéfinis (`predefined()`) — caps uniformes exprimés en
//!   fraction de la limite HW — et les profils personnalisés (`load_custom`
//!   / `add_custom` / `remove_custom`) — snapshots complets par cœur —,
//!   persistés dans `$XDG_CONFIG_HOME/nyvoara/profiles.conf` ;
//! - la capture des réglages courants (`Profile::from_store`) pour
//!   « sauvegarder l'état actuel » depuis la GUI.

use crate::core::{read_f64, write_sysfs, ReadLatest, Result};
use crate::paths;
use std::path::PathBuf;

// ────────────────────────────────────────────
//  Primitives d'écriture (GUI + profils)
// ────────────────────────────────────────────

/// Écrit un gouverneur sur tous les cœurs.
pub fn set_cpu_governor(gov: &str) -> Result<()> {
    for cpu in 0..paths::CPU_THREADS {
        set_cpu_governor_core(cpu, gov)?;
    }
    Ok(())
}

/// Écrit le gouverneur d'un cœur précis.
pub fn set_cpu_governor_core(cpu: usize, gov: &str) -> Result<()> {
    write_sysfs(&format!("{}/scaling_governor", paths::cpufreq(cpu)), gov)
}

/// Écrit la fréquence min (MHz) sur tous les cœurs.
pub fn set_cpu_min_mhz(min_mhz: f64) -> Result<()> {
    set_cpu_freq("scaling_min_freq", min_mhz)
}

/// Écrit la fréquence max (MHz) sur tous les cœurs.
pub fn set_cpu_max_mhz(max_mhz: f64) -> Result<()> {
    set_cpu_freq("scaling_max_freq", max_mhz)
}

/// Écrit la fréquence min (MHz) d'un cœur précis.
pub fn set_cpu_min_mhz_core(cpu: usize, min_mhz: f64) -> Result<()> {
    set_cpu_freq_core(cpu, "scaling_min_freq", min_mhz)
}

/// Écrit la fréquence max (MHz) d'un cœur précis.
pub fn set_cpu_max_mhz_core(cpu: usize, max_mhz: f64) -> Result<()> {
    set_cpu_freq_core(cpu, "scaling_max_freq", max_mhz)
}

/// Écrit un fichier cpufreq (exprimé en kHz) sur tous les cœurs.
fn set_cpu_freq(file: &str, mhz: f64) -> Result<()> {
    for cpu in 0..paths::CPU_THREADS {
        set_cpu_freq_core(cpu, file, mhz)?;
    }
    Ok(())
}

/// Écrit un fichier cpufreq (exprimé en kHz) sur un cœur précis.
fn set_cpu_freq_core(cpu: usize, file: &str, mhz: f64) -> Result<()> {
    let khz = (mhz * 1000.0).round() as u64;
    write_sysfs(&format!("{}/{file}", paths::cpufreq(cpu)), &khz.to_string())
}

/// Écrit la fréquence min (MHz) du GPU.
pub fn set_gpu_min_mhz(min_mhz: f64) -> Result<()> {
    set_gpu_freq("gt_min_freq_mhz", min_mhz)
}

/// Écrit la fréquence max (MHz) du GPU.
pub fn set_gpu_max_mhz(max_mhz: f64) -> Result<()> {
    set_gpu_freq("gt_max_freq_mhz", max_mhz)
}

/// Écrit un fichier de fréquence du GPU (exprimé en MHz).
fn set_gpu_freq(file: &str, mhz: f64) -> Result<()> {
    let gpu = match paths::gpu_dir() {
        Some(g) => g,
        None => return Ok(()), // pas de GPU → silencieux
    };
    write_sysfs(&format!("{gpu}/{file}"), &(mhz.round() as u64).to_string())
}

// ────────────────────────────────────────────
//  Profils
// ────────────────────────────────────────────

/// Un profil : un ensemble de réglages nommé.
///
/// Deux formes complémentaires :
/// - **caps uniformes** (prédéfinis) : `governor`, `cpu_max`, `gpu_max` —
///   un seul réglage appliqué à tous les cœurs ;
/// - **snapshot par cœur** (personnalisés) : `core_governor`, `core_min`,
///   `core_max` — valeurs exactes de chaque cœur, + `gpu_min`.
///
/// Chaque champ `Option` : `None` = paramètre non touché.
#[derive(Clone, Debug, PartialEq)]
pub struct Profile {
    pub name: String,
    /// Gouverneur CPU global (tous cœurs identiques).
    pub governor: Option<String>,
    /// Gouverneurs par cœur (si différenciés) — supplante `governor`.
    pub core_governor: Option<Vec<String>>,
    /// Plafond CPU max uniforme — fraction de la limite HW (0.0–1.0).
    pub cpu_max: Option<f64>,
    /// Fréquences min par cœur (MHz) — snapshot personnalisé.
    pub core_min: Option<Vec<f64>>,
    /// Fréquences max par cœur (MHz) — snapshot personnalisé.
    pub core_max: Option<Vec<f64>>,
    /// Fréquence min GPU (MHz).
    pub gpu_min: Option<f64>,
    /// Plafond GPU max — fraction de la limite HW (0.0–1.0).
    pub gpu_max: Option<f64>,
}

impl Profile {
    /// Construit un profil « cap uniforme » (prédéfini).
    pub fn new(
        name: impl Into<String>,
        governor: Option<&str>,
        cpu_max: Option<f64>,
        gpu_max: Option<f64>,
    ) -> Self {
        Self {
            name: name.into(),
            governor: governor.map(String::from),
            core_governor: None,
            cpu_max,
            core_min: None,
            core_max: None,
            gpu_min: None,
            gpu_max,
        }
    }

    /// Applique le profil (écritures sysfs immédiates).
    pub fn apply(&self) -> Result<()> {
        if let Some(gov) = &self.governor {
            set_cpu_governor(gov)?;
        }
        if let Some(govs) = &self.core_governor {
            for (i, g) in govs.iter().enumerate() {
                set_cpu_governor_core(i, g)?;
            }
        }

        if let Some(ratio) = self.cpu_max {
            let cpu0 = paths::cpufreq(0);
            let hw_min = read_f64(&format!("{cpu0}/cpuinfo_min_freq"))? * 0.001;
            let hw_max = read_f64(&format!("{cpu0}/cpuinfo_max_freq"))? * 0.001;
            let target = target_from_ratio(ratio, hw_min, hw_max);

            // Si le min actuel dépasse la cible, le kernel plafonnerait la
            // max à ce min (l'effet serait annulé) : on réinitialise d'abord.
            let cur_min = read_f64(&format!("{cpu0}/scaling_min_freq"))? * 0.001;
            if target < cur_min {
                set_cpu_min_mhz(hw_min)?;
            }
            set_cpu_max_mhz(target)?;
        }

        apply_core_cpu(self)?;

        // GPU : ordre max → min → max — le kernel clampe min/max l'un sur
        // l'autre ; cet ordre restaure le couple même si le min courant
        // dépasse la cible.
        if self.gpu_max.is_some() || self.gpu_min.is_some() {
            if let Some(gpu) = paths::gpu_dir() {
                let max_t = match self.gpu_max {
                    Some(ratio) => {
                        let rp0 = read_f64(&format!("{gpu}/gt_RP0_freq_mhz"))?;
                        let rp1 = read_f64(&format!("{gpu}/gt_RP1_freq_mhz"))?;
                        Some(target_from_ratio(ratio, rp1, rp0))
                    }
                    None => None,
                };
                if let Some(t) = max_t {
                    set_gpu_max_mhz(t)?;
                }
                if let Some(m) = self.gpu_min {
                    set_gpu_min_mhz(m.min(max_t.unwrap_or(m)))?;
                }
                if let Some(t) = max_t {
                    set_gpu_max_mhz(t)?;
                }
            }
        }

        Ok(())
    }

    /// Capture les réglages actuels depuis le Store : gouverneurs et
    /// fréquences min/max de **chaque cœur** + GPU. `None` si aucune
    /// lecture exploitable n'est disponible.
    pub fn from_store<S: ReadLatest>(store: &S, name: &str) -> Option<Profile> {
        let governors: Option<Vec<String>> = (0..paths::CPU_THREADS)
            .map(|i| store.text("cpu", &format!("CPU{i} Governor")))
            .collect();
        let mins: Option<Vec<f64>> = (0..paths::CPU_THREADS)
            .map(|i| store.num("cpu", &format!("CPU{i} Min")))
            .collect();
        let maxes: Option<Vec<f64>> = (0..paths::CPU_THREADS)
            .map(|i| store.num("cpu", &format!("CPU{i} Max")))
            .collect();
        let gpu_min = store.num("gpu", "Min Freq");
        let gpu_max = ratio(store.num("gpu", "Max Freq"), store.num("gpu", "RP0 Max"));

        if governors.is_none()
            && mins.is_none()
            && maxes.is_none()
            && gpu_min.is_none()
            && gpu_max.is_none()
        {
            return None; // rien à capturer
        }

        // Gouverneurs uniformes → champ global ; différenciés → par cœur.
        let (governor, core_governor) = match &governors {
            Some(v) if v.iter().all(|g| g == &v[0]) => (Some(v[0].clone()), None),
            Some(v) => (None, Some(v.clone())),
            None => (None, None),
        };

        Some(Profile {
            name: name.to_string(),
            governor,
            core_governor,
            cpu_max: None,
            core_min: mins,
            core_max: maxes,
            gpu_min,
            gpu_max,
        })
    }

    /// Vrai si les lectures actuelles correspondent au profil
    /// (gouverneurs identiques, fréquences à ±150 MHz de la cible).
    pub fn is_active<S: ReadLatest>(&self, store: &S) -> bool {
        const TOL_MHZ: f64 = 150.0;

        if let Some(gov) = &self.governor {
            if store.text("cpu", "CPU0 Governor").as_deref() != Some(gov.as_str()) {
                return false;
            }
        }
        if let Some(govs) = &self.core_governor {
            for (i, g) in govs.iter().enumerate() {
                if store.text("cpu", &format!("CPU{i} Governor")).as_deref() != Some(g.as_str()) {
                    return false;
                }
            }
        }
        if let Some(ratio) = self.cpu_max {
            let Some(hw_max) = store.num("cpu", "HW Max Freq") else {
                return false;
            };
            let target = hw_max * ratio;
            match store.num("cpu", "CPU0 Max") {
                Some(cur) if (cur - target).abs() <= TOL_MHZ => {}
                _ => return false,
            }
        }
        if let Some(mins) = &self.core_min {
            for (i, m) in mins.iter().enumerate() {
                match store.num("cpu", &format!("CPU{i} Min")) {
                    Some(cur) if (cur - m).abs() <= TOL_MHZ => {}
                    _ => return false,
                }
            }
        }
        if let Some(maxes) = &self.core_max {
            for (i, m) in maxes.iter().enumerate() {
                match store.num("cpu", &format!("CPU{i} Max")) {
                    Some(cur) if (cur - m).abs() <= TOL_MHZ => {}
                    _ => return false,
                }
            }
        }
        if let Some(m) = self.gpu_min {
            match store.num("gpu", "Min Freq") {
                Some(cur) if (cur - m).abs() <= TOL_MHZ => {}
                _ => return false,
            }
        }
        if let Some(ratio) = self.gpu_max {
            let Some(rp0) = store.num("gpu", "RP0 Max") else {
                return false;
            };
            let target = rp0 * ratio;
            match store.num("gpu", "Max Freq") {
                Some(cur) if (cur - target).abs() <= TOL_MHZ => {}
                _ => return false,
            }
        }
        true
    }
}

/// Applique les fréquences par cœur (snapshot personnalisé).
///
/// Ordre max → min → max par cœur : le kernel clampe min et max l'un sur
/// l'autre, cet ordre restaure le couple exact même si le min courant
/// dépasse la cible (sinon la max serait silencieusement bornée au min).
fn apply_core_cpu(p: &Profile) -> Result<()> {
    let n = paths::CPU_THREADS;
    if p.core_min.as_ref().is_some_and(|v| v.len() != n)
        || p.core_max.as_ref().is_some_and(|v| v.len() != n)
    {
        return Err("core_min/core_max must have one value per CPU".into());
    }

    match (&p.core_min, &p.core_max) {
        (Some(mins), Some(maxes)) => {
            for i in 0..n {
                set_cpu_max_mhz_core(i, maxes[i])?;
                set_cpu_min_mhz_core(i, mins[i])?;
                set_cpu_max_mhz_core(i, maxes[i])?;
            }
        }
        (Some(mins), None) => {
            for (i, m) in mins.iter().enumerate() {
                set_cpu_min_mhz_core(i, *m)?;
            }
        }
        (None, Some(maxes)) => {
            for (i, m) in maxes.iter().enumerate() {
                set_cpu_max_mhz_core(i, *m)?;
            }
        }
        (None, None) => {}
    }
    Ok(())
}

/// Ratio valeur / limite HW, borné à [0, 1].
fn ratio(cur: Option<f64>, hw: Option<f64>) -> Option<f64> {
    match (cur, hw) {
        (Some(c), Some(h)) if h > 0.0 => Some((c / h).clamp(0.0, 1.0)),
        _ => None,
    }
}

/// Fréquence cible (MHz) pour un ratio de la limite HW max,
/// bornée par les limites matérielles.
fn target_from_ratio(ratio: f64, hw_min: f64, hw_max: f64) -> f64 {
    (hw_max * ratio.clamp(0.0, 1.0)).max(hw_min)
}

// ────────────────────────────────────────────
//  Listes de profils
// ────────────────────────────────────────────

/// Profils prédéfinis, du plus économe au plus performant.
pub fn predefined() -> Vec<Profile> {
    vec![
        Profile::new("Battery Saver", Some("powersave"), Some(0.30), Some(0.25)),
        Profile::new("Balanced", Some("powersave"), Some(1.0), Some(1.0)),
        Profile::new("Performance", Some("performance"), Some(1.0), Some(1.0)),
    ]
}

/// Tous les profils : prédéfinis d'abord, puis personnalisés (ordre garanti).
pub fn load_all() -> Vec<Profile> {
    let mut all = predefined();
    all.extend(load_custom());
    all
}

/// Sépare profils prédéfinis et personnalisés (prédéfinis d'abord).
pub fn split(profiles: &[Profile]) -> (Vec<&Profile>, Vec<&Profile>) {
    let n = predefined().len();
    (
        profiles.iter().take(n).collect(),
        profiles.iter().skip(n).collect(),
    )
}

/// Retrouve un profil par nom (insensible à la casse).
pub fn find<'a>(profiles: &'a [Profile], name: &str) -> Option<&'a Profile> {
    profiles.iter().find(|p| p.name.eq_ignore_ascii_case(name))
}

/// Noms des profils, dans l'ordre.
pub fn names(profiles: &[Profile]) -> Vec<&str> {
    profiles.iter().map(|p| p.name.as_str()).collect()
}

// ────────────────────────────────────────────
//  Persistance des profils personnalisés
// ────────────────────────────────────────────

/// Fichier de persistance des profils personnalisés.
fn profiles_path() -> PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            std::env::var("HOME")
                .map(|h| PathBuf::from(h).join(".config"))
                .unwrap_or_else(|_| PathBuf::from("."))
        });
    base.join("nyvoara").join("profiles.conf")
}

/// Charge les profils personnalisés depuis le disque.
/// Fichier absent ou partiellement corrompu → liste des blocs valides
/// (jamais d'erreur fatale).
pub fn load_custom() -> Vec<Profile> {
    let path = profiles_path();
    match std::fs::read_to_string(&path) {
        Ok(content) => parse_profiles(&content),
        Err(_) => Vec::new(),
    }
}

/// Sauvegarde les profils personnalisés.
///
/// Écriture atomique (fichier tmp + rename) : une interruption en cours
/// d'écriture ne corrompt jamais le fichier existant.
pub fn save_custom(profiles: &[Profile]) -> Result<()> {
    let path = profiles_path();
    if let Some(dir) = path.parent() {
        if !dir.as_os_str().is_empty() {
            std::fs::create_dir_all(dir).map_err(|e| format!("create {}: {}", dir.display(), e))?;
        }
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, format_profiles(profiles))
        .map_err(|e| format!("write {}: {}", tmp.display(), e))?;
    std::fs::rename(&tmp, &path).map_err(|e| format!("rename {}: {}", path.display(), e))?;
    Ok(())
}

/// Ajoute ou remplace un profil personnalisé (même nom, insensible à la casse).
pub fn add_custom(profile: &Profile) -> Result<Vec<Profile>> {
    let mut custom = load_custom();
    custom.retain(|p| !p.name.eq_ignore_ascii_case(&profile.name));
    custom.push(profile.clone());
    save_custom(&custom)?;
    Ok(custom)
}

/// Supprime un profil personnalisé par nom (insensible à la casse).
pub fn remove_custom(name: &str) -> Result<Vec<Profile>> {
    let mut custom = load_custom();
    custom.retain(|p| !p.name.eq_ignore_ascii_case(name));
    save_custom(&custom)?;
    Ok(custom)
}

// ────────────────────────────────────────────
//  Format texte (clé = valeur)
// ────────────────────────────────────────────

/// Parse le format texte des profils personnalisés.
///
/// Blocs délimités par `[profile]`, lignes `clé = valeur`, commentaires
/// `#` ignorés. Les blocs sans nom et les lignes invalides sont ignorés.
/// Les tableaux par cœur (`core_min`, `core_max`, `core_governor`) doivent
/// contenir exactement `CPU_THREADS` valeurs (séparées par des virgules),
/// sinon le champ est ignoré.
fn parse_profiles(content: &str) -> Vec<Profile> {
    let mut profiles = Vec::new();
    let mut draft: Option<Profile> = None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line == "[profile]" {
            if let Some(p) = draft.take() {
                profiles.push(p);
            }
            draft = Some(Profile::new(String::new(), None, None, None));
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let Some(p) = draft.as_mut() else {
            continue; // valeur hors bloc → ignorée
        };
        match (key.trim(), value.trim()) {
            ("name", v) => p.name = v.to_string(),
            ("governor", v) => p.governor = Some(v.to_string()),
            ("core_governor", v) => p.core_governor = parse_vec_str(v),
            ("cpu_max", v) => p.cpu_max = v.parse().ok(),
            ("core_min", v) => p.core_min = parse_vec_f64(v),
            ("core_max", v) => p.core_max = parse_vec_f64(v),
            ("gpu_min", v) => p.gpu_min = v.parse().ok(),
            ("gpu_max", v) => p.gpu_max = v.parse().ok(),
            _ => {}
        }
    }
    if let Some(p) = draft {
        profiles.push(p);
    }
    profiles.retain(|p| !p.name.is_empty());
    profiles
}

/// Sérialise les profils personnalisés au format texte.
fn format_profiles(profiles: &[Profile]) -> String {
    let mut out = String::from("# NyVoara custom profiles\n");
    for p in profiles {
        out.push_str("[profile]\n");
        out.push_str(&format!("name = {}\n", p.name));
        if let Some(g) = &p.governor {
            out.push_str(&format!("governor = {g}\n"));
        }
        if let Some(v) = &p.core_governor {
            out.push_str(&format!("core_governor = {}\n", v.join(",")));
        }
        if let Some(v) = p.cpu_max {
            out.push_str(&format!("cpu_max = {v}\n"));
        }
        if let Some(v) = &p.core_min {
            out.push_str(&format!("core_min = {}\n", join_f64(v)));
        }
        if let Some(v) = &p.core_max {
            out.push_str(&format!("core_max = {}\n", join_f64(v)));
        }
        if let Some(v) = p.gpu_min {
            out.push_str(&format!("gpu_min = {v}\n"));
        }
        if let Some(v) = p.gpu_max {
            out.push_str(&format!("gpu_max = {v}\n"));
        }
        out.push('\n');
    }
    out
}

/// Parse un tableau f64 (séparé par virgules) de taille exacte.
fn parse_vec_f64(s: &str) -> Option<Vec<f64>> {
    let v: Vec<f64> = s.split(',').filter_map(|x| x.trim().parse().ok()).collect();
    (v.len() == paths::CPU_THREADS).then_some(v)
}

/// Parse un tableau de chaînes (séparé par virgules) de taille exacte.
fn parse_vec_str(s: &str) -> Option<Vec<String>> {
    let v: Vec<String> = s.split(',').map(|x| x.trim().to_string()).collect();
    (v.len() == paths::CPU_THREADS).then_some(v)
}

/// Formate un tableau f64 en liste séparée par virgules.
fn join_f64(v: &[f64]) -> String {
    v.iter()
        .map(|x| x.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Sample, Store, Unit, Value};

    fn sample(kind: &str, label: &str, value: Value) -> Sample {
        Sample {
            kind: kind.to_string(),
            label: label.to_string(),
            value,
            unit: Unit::Raw,
        }
    }

    /// Store simulé dans l'état par défaut (limites HW, pas de plafond).
    fn default_store() -> Store {
        let mut store = Store::new(10);
        store.push_batch(vec![
            sample("cpu", "CPU0 Governor", Value::Text("powersave".to_string())),
            sample("cpu", "HW Max Freq", Value::Num(3700.0)),
            sample("cpu", "CPU0 Max", Value::Num(3700.0)),
            sample("gpu", "RP0 Max", Value::Num(1050.0)),
            sample("gpu", "Max Freq", Value::Num(1050.0)),
        ]);
        store
    }

    #[test]
    fn target_is_ratio_of_hw_max() {
        assert_eq!(target_from_ratio(0.5, 400.0, 3700.0), 1850.0);
        assert_eq!(target_from_ratio(1.0, 400.0, 3700.0), 3700.0);
    }

    #[test]
    fn target_clamped_to_hw_range() {
        assert_eq!(target_from_ratio(0.0, 400.0, 3700.0), 400.0);
        assert_eq!(target_from_ratio(2.0, 400.0, 3700.0), 3700.0);
    }

    #[test]
    fn balanced_active_on_default_state() {
        let store = default_store();
        let profiles = predefined();
        assert!(profiles[1].is_active(&store)); // Balanced
        assert!(!profiles[0].is_active(&store)); // Battery Saver : plafond absent
        assert!(!profiles[2].is_active(&store)); // Performance : gouverneur différent
    }

    #[test]
    fn battery_saver_active_when_capped() {
        let mut store = default_store();
        store.push_batch(vec![
            sample("cpu", "CPU0 Max", Value::Num(3700.0 * 0.30)),
            sample("gpu", "Max Freq", Value::Num(1050.0 * 0.25)),
        ]);
        let profiles = predefined();
        assert!(profiles[0].is_active(&store));
        assert!(!profiles[1].is_active(&store));
    }

    #[test]
    fn tolerance_ignores_small_freq_drift() {
        let mut store = default_store();
        store.push_batch(vec![
            sample("cpu", "CPU0 Max", Value::Num(1110.0 + 50.0)),
            sample("gpu", "Max Freq", Value::Num(1050.0 * 0.25)),
        ]);
        assert!(predefined()[0].is_active(&store));
    }

    #[test]
    fn find_matches_case_insensitively() {
        let profiles = predefined();
        assert!(find(&profiles, "balanced").is_some());
        assert!(find(&profiles, "PERFORMANCE").is_some());
        assert!(find(&profiles, "nope").is_none());
    }

    #[test]
    fn parse_profiles_reads_blocks() {
        let content = "\
# comment
[profile]
name = Gaming
governor = performance
cpu_max = 0.9
gpu_max = 0.8

[profile]
name = Cinema
governor = powersave

garbage line
";
        let profiles = parse_profiles(content);
        assert_eq!(profiles.len(), 2);
        assert_eq!(profiles[0].name, "Gaming");
        assert_eq!(profiles[0].governor.as_deref(), Some("performance"));
        assert_eq!(profiles[0].cpu_max, Some(0.9));
        assert_eq!(profiles[0].gpu_max, Some(0.8));
        assert_eq!(profiles[1].name, "Cinema");
        assert_eq!(profiles[1].governor.as_deref(), Some("powersave"));
        assert_eq!(profiles[1].cpu_max, None);
    }

    #[test]
    fn parse_profiles_reads_per_core_fields() {
        let content = "\
[profile]
name = Mine
core_governor = powersave,powersave,powersave,powersave,performance,performance,performance,performance
core_min = 400,400,400,400,400,400,400,400
core_max = 2000,2000,2000,2000,2000,2000,2000,2000
gpu_min = 300
";
        let profiles = parse_profiles(content);
        assert_eq!(profiles.len(), 1);
        let p = &profiles[0];
        let govs = p.core_governor.as_ref().unwrap();
        assert_eq!(govs.len(), 8);
        assert_eq!(govs[0], "powersave");
        assert_eq!(govs[4], "performance");
        assert_eq!(p.core_min.as_ref().unwrap()[0], 400.0);
        assert_eq!(p.core_max.as_ref().unwrap()[7], 2000.0);
        assert_eq!(p.gpu_min, Some(300.0));
    }

    #[test]
    fn parse_rejects_wrong_length_arrays() {
        let content = "[profile]\nname = Bad\ncore_max = 2000,2000,2000\n";
        let profiles = parse_profiles(content);
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].core_max, None);
    }

    #[test]
    fn parse_drops_unnamed_blocks() {
        let content = "[profile]\ngovernor = powersave\n";
        assert!(parse_profiles(content).is_empty());
    }

    #[test]
    fn format_parse_round_trip() {
        let profiles = vec![
            Profile::new("Gaming", Some("performance"), Some(0.9), Some(0.8)),
            Profile {
                name: "Mine".into(),
                governor: None,
                core_governor: Some(vec!["powersave".into(); 8]),
                cpu_max: None,
                core_min: Some(vec![400.0; 8]),
                core_max: Some(vec![2000.0; 8]),
                gpu_min: Some(300.0),
                gpu_max: Some(0.25),
            },
        ];
        let parsed = parse_profiles(&format_profiles(&profiles));
        assert_eq!(parsed.len(), 2);
        // Gaming — caps uniformes (rétro-compatible)
        assert_eq!(parsed[0].name, "Gaming");
        assert_eq!(parsed[0].governor.as_deref(), Some("performance"));
        assert_eq!(parsed[0].cpu_max, Some(0.9));
        assert_eq!(parsed[0].gpu_max, Some(0.8));
        // Mine — snapshot par cœur
        assert_eq!(parsed[1].name, "Mine");
        assert_eq!(parsed[1].core_governor, Some(vec!["powersave".into(); 8]));
        assert_eq!(parsed[1].core_min, Some(vec![400.0; 8]));
        assert_eq!(parsed[1].core_max, Some(vec![2000.0; 8]));
        assert_eq!(parsed[1].gpu_min, Some(300.0));
        assert_eq!(parsed[1].gpu_max, Some(0.25));
    }

    #[test]
    fn from_store_captures_current_settings() {
        let mut store = Store::new(10);
        let mut samples = vec![
            sample("gpu", "RP0 Max", Value::Num(1050.0)),
            sample("gpu", "Max Freq", Value::Num(262.5)),
        ];
        for i in 0..8 {
            samples.push(sample(
                "cpu",
                &format!("CPU{i} Governor"),
                Value::Text("powersave".to_string()),
            ));
            samples.push(sample("cpu", &format!("CPU{i} Min"), Value::Num(400.0)));
            samples.push(sample("cpu", &format!("CPU{i} Max"), Value::Num(1110.0)));
        }
        store.push_batch(samples);
        let p = Profile::from_store(&store, "Mine").unwrap();
        assert_eq!(p.name, "Mine");
        assert_eq!(p.governor.as_deref(), Some("powersave")); // uniforme → global
        assert_eq!(p.core_governor, None);
        assert_eq!(p.core_min, Some(vec![400.0; 8]));
        assert_eq!(p.core_max, Some(vec![1110.0; 8]));
        assert_eq!(p.gpu_min, None);
        assert!((p.gpu_max.unwrap() - 0.25).abs() < 1e-9);
    }

    #[test]
    fn from_store_captures_per_core_settings() {
        let mut store = Store::new(10);
        let mut samples = Vec::new();
        for i in 0..8 {
            let gov = if i < 4 { "powersave" } else { "performance" };
            samples.push(sample(
                "cpu",
                &format!("CPU{i} Governor"),
                Value::Text(gov.to_string()),
            ));
            samples.push(sample(
                "cpu",
                &format!("CPU{i} Min"),
                Value::Num(400.0 + i as f64 * 100.0),
            ));
            samples.push(sample(
                "cpu",
                &format!("CPU{i} Max"),
                Value::Num(3700.0 - i as f64 * 100.0),
            ));
        }
        store.push_batch(samples);
        let p = Profile::from_store(&store, "Mine").unwrap();
        assert_eq!(p.governor, None); // différencié → par cœur
        let govs = p.core_governor.unwrap();
        assert_eq!(govs[0], "powersave");
        assert_eq!(govs[7], "performance");
        assert_eq!(p.core_min.unwrap()[7], 1100.0);
        assert_eq!(p.core_max.unwrap()[7], 3000.0);
        assert_eq!(p.cpu_max, None);
    }

    #[test]
    fn from_store_none_without_readings() {
        let store = Store::new(10);
        assert!(Profile::from_store(&store, "X").is_none());
    }

    #[test]
    fn is_active_detects_per_core_profiles() {
        let mut store = Store::new(10);
        for i in 0..8 {
            store.push(sample(
                "cpu",
                &format!("CPU{i} Governor"),
                Value::Text("powersave".to_string()),
            ));
            store.push(sample("cpu", &format!("CPU{i} Min"), Value::Num(400.0)));
            store.push(sample("cpu", &format!("CPU{i} Max"), Value::Num(2000.0)));
        }
        let p = Profile {
            name: "Mine".into(),
            governor: Some("powersave".into()),
            core_governor: None,
            cpu_max: None,
            core_min: Some(vec![400.0; 8]),
            core_max: Some(vec![2000.0; 8]),
            gpu_min: None,
            gpu_max: None,
        };
        assert!(p.is_active(&store));
        // Un seul cœur déviant → profil inactif
        store.push(sample("cpu", "CPU3 Max", Value::Num(1800.0)));
        assert!(!p.is_active(&store));
    }
}
