//! Point d'entrée — NyVoara Power & Performance Dashboard.
//!
//! 1. Parse les arguments
//! 2. Auto-élévation via pkexec
//! 3. Dispatch : profil (--apply), GUI (egui) ou CLI

mod args;
mod cli;
mod gui;

pub use nyvoara_core::core;
pub use nyvoara_core::paths;
pub use nyvoara_core::profiles;
pub use nyvoara_library as lib;

fn main() {
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

    // 2. Élévation si nécessaire
    args::ensure_elevated();

    // 3. Application d'un profil (one-shot)
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

    // 4. Dispatch
    if args.cli {
        cli::run(args.watch);
    } else if has_display() {
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
fn has_display() -> bool {
    std::env::var("WAYLAND_DISPLAY").is_ok()
        || std::env::var("DISPLAY")
            .map(|v| !v.is_empty())
            .unwrap_or(false)
}
