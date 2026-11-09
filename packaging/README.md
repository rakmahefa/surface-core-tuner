# Packaging `/opt/nyvoara`

Artefact d'installation système — prêt à copier-coller sur la machine cible.

```
packaging/
├── install.sh       sudo bash install.sh   (dépendances + /opt/nyvoara + menu + icône + polkit)
├── uninstall.sh     sudo bash uninstall.sh
├── nyvoara.desktop  entrée de menu (installée dans /usr/share/applications)
├── nyvoara.png      icône 128×128 (installée dans hicolor)
└── io.github.rakmahefa.nyvoara.policy   action polkit (auth_admin_keep)
```

L'installateur s'attend à trouver le binaire à côté : `opt/nyvoara/nyvoara`
(cf. structure ci-dessous). Le workflow GitHub Actions
(`.github/workflows/release.yml`, déclenché par un tag `v*`) construit et
publie l'archive complète.

```
nyvoara-linux-x86_64/
├── install.sh
├── uninstall.sh
├── io.github.rakmahefa.nyvoara.policy
├── README.md
├── opt/nyvoara/{nyvoara, icon.png}
└── share/{applications/nyvoara.desktop, icons/hicolor/128x128/apps/nyvoara.png}
```

## Installation (KDE Manjaro / Arch, Debian/Ubuntu, Fedora…)

```bash
tar xzf nyvoara-linux-x86_64.tar.gz
cd nyvoara-linux-x86_64
sudo bash install.sh
```

`install.sh` détecte le gestionnaire de paquets (`pacman`, `apt-get`)
et installe les dépendances : webkit2gtk-4.1, gtk3, libsoup3,
libayatana-appindicator, libxdo, polkit.

## Architecture d'élévation (v2)

L'application graphique tourne **sans privilèges** — condition
nécessaire pour que l'icône de notification s'enregistre sur le bus de
session D-Bus de l'utilisateur (un processus root ne peut généralement
pas joindre le bus de la session : icône absente du tray, menu inerte).

- **lectures** (CPU, thermique, batterie…) : sysfs lisible par tous ;
- **écritures** (gouverneurs, fréquences, profils) : déléguées à l'aide
  one-shot `pkexec /opt/nyvoara/nyvoara --sysfs-write <path> <value>`,
  confinée à une allowlist d'arbres sysfs
  (`/sys/devices/system/cpu/`, `/sys/class/drm/`,
  `/sys/class/power_supply/`) ;
- l'action polkit `io.github.rakmahefa.nyvoara.sysfs`
  (`auth_admin_keep`) mémorise le mot de passe quelques minutes :
  un seul prompt par session de réglage ;
- `nyvoara --root` restaure l'ancien comportement (tout le processus
  élevé) — en diagnostic uniquement, l'icône tray peut être absente.

## Construction de l'artefact

Automatique (recommandé) :

```bash
git tag v0.2.0 && git push origin v0.2.0   # → GitHub Release
```

Manuelle :

```bash
cargo build --release -p nyvoara
ARTIFACT=nyvoara-linux-x86_64
mkdir -p $ARTIFACT/opt/nyvoara $ARTIFACT/share/applications $ARTIFACT/share/icons/hicolor/128x128/apps
install -m 0755 target/release/nyvoara $ARTIFACT/opt/nyvoara/nyvoara
install -m 0644 application/assets/icon.png $ARTIFACT/opt/nyvoara/icon.png
install -m 0755 packaging/install.sh packaging/uninstall.sh $ARTIFACT/
install -m 0644 packaging/nyvoara.desktop $ARTIFACT/share/applications/
install -m 0644 packaging/nyvoara.png $ARTIFACT/share/icons/hicolor/128x128/apps/nyvoara.png
install -m 0644 packaging/io.github.rakmahefa.nyvoara.policy $ARTIFACT/
install -m 0644 packaging/README.md $ARTIFACT/README.md
tar czf $ARTIFACT.tar.gz $ARTIFACT
```

## Notes plateforme

- le binaire est lié dynamiquement à webkit2gtk-4.1, gtk3, libsoup3,
  javascriptcore, **libxdo3** et **libayatana-appindicator3-1** — toutes
  installées par `install.sh` ;
- si la libc de la distribution est trop ancienne pour le binaire fourni,
  `install.sh` bascule automatiquement sur une compilation depuis les
  sources (`git` + `rustup` requis) ;
- l'icône de notification exige un bus D-Bus de session : lancer depuis la
  session graphique. Sans bus (SSH…), l'app le détecte (poignée de main
  D-Bus validée au démarrage, `application/src/session.rs`) et le bouton
  ✕ **quitte** l'application au lieu de la masquer — jamais de fenêtre
  réduite injoignable.
