//! Bibliothèque interne — types fondamentaux, traits, Sampler, Store.
//! Réutilisable pour tout projet de monitoring système.
//!
//! # Organisation
//!
//! | Type | Rôle |
//! |---|---|
//! | `Value` | Num(f64) ou Text(String) |
//! | `Unit` | Unité d'affichage avec formatage |
//! | `Sample` | Une mesure datée |
//! | `Source` | Trait : lit et produit des `Sample` |
//! | `Sampler` | Orchestre une collection de sources |
//! | `Store` | Stocke latest + historique (ring buffer) |
//! | `ReadLatest` | Trait de lecture des dernières valeurs (Store, View) |
//! | `View` | Snapshot de rendu — copie de `latest` sous lock bref |

use std::collections::{HashMap, VecDeque};
use std::time::Instant;

// ────────────────────────────────────────────
//  Value
// ────────────────────────────────────────────

/// Une valeur mesurée : numérique ou textuelle.
/// Toute source produit une `Value` — stockage uniforme
/// quelle que soit la métrique.
#[derive(Clone, Debug)]
pub enum Value {
    Num(f64),
    Text(String),
}

impl Value {
    pub fn as_num(&self) -> Option<f64> {
        match self {
            Value::Num(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            Value::Text(v) => Some(v),
            _ => None,
        }
    }
}

// ────────────────────────────────────────────
//  Unit
// ────────────────────────────────────────────

/// Unité de mesure, avec formatage automatique
/// (échelles, suffixes, précision).
#[derive(Clone, Debug, PartialEq)]
pub enum Unit {
    Raw,
    Celsius,
    Percent,
    Mhz,
    W,
    V,
    Wh,
    Cycles,
    Bytes,
    KiBps,
    Ms,
    Count,
}

impl Unit {
    pub fn suffix(&self) -> &str {
        match self {
            Unit::Raw => "",
            Unit::Celsius => "°C",
            Unit::Percent => "%",
            Unit::Mhz => "MHz",
            Unit::W => "W",
            Unit::V => "V",
            Unit::Wh => "Wh",
            Unit::Cycles => " cycles",
            Unit::Bytes => "B",
            Unit::KiBps => "KiB/s",
            Unit::Ms => "ms",
            Unit::Count => "",
        }
    }

    /// Formate une valeur f64 avec la bonne unité et échelle.
    pub fn fmt(&self, value: f64) -> String {
        match self {
            // Auto-scale fréquences
            Unit::Mhz if value >= 1000.0 => format!("{:.2} GHz", value / 1000.0),
            // Auto-scale bytes
            Unit::Bytes if value >= 1024.0 * 1024.0 * 1024.0 => {
                format!("{:.2} GiB", value / (1024.0 * 1024.0 * 1024.0))
            }
            Unit::Bytes if value >= 1024.0 * 1024.0 => {
                format!("{:.1} MiB", value / (1024.0 * 1024.0))
            }
            Unit::Bytes if value >= 1024.0 => {
                format!("{:.1} KiB", value / 1024.0)
            }
            Unit::KiBps if value >= 1024.0 => {
                format!("{:.1} MiB/s", value / 1024.0)
            }
            // Milliseconds auto-scale (min avant s : ordre des guards)
            Unit::Ms if value >= 60000.0 => format!("{:.2} min", value / 60000.0),
            Unit::Ms if value >= 1000.0 => format!("{:.2} s", value / 1000.0),
            // Auto-scale Wh
            Unit::Wh if value >= 1000.0 => format!("{:.1} kWh", value / 1000.0),
            // Précision adaptative
            _ if value.abs() < 0.01 => format!("{:.4}{}", value, self.suffix()),
            _ if value.abs() < 1.0 => format!("{:.3}{}", value, self.suffix()),
            _ if value.abs() < 100.0 => format!("{:.2}{}", value, self.suffix()),
            _ => format!("{:.1}{}", value, self.suffix()),
        }
    }
}

// ────────────────────────────────────────────
//  Sample
// ────────────────────────────────────────────

/// Une mesure ponctuelle.
#[derive(Clone, Debug)]
pub struct Sample {
    pub kind: String,
    pub label: String,
    pub value: Value,
    pub unit: Unit,
}

impl Sample {
    pub fn key(&self) -> String {
        format!("{}/{}", self.kind, self.label)
    }

