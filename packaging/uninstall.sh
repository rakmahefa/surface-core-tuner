#!/usr/bin/env bash
# NyVoara — désinstallateur système (inverse install.sh)
set -euo pipefail

INSTALL_ROOT="${INSTALL_ROOT:-/}"

say() { printf '\033[1;34m==>\033[0m %s\n' "$*"; }

[ "$(id -u)" -eq 0 ] || { echo "lancez en root : sudo bash $0" >&2; exit 1; }

say "suppression de l'application…"
rm -rf "${INSTALL_ROOT%/}/opt/nyvoara"
rm -f  "${INSTALL_ROOT%/}/usr/share/applications/nyvoara.desktop"
rm -f  "${INSTALL_ROOT%/}/usr/share/icons/hicolor/128x128/apps/nyvoara.png"
rm -f  "${INSTALL_ROOT%/}/usr/local/bin/nyvoara"

command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database -q "${INSTALL_ROOT%/}/usr/share/applications" 2>/dev/null || true

echo " ✓ NyVoara désinstallé."
