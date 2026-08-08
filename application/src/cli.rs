//! Interface CLI.
//!
//! Deux modes :
//! - `--cli` : one-shot, imprime toutes les métriques et exit
//! - `--cli --watch` : continu, rafraîchit toutes les secondes

use crate::core::*;
use crate::library;
use std::io::{stdout, Write};
use std::time::Duration;

/// Lance le mode CLI.
pub fn run(watch: bool) {
    let mut sampler = library::build_sampler();

    if watch {
        run_watch(&mut sampler);
    } else {
        run_once(&mut sampler);
    }
}

/// Délai entre les deux lectures du mode one-shot : les métriques
/// par delta (load%, busy%, IOPS, débits) ont besoin d'un intervalle
/// mesurable entre deux échantillons.
const WARMUP_MS: u64 = 250;

/// Mode one-shot : imprime tout et exit.
fn run_once(sampler: &mut Sampler) {
    let _ = sampler.poll_all(); // warm-up : amorce les compteurs de delta
    std::thread::sleep(Duration::from_millis(WARMUP_MS));
    let samples = sampler.poll_all();
    print_report(&samples);
}

/// Mode watch : boucle avec rafraîchissement ANSI.
fn run_watch(sampler: &mut Sampler) {
    let mut stdout = stdout();
    loop {
        let samples = sampler.poll_all();
        print_clear(&mut stdout);
        print_report(&samples);
        print_flush(&mut stdout);
        std::thread::sleep(Duration::from_secs(1));
    }
}

// ── Formatage ───────────────────────────────

fn print_report(samples: &[Sample]) {
    let mut by_kind: std::collections::BTreeMap<&str, Vec<&Sample>> =
        std::collections::BTreeMap::new();
    for s in samples {
        by_kind.entry(s.kind.as_str()).or_default().push(s);
    }

    println!("╔══════════════════════════════════════════╗");
    println!("║  NyVoara — Power & Performance Dashboard ║");
    println!("╚══════════════════════════════════════════╝");
    println!();

    for (kind, samples) in &by_kind {
        println!("── {} ──", kind.to_uppercase());
        for s in samples {
            let display = s.display();
            println!("  {:30} {}", s.label, display);
        }
        println!();
    }
}

fn print_clear(stdout: &mut std::io::Stdout) {
    // ANSI clear screen and move cursor home
    let _ = write!(stdout, "\x1b[H\x1b[J");
}

fn print_flush(stdout: &mut std::io::Stdout) {
    let _ = stdout.flush();
}
