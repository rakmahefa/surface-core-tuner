//! Point d'entrée — NyVoara Power & Performance Dashboard.

mod app;
#[cfg(not(target_family = "wasm"))]
mod args;
#[cfg(not(target_family = "wasm"))]
mod cli;
mod data;
mod engine;
mod gui;
#[cfg(not(target_family = "wasm"))]
mod session;
mod ui;

pub use nyvoara_core::core;
pub use nyvoara_core::paths;
pub use nyvoara_core::profiles;
pub use nyvoara_library as library;

#[cfg(target_family = "wasm")]
fn main() {
    // Build web : démonstration uniquement (pas de sysfs depuis un navigateur).
    gui::run();
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    // 0. Bus de session D-Bus : l'icône de notification en dépend
    //    (libayatana-appindicator). Détecté avant tout le reste pour que
    //    l'état soit connu au moment de choisir le comportement du ✕
    //    (voir `ui::tray` : sans bus, ✕ quitte au lieu de masquer).
    session::ensure_session_bus();

    // 1. Arguments
    let args = args::Args::parse();

    // Liste des profils : lisible sans élévation
    if args.list_profiles {
        let all = profiles::load_all();
        let (predefined, custom) = profiles::split(&all);
        println!("Available profiles:");
        for p in predefined {
            println!("  {}", p.name);
        }
        for p in custom {
            println!("  {} (custom)", p.name);
        }
        return;
    }

    // 2. Aide privilégiée one-shot `--sysfs-write <path> <value>`.
    //    Lancée par pkexec (donc root) depuis `core::write_sysfs` —
    //    jamais tapée à la main. Elle écrit DIRECTEMENT (pas de
    //    re-délégation) et refuse tout chemin hors allowlist.
    if let Some((path, value)) = &args.sysfs_write {
        if !core::sysfs_writable(path) {
            eprintln!("✗ Refused: {path} is outside the allowed sysfs trees.");
            std::process::exit(1);
        }
        if !core::is_root() {
            eprintln!("✗ --sysfs-write is an internal helper: run it via pkexec.");
            std::process::exit(1);
        }
        if value.is_empty() || value.len() > 64 || !value.chars().all(|c| c.is_ascii_graphic()) {
            eprintln!("✗ Refused: invalid sysfs value.");
            std::process::exit(1);
        }
        match std::fs::write(path, value) {
            Ok(()) => std::process::exit(0),
            Err(e) => {
                eprintln!("✗ write {path}: {e}");
                std::process::exit(1);
            }
        }
    }

    // 3. Fichier de profils explicite (aides pkexec : HOME=/root).
    if let Some(file) = &args.profiles_file {
        profiles::set_override_path(std::path::PathBuf::from(file));
    }

    // 4. Mode `--root` : comportement historique (tout le processus
    //    élevé). Le mode par défaut ne s'élève PLUS — l'icône de
    //    notification exige le bus de session de l'utilisateur.
    if args.root {
        args::ensure_elevated();
    }

    // 5. Application d'un profil (one-shot). Hors root, `Profile::apply`
    //    délègue à l'aide privilégiée (une seule demande de mot de passe).
    if let Some(name) = args.apply.as_deref() {
        let all = profiles::load_all();
        match profiles::find(&all, name) {
            Some(p) => match p.apply() {
                Ok(()) => println!("✓ Profile «{}» applied.", p.name),
                Err(e) => {
                    eprintln!("✗ Failed to apply «{}»: {}", p.name, e);
                    std::process::exit(1);
                }
            },
            None => {
                eprintln!("✗ Unknown profile «{name}».");
                eprintln!("  Available: {}", profiles::names(&all).join(", "));
                std::process::exit(1);
            }
        }
        return;
    }

    // 6. Dispatch — sans élévation : lectures sysfs lisibles par tous,
    //    écritures déléguées à l'aide pkexec.
    if args.cli {
        cli::run(args.watch);
    } else if has_display() {
        session::warn_if_no_session_bus();
        gui::run();
    } else {
        let exe = std::env::current_exe()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "nyvoara".into());
        eprintln!("⚠ Aucun serveur d'affichage détecté (ni WAYLAND ni X11).");
        eprintln!("  Utilisez --cli pour le mode console :");
        eprintln!("    {exe} --cli");
        eprintln!("    {exe} --cli --watch");
        eprintln!("  Ou appliquez un profil :");
        eprintln!("    {exe} --list-profiles");
        eprintln!("    {exe} --apply <nom>");
        std::process::exit(1);
    }
}

/// Vérifie la disponibilité d'un serveur d'affichage.
#[cfg(not(target_family = "wasm"))]
fn has_display() -> bool {
    std::env::var("WAYLAND_DISPLAY").is_ok()
        || std::env::var("DISPLAY")
            .map(|v| !v.is_empty())
            .unwrap_or(false)
}