    /// Valeur affichable.
    pub fn display(&self) -> String {
        match &self.value {
            Value::Num(v) => self.unit.fmt(*v),
            Value::Text(t) => t.clone(),
        }
    }
}

// ────────────────────────────────────────────
//  Source trait
// ────────────────────────────────────────────

/// Résultat simplifié pour le MVP.
pub type Result<T> = std::result::Result<T, String>;

/// Une source de données produit des échantillons.
pub trait Source: Send {
    /// Interroge la source et retourne ses échantillons.
    fn read(&mut self) -> Result<Vec<Sample>>;
}

// ────────────────────────────────────────────
//  Sampler
// ────────────────────────────────────────────

/// Post-traitement : produit des métriques dérivées à partir du lot courant.
///
/// Permet de calculer des métriques à partir d'autres métriques du même
/// lot (ex. usure batterie) sans relire les fichiers sources.
pub type Deriver = fn(&[Sample]) -> Vec<Sample>;

/// Orchestre une collection de sources puis applique les dérivateurs.
/// Appelle `read()` sur chaque source et collecte
/// tous les échantillons produits.
pub struct Sampler {
    sources: Vec<Box<dyn Source>>,
    derivers: Vec<Deriver>,
}

impl Sampler {
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
            derivers: Vec::new(),
        }
    }

    pub fn add(&mut self, source: Box<dyn Source>) {
        self.sources.push(source);
    }

    /// Enregistre un dérivateur : reçoit le lot courant et renvoie
    /// des échantillons supplémentaires.
    pub fn derive(&mut self, deriver: Deriver) {
        self.derivers.push(deriver);
    }

    /// Interroge toutes les sources puis applique les dérivateurs.
    /// Les erreurs individuelles sont ignorées (un fichier
    /// sysfs peut ne pas exister sur certaines configs).
    pub fn poll_all(&mut self) -> Vec<Sample> {
        let mut all = Vec::new();
        for source in &mut self.sources {
            if let Ok(samples) = source.read() {
                all.extend(samples);
            }
        }
        for deriver in &self.derivers {
            all.extend(deriver(&all));
        }
        all
    }
}

// ────────────────────────────────────────────
//  Store
// ────────────────────────────────────────────

/// Stockage thread-safe des échantillons.
///
/// - `latest` : HashMap dernière valeur connue par clé
/// - `history` : Ring buffer temporel pour les valeurs numériques
#[derive(Clone)]
pub struct Store {
    pub latest: HashMap<String, Sample>,
    pub history: HashMap<String, VecDeque<(Instant, f64)>>,
    max_len: usize,
}

impl Store {
    pub fn new(max_history: usize) -> Self {
        Self {
            latest: HashMap::new(),
            history: HashMap::new(),
            max_len: max_history,
        }
    }

    /// Insère un échantillon.
    pub fn push(&mut self, sample: Sample) {
        let key = sample.key();
        self.latest.insert(key.clone(), sample.clone());

        // Historique uniquement pour les valeurs numériques
        if let Value::Num(v) = sample.value {
            let entry = self
                .history
                .entry(key)
                .or_insert_with(|| VecDeque::with_capacity(self.max_len));
            entry.push_back((Instant::now(), v));
            while entry.len() > self.max_len {
                entry.pop_front();
            }
        }
    }

    /// Insère un lot d'échantillons.
    pub fn push_batch(&mut self, samples: Vec<Sample>) {
        for s in samples {
            self.push(s);
        }
    }

    /// Historique des valeurs numériques.
    pub fn history(&self, kind: &str, label: &str) -> Option<&VecDeque<(Instant, f64)>> {
        let key = format!("{}/{}", kind, label);
        self.history.get(&key)
    }
}

// ────────────────────────────────────────────
//  ReadLatest — lecture des dernières valeurs
// ────────────────────────────────────────────

/// Accès en lecture aux dernières valeurs.
///
/// Implémenté par `Store` (source de vérité) et `View` (snapshot de
/// rendu) : les consommateurs (GUI, profils) sont agnostiques du
/// support, et les lectures ne sont jamais dupliquées.
pub trait ReadLatest {
    /// Dernières valeurs par clé `"kind/label"`.
    fn latest(&self) -> &HashMap<String, Sample>;

    /// Dernier échantillon d'une métrique.
    fn get(&self, kind: &str, label: &str) -> Option<&Sample> {
        self.latest().get(&format!("{}/{}", kind, label))
    }

    /// Dernière valeur numérique d'une métrique.
    fn num(&self, kind: &str, label: &str) -> Option<f64> {
        self.get(kind, label).and_then(|s| s.value.as_num())
    }

    /// Dernière valeur textuelle d'une métrique.
    fn text(&self, kind: &str, label: &str) -> Option<String> {
        self.get(kind, label)
            .and_then(|s| s.value.as_text())
            .map(|s| s.to_string())
    }

