#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# NyVoara — installateur système
#
# Installe l'application dans /opt/nyvoara + l'entrée de menu + l'icône +
# l'action polkit :
#   /opt/nyvoara/nyvoara                              binaire
#   /opt/nyvoara/icon.png                             icône embarquée
#   /usr/share/applications/nyvoara.desktop           lanceur de menu
#   /usr/share/icons/hicolor/128x128/apps/nyvoara.png
#   /usr/share/polkit-1/actions/io.github.rakmahefa.nyvoara.policy
#   /usr/local/bin/nyvoara → /opt/nyvoara/nyvoara
#
# Usage :
#   sudo bash install.sh                 installation standard (tout le monde)
#   INSTALL_ROOT=/un/rep bash install.sh installation déportée (tests)
#   bash install.sh --build              force la compilation depuis les sources
#
# Le script :
#   1. installe les dépendances système (apt/pacman/dnf) si absentes ;
#   2. copie le binaire fourni ; si celui-ci est incompatible avec la
#      libc/distribution, il bascule sur une compilation depuis GitHub ;
#   3. installe lanceur, icône, action polkit et symlink.
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

INSTALL_ROOT="${INSTALL_ROOT:-/}"
APP_DIR="${INSTALL_ROOT%/}/opt/nyvoara"
SRC_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_URL="https://github.com/rakmahefa/surface-core-tuner"
REPO_BRANCH="feature/tray-minimize"
FORCE_BUILD="${1:-}"

say()  { printf '\033[1;34m==>\033[0m %s\n' "$*"; }
ok()   { printf '\033[1;32m ✓\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m ⚠\033[0m %s\n' "$*"; }
die()  { printf '\033[1;31m ✗\033[0m %s\n' "$*" >&2; exit 1; }

if [ "${INSTALL_ROOT:-/}" = "/" ] && [ "$(id -u)" -ne 0 ]; then
    die "lancez en root : sudo bash $0 (ou INSTALL_ROOT=/rep pour un test sans root)"
fi

# ── 1. Dépendances système ───────────────────────────────────────────────────
# webkit2gtk-4.1 (webview), gtk3 (fenêtrage), libsoup (webview),
# libxdo (raccourcis muda), libayatana-appindicator3 (icône de
# notification), polkit/pkexec (élévation des écritures sysfs).

if command -v pacman >/dev/null 2>&1; then
    # Arch / Manjaro / EndeavourOS…
    DEPS_PACMAN="webkit2gtk-4.1 gtk3 libayatana-appindicator libxdo libsoup3 polkit"
    MISSING=""
    for p in $DEPS_PACMAN; do
        pacman -Qi "$p" >/dev/null 2>&1 || MISSING="$MISSING $p"
    done
    if [ -n "${MISSING// /}" ]; then
        say "installation des dépendances :${MISSING}"
        pacman -Sy --needed --noconfirm $MISSING || warn "certaines dépendances n'ont pas pu s'installer — le script continue"
    else
        ok "dépendances système déjà présentes"
    fi
elif command -v apt-get >/dev/null 2>&1; then
    # Debian / Ubuntu / Mint…
    DEPS_APT="libwebkit2gtk-4.1-0 libgtk-3-0 libsoup-3.0-0 libjavascriptcoregtk-4.1-0 \
libxdo3 libayatana-appindicator3-1 pkexec"
    MISSING=""
    for p in $DEPS_APT; do
        dpkg-query -W -f='${Status}' "$p" 2>/dev/null | grep -q "install ok installed" || MISSING="$MISSING $p"
    done
    # Variante t64 du nom de paquet (Debian ≥13 : libgtk-3-0t64…)
    if echo "$MISSING" | grep -q "libgtk-3-0"; then
        dpkg-query -W -f='${Status}' libgtk-3-0t64 2>/dev/null | grep -q "install ok installed" \
            && MISSING="$(echo "$MISSING" | sed 's/libgtk-3-0 //')"
    fi
    if [ -n "${MISSING// /}" ]; then
        say "installation des dépendances :${MISSING}"
        apt-get update -qq || true
        # shellcheck disable=SC2086
        apt-get install -y -qq $MISSING || warn "certaines dépendances n'ont pas pu s'installer — le script continue"
    else
        ok "dépendances système déjà présentes"
    fi
else
    warn "ni pacman ni apt-get — vérifiez manuellement : webkit2gtk-4.1, gtk3, libayatana-appindicator, libxdo, polkit"
fi

# ── 2. Installation du binaire ───────────────────────────────────────────────
mkdir -p "$APP_DIR"
install -m 0755 "$SRC_DIR/opt/nyvoara/nyvoara" "$APP_DIR/nyvoara"
install -m 0644 "$SRC_DIR/opt/nyvoara/icon.png" "$APP_DIR/icon.png"

