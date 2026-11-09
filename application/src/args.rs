//! Gestion des arguments CLI et élévation explicite via pkexec.
//!
//! # Modes
//!
//! - Sans argument : GUI (par défaut) — **sans élévation** : l'icône de
//!   notification et le menu contextuel exigent le bus de session D-Bus de
//!   l'utilisateur, inaccessible à un processus root ; les écritures sysfs
//!   sont déléguées à l'aide `pkexec … --sysfs-write` (voir
//!   `nyvoara_core::core::write_sysfs`) ;
//! - `--cli` : affichage console one-shot (lecture seule, sans élévation) ;
//! - `--cli --watch` : affichage console continu ;
//! - `--apply <nom>` : applique un profil et exit (auto-délégué à l'aide
//!   privilégiée hors root) ;
//! - `--list-profiles` : liste les profils disponibles ;
//! - `--root` : comportement historique — éève TOUT le processus (utile en
//!   diagnostic ; l'icône tray peut alors être absente) ;
//! - `--sysfs-write <path> <value>` : aide interne one-shot, lancée par
//!   pkexec, jamais tapée à la main ;
//! - `--profiles-file <path>` : utilisé par l'aide pour retrouver les
//!   profils personnalisés de l'utilisateur (pkexec impose `HOME=/root`).

use std::env;

/// Arguments du programme.
pub struct Args {
    pub cli: bool,
    pub watch: bool,
    pub apply: Option<String>,
    pub list_profiles: bool,
    /// Comportement historique : élever tout le processus.
    pub root: bool,
    /// Aide privilégiée interne : `--sysfs-write <path> <value>`.
    pub sysfs_write: Option<(String, String)>,
    /// Fichier de profils personnalisés explicite (aides pkexec).
    pub profiles_file: Option<String>,
}

impl Args {
    /// Parse les arguments de la ligne de commande.
    /// Minimum de code, pas de dépendance externe.
    pub fn parse() -> Self {
        let mut cli = false;
        let mut watch = false;
        let mut apply = None;
        let mut list_profiles = false;
        let mut root = false;
        let mut sysfs_write = None;
        let mut profiles_file = None;

        let mut args = env::args().skip(1);
        while let Some(a) = args.next() {
            match a.as_str() {
                "--cli" => cli = true,
                "--watch" => watch = true,
                "--list-profiles" => list_profiles = true,
                "--root" => root = true,
                "--apply" => apply = args.next(),
                "--sysfs-write" => {
                    let path = args.next();
                    let value = args.next();
                    if let (Some(p), Some(v)) = (path, value) {
                        sysfs_write = Some((p, v));
                    }
                }
                "--profiles-file" => profiles_file = args.next(),
                _ => {}
            }
        }

        Self {
            cli,
            watch,
            apply,
            list_profiles,
            root,
            sysfs_write,
            profiles_file,
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
/// Réservé au mode explicite `--root` : le mode graphique par défaut ne
/// s'élève plus (icône de notification), les écritures passent par l'aide
/// one-shot `--sysfs-write`.
///
/// Passe **tout** l'environnement courant : `pkexec` purge
/// l'environnement, et seul `env` peut le reconstruire dans le
/// processus root. Sans cela :
/// - `XDG_RUNTIME_DIR` manquant → libwayland échoue
///   (« XDG_RUNTIME_DIR is invalid or not set ») et les couches
///   Vulkan implicites (mangohud, gamescope-wsi) plantent ;
/// - pas d'accès au display (WAYLAND_DISPLAY + XDG_RUNTIME_DIR, ou
///   DISPLAY + XAUTHORITY) ;
/// - le bus de session D-Bus serait perdu → icône de notification
///   et menu contextuel inopérants (voir [`crate::session`]).
///
/// `NYVOARA_SKIP_ELEVATION=1` contourne l'élévation (dev, CI,
/// conteneurs de test sans polkit).
pub fn ensure_elevated() {
    if is_root() {
        return;
    }

    if env::var_os("NYVOARA_SKIP_ELEVATION").is_some_and(|v| v == "1") {
        return;
    }

    let exe = env::current_exe().expect("current exe path");
    let args: Vec<String> = env::args().skip(1).collect();

    let status = match std::process::Command::new("pkexec")
        .arg("env")
        .args(full_env())
        .arg(&exe)
        .args(&args)
        .status()
    {
        Ok(status) => status,
        Err(e) => {
            eprintln!("✗ Cannot elevate: pkexec failed to start ({e}).");
            eprintln!("  Install polkit's pkexec (e.g. `apt install policykit-1`),");
            eprintln!("  or run NyVoara as root: sudo nyvoara");
            std::process::exit(1);
        }
    };

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
