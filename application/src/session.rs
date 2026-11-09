//! Environnement de session (bus D-Bus) pour l'icône de notification.
//!
//! Sur Linux, l'icône de la zone de notification repose sur
//! libayatana-appindicator, qui a impérativement besoin d'un **bus de session
//! D-Bus** pour s'enregistrer auprès de l'hôte StatusNotifier (KDE Plasma,
//! XFCE, GNOME + extension AppIndicator…) et router les clics du menu
//! contextuel vers l'application.
//!
//! Deux situations courantes privent le processus de ce bus :
//! - lancement via `sudo` : sudo retire `DBUS_SESSION_BUS_ADDRESS` et
//!   `XDG_RUNTIME_DIR` de l'environnement — GLib tente alors un autolaunch
//!   X11 (`dbus-launch --autolaunch=…`) qui échoue, et l'appindicator
//!   s'affiche en repli XEmbed avec un menu dont les clics n'arrivent jamais
//!   (« Unable to get the session bus » dans la console) ;
//! - lancement depuis un contexte amputé (SSH, service, terminal hors
//!   session), puis auto-élévation `pkexec env …` qui transmet ce contexte
//!   vide au processus root.
//!
//! Ce module reconstruit l'adresse du bus à partir des chemins standards
//! systemd (`/run/user/<uid>/bus`) **avant** l'élévation — elle est ainsi
//! transmise au processus élevé — et la re-détecte dans le processus élevé
//! grâce à `SUDO_UID` / `PKEXEC_UID` (le bus de l'utilisateur réel reste
//! accessible à root).

use std::env;
use std::path::PathBuf;

/// Garantit au mieux un bus de session D-Bus joignable.
///
/// Sans effet si `DBUS_SESSION_BUS_ADDRESS` est déjà défini, ou si aucun
/// socket standard n'existe (session réellement sans bus : rien à faire).
pub fn ensure_session_bus() {
    let already_set = env::var_os("DBUS_SESSION_BUS_ADDRESS")
        .is_some_and(|v| !v.is_empty());
    if already_set {
        return;
    }

    for uid in candidate_uids() {
        let socket = PathBuf::from(format!("/run/user/{uid}/bus"));
        if socket.exists() {
            let addr = format!("unix:path={}", socket.display());
            env::set_var("DBUS_SESSION_BUS_ADDRESS", &addr);

            // `sudo` retire aussi XDG_RUNTIME_DIR : le restaurer aide GLib
            // (repli natif $XDG_RUNTIME_DIR/bus), la webview Wayland et le
            // répertoire temporaire de l'icône tray (XDG_RUNTIME_DIR/tray-icon).
            if env::var_os("XDG_RUNTIME_DIR").map_or(true, |v| v.is_empty()) {
                let runtime = PathBuf::from(format!("/run/user/{uid}"));
                if runtime.is_dir() {
                    env::set_var("XDG_RUNTIME_DIR", runtime);
                }
            }
            return;
        }
    }
}

/// Avertit sur stderr si aucun bus de session n'est joignable : l'icône de
/// notification et son menu ne fonctionneront pas (clics sans effet).
pub fn warn_if_no_session_bus() {
    let set = env::var_os("DBUS_SESSION_BUS_ADDRESS")
        .is_some_and(|v| !v.is_empty());
    if set {
        return;
    }
    eprintln!("⚠ No D-Bus session bus found — the notification-area icon and its menu will NOT work.");
    eprintln!("  Start NyVoara from your desktop session (application menu) rather than");
    eprintln!("  over SSH or a bare `sudo`; this keeps the tray icon connected to your session.");
}

/// Uid candidats, par ordre de préférence : l'uid effectif du processus,
/// puis l'uid de l'utilisateur à l'origine de l'élévation (`sudo`/`pkexec`).
fn candidate_uids() -> Vec<String> {
    let mut uids: Vec<String> = Vec::new();

    if let Ok(out) = std::process::Command::new("id").arg("-u").output() {
        let uid = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !uid.is_empty() {
            uids.push(uid);
        }
    }

    // Processus root issu d'une élévation : le bus de l'utilisateur réel
    // (/run/user/<uid>/bus) reste joignable par root.
    for var in ["SUDO_UID", "PKEXEC_UID"] {
        if let Ok(v) = env::var(var) {
            if !v.is_empty() && v.chars().all(|c| c.is_ascii_digit()) && !uids.contains(&v) {
                uids.push(v);
            }
        }
    }

    uids
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidate_uids_are_numeric_and_unique() {
        let uids = candidate_uids();
        assert!(!uids.is_empty());
        for uid in &uids {
            assert!(uid.chars().all(|c| c.is_ascii_digit()));
        }
        let mut sorted = uids.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(uids.len(), sorted.len());
    }
}
