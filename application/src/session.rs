//! Environnement de session (bus D-Bus) pour l'icône de notification.
//!
//! Sur Linux, l'icône de la zone de notification repose sur
//! libayatana-appindicator, qui a impérativement besoin d'un **bus de
//! session D-Bus** pour s'enregistrer auprès de l'hôte StatusNotifier
//! (KDE Plasma, XFCE, GNOME + extension AppIndicator…) et router les
//! clics du menu contextuel vers l'application.
//!
//! Situations courantes qui privent le processus de ce bus :
//! - lancement via `sudo` : sudo retire `DBUS_SESSION_BUS_ADDRESS` et
//!   `XDG_RUNTIME_DIR` de l'environnement — GLib tente alors un autolaunch
//!   X11 (`dbus-launch --autolaunch=…`) qui échoue, et l'appindicator
//!   reste invisible (« Unable to get the session bus » dans la console) ;
//! - une adresse `DBUS_SESSION_BUS_ADDRESS` **périmée** (socket mort,
//!   session fermée) : la connexion est refusée/réinitialisée au lieu
//!   d'échouer proprement ;
//! - un contexte amputé (SSH, service, terminal hors session).
//!
//! Ce module **valide** l'adresse du bus par une vraie poignée de main
//! D-Bus (SASL `AUTH EXTERNAL`) avant de s'en remettre à elle : une
//! adresse morte est retirée de l'environnement, et les sockets
//! standards (`$XDG_RUNTIME_DIR/bus`, `/run/user/<uid>/bus`) sont
//! sondées en repli. Le résultat est exposé à [`bus_ok`] pour que
//! l'interface adapte le comportement du bouton ✕ (sans bus, quitter
//! plutôt que masquer — jamais de fenêtre injoignable).

use std::env;
use std::io::{Read, Write};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Vrai si un bus de session joignable a été confirmé (ou si une adresse
/// non testable est définie). Lu par `ui::tray` pour choisir le
/// comportement du ✕.
static BUS_OK: AtomicBool = AtomicBool::new(false);

/// État du bus de session (après [`ensure_session_bus`]).
pub fn bus_ok() -> bool {
    BUS_OK.load(Ordering::Relaxed)
}

/// Garantit au mieux un bus de session D-Bus joignable.
///
/// 1. si `DBUS_SESSION_BUS_ADDRESS` est défini : poignée de main de
///    validation ; morte → retirée de l'environnement ;
/// 2. sinon (ou après échec) : sondage de `$XDG_RUNTIME_DIR/bus` puis
///    `/run/user/<uid>/bus` pour l'uid effectif et l'uid à l'origine
///    d'une éventuelle élévation (`SUDO_UID`/`PKEXEC_UID`) ;
/// 3. met à jour [`bus_ok`] et, au passage, restaure `XDG_RUNTIME_DIR`.
pub fn ensure_session_bus() {
    let mut ok = false;

    if let Some(addr) = env::var_os("DBUS_SESSION_BUS_ADDRESS") {
        let addr = addr.to_string_lossy().to_string();
        if !addr.is_empty() {
            if let Some(path) = unix_path(&addr) {
                if probe(&path) {
                    ok = true;
                } else {
                    // Adresse périmée : la retirer évite à GLib de
                    // s'accrocher à un socket mort — les replis standards
                    // restent tentés ci-dessous.
                    env::remove_var("DBUS_SESSION_BUS_ADDRESS");
                    eprintln!("⚠ DBUS_SESSION_BUS_ADDRESS points to a dead bus ({addr}) — removed, falling back to standard sockets.");
                }
            } else {
                // Forme non testable (abstract, tcp…) : faire confiance.
                ok = true;
            }
        }
    }

    if !ok {
        for path in candidate_sockets() {
            if probe(&path) {
                env::set_var("DBUS_SESSION_BUS_ADDRESS", format!("unix:path={}", path.display()));
                ok = true;
                break;
            }
        }
    }

    // `sudo`/`pkexec` retirent aussi XDG_RUNTIME_DIR : le restaurer aide
    // GLib (repli natif $XDG_RUNTIME_DIR/bus), la webview Wayland et le
    // répertoire temporaire de l'icône tray (XDG_RUNTIME_DIR/tray-icon).
    if env::var_os("XDG_RUNTIME_DIR").map_or(true, |v| v.is_empty()) {
        if let Some(uid) = first_uid() {
            let runtime = PathBuf::from(format!("/run/user/{uid}"));
            if runtime.is_dir() {
                env::set_var("XDG_RUNTIME_DIR", runtime);
            }
        }
    }

    BUS_OK.store(ok, Ordering::Relaxed);
}

/// Avertit sur stderr si aucun bus de session n'est joignable : l'icône de
/// notification et son menu ne fonctionneront pas (clics sans effet).
pub fn warn_if_no_session_bus() {
    if bus_ok() {
        return;
    }
    eprintln!("⚠ No D-Bus session bus found — the notification-area icon and its menu will NOT work.");
    eprintln!("  The ✕ button will therefore EXIT the app instead of minimizing it.");
    eprintln!("  Start NyVoara from your desktop session (application menu) rather than");
    eprintln!("  over SSH or a bare `sudo`; this keeps the tray icon connected to your session.");
}

/// Extrait le chemin d'une adresse `unix:path=…` (sinon `None` :
/// abstract, tcp… non testables).
fn unix_path(addr: &str) -> Option<PathBuf> {
    let body = addr.strip_prefix("unix:")?;
    body.split(',')
        .find_map(|kv| kv.strip_prefix("path="))
        .filter(|p| p.starts_with('/'))
        .map(PathBuf::from)
}

/// Sockets candidates, par ordre de préférence : runtime dir courant,
/// bus de l'uid effectif, puis bus de l'utilisateur à l'origine d'une
/// élévation (`sudo`/`pkexec` : le bus de l'utilisateur réel reste
/// accessible à root).
fn candidate_sockets() -> Vec<PathBuf> {
    let mut out = Vec::new();

    if let Some(rd) = env::var_os("XDG_RUNTIME_DIR") {
        if !rd.is_empty() {
            out.push(PathBuf::from(rd).join("bus"));
        }
    }
    for uid in candidate_uids() {
        out.push(PathBuf::from(format!("/run/user/{uid}/bus")));
    }
    out.retain(|p| p.exists());
    out
}

/// Poignée de main D-Bus minimale : connexion + SASL `AUTH EXTERNAL`.
///
/// Un `OK <guid>` prouve qu'un vrai daemon écoute ET accepte notre uid —
/// c'est exactement ce que fera GDBus quelques instants plus tard.
/// Socket mort, daemon absent ou auth refusée → `false` (silencieux :
/// ce sondage est un diagnostic, pas une erreur utilisateur).
fn probe(path: &PathBuf) -> bool {
    let Ok(mut stream) = UnixStream::connect(path) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(500)));

    let uid = first_uid().unwrap_or_else(|| "0".into());
    let hex: String = uid.bytes().map(|b| format!("{b:02x}")).collect();

    // Protocole D-Bus : octet nul, puis AUTH EXTERNAL <uid hex>.
    let hello = format!("\0AUTH EXTERNAL {hex}\r\n");
    if stream.write_all(hello.as_bytes()).is_err() {
        return false;
    }

    let mut buf = [0u8; 64];
    match stream.read(&mut buf) {
        Ok(n) if n > 0 => {
            let reply = String::from_utf8_lossy(&buf[..n]);
            // Fermer proprement (CANCEL + disconnect) avant de dropper.
            let _ = stream.write_all(b"CANCEL\r\n");
            let _ = stream.shutdown(Shutdown::Both);
            reply.starts_with("OK ")
        }
        _ => false,
    }
}

