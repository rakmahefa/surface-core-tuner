//! Gestion des arguments CLI et auto-élévation via pkexec.
//!
//! # Modes
//!
//! - Sans argument : GUI egui (par défaut)
//! - `--cli` : affichage console one-shot
//! - `--cli --watch` : affichage console continu
//! - `--apply <nom>` : applique un profil et exit
//! - `--list-profiles` : liste les profils disponibles

use std::env;

/// Arguments du programme.
pub struct Args {
    pub cli: bool,
    pub watch: bool,
    pub apply: Option<String>,
    pub list_profiles: bool,
}

impl Args {
    /// Parse les arguments de la ligne de commande.
    /// Minimum de code, pas de dépendance externe.
    pub fn parse() -> Self {
        let mut cli = false;
        let mut watch = false;
        let mut apply = None;
        let mut list_profiles = false;

        let mut args = env::args().skip(1);
        while let Some(a) = args.next() {
            match a.as_str() {
                "--cli" => cli = true,
                "--watch" => watch = true,
                "--list-profiles" => list_profiles = true,
                "--apply" => apply = args.next(),
                _ => {}
            }
        }

        Self {
            cli,
            watch,
            apply,
            list_profiles,
        }
    }
}

/// Vérifie si le processus tourne avec les droits root.
fn is_root() -> bool {
    std::process::Command::new("id")
        .arg("-u")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "0")
        .unwrap_or(false)
}

/// S'élève via pkexec si nécessaire.
///
/// Passe **tout** l'environnement courant : `pkexec` purge
/// l'environnement, et seul `env` peut le reconstruire dans le
/// processus root. Sans cela :
/// - `XDG_RUNTIME_DIR` manquant → libwayland échoue
///   (« XDG_RUNTIME_DIR is invalid or not set ») et les couches
///   Vulkan implicites (mangohud, gamescope-wsi) plantent ;
/// - pas d'accès au display (WAYLAND_DISPLAY + XDG_RUNTIME_DIR, ou
///   DISPLAY + XAUTHORITY) ;
/// - mangohud et autres outils injectés inopérants (MANGOHUD=1,
///   LD_PRELOAD, layers Vulkan).
pub fn ensure_elevated() {
    if is_root() {
        return;
    }

    let exe = env::current_exe().expect("current exe path");
    let args: Vec<String> = env::args().skip(1).collect();

    let status = std::process::Command::new("pkexec")
        .arg("env")
        .args(full_env())
        .arg(&exe)
        .args(&args)
        .status()
        .expect("failed to launch pkexec");

    std::process::exit(status.code().unwrap_or(1));
}

/// Reconstruction complète de l'environnement (`NAME=value`) pour `env`.
///
/// `vars_os` + `to_string_lossy` : une variable non-UTF-8 ne peut pas
/// faire paniquer l'élévation.
fn full_env() -> Vec<String> {
    env::vars_os()
        .map(|(k, v)| format!("{}={}", k.to_string_lossy(), v.to_string_lossy()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::full_env;

    #[test]
    fn full_env_mirrors_process_environment() {
        let expected: Vec<String> = std::env::vars_os()
            .map(|(k, v)| format!("{}={}", k.to_string_lossy(), v.to_string_lossy()))
            .collect();
        assert_eq!(full_env(), expected);
    }
}
