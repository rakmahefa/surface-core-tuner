# Packaging `/opt/nyvoara`

Artefact d'installation système — prêt à copier-coller sur la machine cible.

```
packaging/
├── install.sh       sudo bash install.sh   (dépendances + /opt/nyvoara + menu + icône)
├── uninstall.sh     sudo bash uninstall.sh
├── nyvoara.desktop  entrée de menu (installée dans /usr/share/applications)
└── nyvoara.png      icône 128×128 (installée dans hicolor)
```

L'installateur s'attend à trouver le binaire à côté : `opt/nyvoara/nyvoara`
(cf. structure ci-dessous). Le workflow GitHub Actions peut déposer le
binaire `target/release/nyvoara` à cet emplacement avant publication.

```
nyvoara/
├── install.sh
├── uninstall.sh
├── opt/nyvoara/{nyvoara, icon.png}
└── share/{applications/nyvoara.desktop, icons/hicolor/128x128/apps/nyvoara.png}
```

## Construction de l'artefact

```bash
cargo build --release -p nyvoara
mkdir -p nyvoara/opt/nyvoara nyvoara/share/applications nyvoara/share/icons/hicolor/128x128/apps
cp target/release/nyvoara nyvoara/opt/nyvoara/
cp application/assets/icon.png nyvoara/opt/nyvoara/icon.png
cp packaging/install.sh packaging/uninstall.sh nyvoara/
cp packaging/nyvoara.desktop nyvoara/share/applications/
cp packaging/nyvoara.png nyvoara/share/icons/hicolor/128x128/apps/
tar czf nyvoara-linux-x86_64.tar.gz nyvoara
```

## Notes plateforme

- le binaire est lié dynamiquement à webkit2gtk-4.1, gtk3, libsoup3,
  javascriptcore, **libxdo3** et **libayatana-appindicator3-1** — toutes
  installées par `install.sh` ;
- si la libc de la distribution est trop ancienne pour le binaire fourni,
  `install.sh` bascule automatiquement sur une compilation depuis les
  sources (`git` + `rustup` requis) ;
- l'icône de notification exige un bus D-Bus de session : lancer depuis la
  session graphique (l'app retrouve le bus même sous `sudo`, via
  `SUDO_UID`/`PKEXEC_UID` — voir `application/src/session.rs`).