/// Premier uid candidat (effectif), pour l'auth SASL.
fn first_uid() -> Option<String> {
    candidate_uids().into_iter().next()
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

    #[test]
    fn unix_path_extracts_path_form_only() {
        assert_eq!(
            unix_path("unix:path=/run/user/1000/bus"),
            Some(PathBuf::from("/run/user/1000/bus"))
        );
        assert_eq!(
            unix_path("unix:path=/run/user/1000/bus,guid=abcdef"),
            Some(PathBuf::from("/run/user/1000/bus"))
        );
        assert_eq!(unix_path("unix:abstract=/tmp/dbus-xyz,guid=1"), None);
        assert_eq!(unix_path("tcp:host=localhost,port=1234"), None);
    }

    /// Un vrai daemon dbus doit être accepté ; un fichier ordinaire ou un
    /// socket sans listener doit être refusé. `dbus-daemon` est présent sur
    /// les environnements de dev/CI ; le test est ignoré sinon.
    #[test]
    fn probe_accepts_live_daemon_and_rejects_dead_socket() {
        let daemon = std::process::Command::new("dbus-daemon")
            .args([
                "--session",
                "--fork",
                "--print-address=1",
                "--print-pid=1",
                "--address=unix:path=/tmp/nyvoara-probe-test",
            ])
            .output();
        let Ok(out) = daemon else {
            eprintln!("dbus-daemon absent — test ignoré");
            return;
        };
        let text = String::from_utf8_lossy(&out.stdout);
        let mut lines = text.lines();
        let Some(path) = lines.next().and_then(unix_path) else {
            eprintln!("adresse inattendue : {text}");
            return;
        };
        let pid: Option<u32> = lines.next().and_then(|l| l.trim().parse().ok());

        // Daemon vivant : poignée de main OK.
        assert!(probe(&path), "live dbus-daemon should pass the probe");

        // Fichier ordinaire : connexion impossible → refusé.
        let plain = PathBuf::from("/tmp/nyvoara-probe-test-plain");
        std::fs::write(&plain, b"not a socket").unwrap();
        assert!(!probe(&plain));
        let _ = std::fs::remove_file(&plain);

        let _ = std::fs::remove_file(&path);
        if let Some(pid) = pid {
            let _ = std::process::Command::new("kill").arg(pid.to_string()).status();
        }
    }
}