    /// Tous les échantillons d'une catégorie.
    fn by_kind(&self, kind: &str) -> Vec<&Sample> {
        self.latest().values().filter(|s| s.kind == kind).collect()
    }
}

impl ReadLatest for Store {
    fn latest(&self) -> &HashMap<String, Sample> {
        &self.latest
    }
}

// ────────────────────────────────────────────
//  View — snapshot de rendu
// ────────────────────────────────────────────

/// Copie légère des dernières valeurs, prise sous un lock bref.
///
/// L'UI rend à partir d'un `View` — jamais du `Store` verrouillé : le
/// Mutex partagé n'est pas tenu pendant le rendu, et le thread de
/// polling n'est jamais bloqué par une frame lente. L'historique reste
/// dans le `Store`, lu uniquement via des locks brefs (graphes).
pub struct View {
    latest: HashMap<String, Sample>,
}

impl View {
    /// Clône uniquement `latest` (quelques dizaines d'entrées) —
    /// l'historique n'est pas copié (ring buffers volumineux).
    pub fn snapshot(store: &Store) -> Self {
        Self {
            latest: store.latest.clone(),
        }
    }
}

impl ReadLatest for View {
    fn latest(&self) -> &HashMap<String, Sample> {
        &self.latest
    }
}

// ────────────────────────────────────────────
//  Helpers
// ────────────────────────────────────────────

/// Lit un fichier texte et le retourne trimé.
pub fn read_trimmed(path: &str) -> Result<String> {
    std::fs::read_to_string(path)
        .map(|s| s.trim().to_string())
        .map_err(|e| format!("{}: {}", path, e))
}

/// Lit un fichier et parse un f64.
pub fn read_f64(path: &str) -> Result<f64> {
    read_trimmed(path)?
        .parse()
        .map_err(|e| format!("parse {}: {}", path, e))
}

/// Écrit une valeur dans un fichier sysfs.
/// Utilisé par les contrôles de frequency scaling (CPU/GPU).
pub fn write_sysfs(path: &str, value: &str) -> Result<()> {
    std::fs::write(path, value).map_err(|e| format!("write {}: {}", path, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mhz_autoscales_to_ghz() {
        assert_eq!(Unit::Mhz.fmt(2400.0), "2.40 GHz");
        assert_eq!(Unit::Mhz.fmt(400.0), "400.0MHz");
    }

    #[test]
    fn bytes_autoscale() {
        assert_eq!(Unit::Bytes.fmt(2048.0), "2.0 KiB");
        assert_eq!(Unit::Bytes.fmt(3.5 * 1024.0 * 1024.0 * 1024.0), "3.50 GiB");
    }

    #[test]
    fn ms_autoscale_prefers_minutes() {
        assert_eq!(Unit::Ms.fmt(65_000.0), "1.08 min");
        assert_eq!(Unit::Ms.fmt(1500.0), "1.50 s");
        assert_eq!(Unit::Ms.fmt(500.0), "500.0ms");
    }

    #[test]
    fn adaptive_precision() {
        assert_eq!(Unit::Percent.fmt(12.345), "12.35%");
        assert_eq!(Unit::Percent.fmt(0.005), "0.0050%");
    }

    #[test]
    fn sample_key_and_display() {
        let s = Sample {
            kind: "cpu".to_string(),
            label: "CPU0 Freq".to_string(),
            value: Value::Num(2400.0),
            unit: Unit::Mhz,
        };
        assert_eq!(s.key(), "cpu/CPU0 Freq");
        assert_eq!(s.display(), "2.40 GHz");
    }

    struct ScalarSource(f64);

    impl Source for ScalarSource {
        fn read(&mut self) -> Result<Vec<Sample>> {
            Ok(vec![Sample {
                kind: "x".to_string(),
                label: "y".to_string(),
                value: Value::Num(self.0),
                unit: Unit::Count,
            }])
        }
    }

    #[test]
    fn poll_all_applies_sources_and_derivers() {
        fn add_one(samples: &[Sample]) -> Vec<Sample> {
            samples
                .iter()
                .filter_map(|s| {
                    s.value.as_num().map(|v| Sample {
                        kind: "derived".to_string(),
                        label: s.label.clone(),
                        value: Value::Num(v + 1.0),
                        unit: Unit::Count,
                    })
                })
                .collect()
        }

        let mut s = Sampler::new();
        s.add(Box::new(ScalarSource(1.0)));
        s.derive(add_one);
        let samples = s.poll_all();
        assert_eq!(samples.len(), 2);
        assert_eq!(samples[0].value.as_num(), Some(1.0));
        assert_eq!(samples[1].value.as_num(), Some(2.0));
    }
}