# Test de compatibilité : --list-profiles ne nécessite ni display ni élévation.
binary_ok() {
    "$APP_DIR/nyvoara" --list-profiles >/dev/null 2>&1
}

if [ "$FORCE_BUILD" = "--build" ] || ! binary_ok; then
    warn "le binaire fourni ne tourne pas sur ce système (libc ou dépendances) — compilation depuis les sources"
    command -v git   >/dev/null 2>&1 || die "git est requis (pacman -S git / apt install git)"
    command -v cargo >/dev/null 2>&1 || die "cargo est requis : https://rustup.rs (rustup default stable)"
    say "clonage de $REPO_URL (branche $REPO_BRANCH)"
    rm -rf /tmp/nyvoara-build
    git clone --depth 1 --branch "$REPO_BRANCH" "$REPO_URL" /tmp/nyvoara-build
    say "installation des dépendances de compilation…"
    if command -v pacman >/dev/null 2>&1; then
        # Arch : les paquets runtime embarquent les en-têtes (pas de -dev).
        pacman -Sy --needed --noconfirm base-devel webkit2gtk-4.1 gtk3 \
            libayatana-appindicator libxdo
    else
        apt-get update -qq && apt-get install -y -qq libwebkit2gtk-4.1-dev libgtk-3-dev \
            libayatana-appindicator3-dev libxdo-dev librsvg2-dev
    fi
    say "compilation (plusieurs minutes)…"
    (cd /tmp/nyvoara-build && cargo build --release -p nyvoara)
    install -m 0755 /tmp/nyvoara-build/target/release/nyvoara "$APP_DIR/nyvoara"
    binary_ok || die "le binaire compilé ne démarre pas non plus — consultez : $APP_DIR/nyvoara --list-profiles"
    ok "compilé et installé"
else
    ok "binaire installé : $APP_DIR/nyvoara"
fi

# ── 3. Lanceur de menu, icône, action polkit, symlink ────────────────────────
install -D -m 0644 "$SRC_DIR/share/applications/nyvoara.desktop" \
    "${INSTALL_ROOT%/}/usr/share/applications/nyvoara.desktop"
install -D -m 0644 "$SRC_DIR/share/icons/hicolor/128x128/apps/nyvoara.png" \
    "${INSTALL_ROOT%/}/usr/share/icons/hicolor/128x128/apps/nyvoara.png"

# Action polkit : mémorise l'authentification de l'aide --sysfs-write
# (auth_admin_keep) — un seul mot de passe par session de réglage.
if [ -f "$SRC_DIR/io.github.rakmahefa.nyvoara.policy" ]; then
    install -D -m 0644 "$SRC_DIR/io.github.rakmahefa.nyvoara.policy" \
        "${INSTALL_ROOT%/}/usr/share/polkit-1/actions/io.github.rakmahefa.nyvoara.policy"
    ok "action polkit installée (io.github.rakmahefa.nyvoara.sysfs)"
fi

mkdir -p "${INSTALL_ROOT%/}/usr/local/bin"
ln -sf "$APP_DIR/nyvoara" "${INSTALL_ROOT%/}/usr/local/bin/nyvoara"

command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database -q "${INSTALL_ROOT%/}/usr/share/applications" 2>/dev/null || true
command -v gtk-update-icon-cache   >/dev/null 2>&1 && gtk-update-icon-cache   -q -t -f "${INSTALL_ROOT%/}/usr/share/icons/hicolor" 2>/dev/null || true

ok "installation terminée"
cat <<'EOF'

  Lancement :
    • depuis le menu d'applications : « NyVoara »
    • en terminal : nyvoara  (ou /opt/nyvoara/nyvoara)

  L'application tourne SANS privilèges — l'icône vit dans la barre de
  notification dès le lancement :
    • bouton ✕           → réduit dans la zone de notification
    • clic droit (icône) → Show NyVoara / Minimize to tray / Quit

  Élévation : à la première action de réglage (profil, gouverneur,
  fréquence…), l'agent polkit du bureau demande le mot de passe
  administrateur, puis le mémorise quelques minutes (action
  io.github.rakmahefa.nyvoara.sysfs, auth_admin_keep). Les écritures
  sont confinées aux arbres sysfs matériels (CPU/GPU/batterie).

  IMPORTANT — icône de notification : lancer depuis la session
  graphique (menu, terminal du bureau). Un lancement via SSH
  n'aura pas de bus D-Bus de session : l'icône sera absente et le
  bouton ✕ quittera l'application (comportement de repli sûr).
  `sudo nyvoara` fonctionne aussi (lectures/écritures directes)
  mais l'icône de notification peut être absente : privilégiez le
  mode normal, ou `nyvoara --root` pour forcer l'ancien
  comportement (tout le processus élevé).

  Désinstallation : sudo bash uninstall.sh
EOF
