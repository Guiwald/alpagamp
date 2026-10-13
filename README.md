# Alpagamp

Un lecteur de musique à l'ancienne, écrit en Rust : l'allure et les raccourcis du lecteur à skins des années
2000, ses skins classiques `.wsz`, ses visualisations, et, à terme, votre musique Spotify.

*An old-school music player written in Rust: classic `.wsz` skins, the familiar three windows and shortcuts,
visualizations, and soon your Spotify music. English summary below.*

> Alpagamp n'est ni affilié ni lié à Winamp, Nullsoft, Radionomy ou Llama Group. Il ne contient aucun code ni aucune
> image de Winamp ; son skin par défaut est original (ADR 0001).

## État

Préversion **0.4** : fenêtre principale, égaliseur, playlist, fenêtres aimantées, skins classiques, fichiers
locaux et radios, fondu enchaîné, ReplayGain, Android, Spotify (Bibliothèque, Spotify Connect). Feuille de route vers la 1.0 :

| Version | Contenu |
|---|---|
| 0.1 | Fenêtre principale, lecture locale (Windows) |
| 0.2 | Égaliseur, playlist, aimantation, bandeau, double taille, « Aller au morceau » |
| 0.3 | Radios, fondu enchaîné, ReplayGain, signets, infos de fichier, première version Android |
| 0.4 | Spotify (connexion, Bibliothèque, Spotify Connect) — Spotify Premium requis |
| 0.5 | MilkDrop, presets, navigateur de skins |
| 0.6 | Plugins WebAssembly et trois plugins d'exemple |
| 0.7 | Android complet, 7 langues partout, barre de menus macOS |
| 1.0 | Testée sur Mac, Pixel 8 et Windows |

Les décisions sont expliquées dans [`docs/adr`](docs/adr/README.md), le vocabulaire dans
[`GLOSSARY.md`](GLOSSARY.md), les changements dans [`CHANGELOG.md`](CHANGELOG.md).

## Installer

- **Windows** : télécharger `Alpagamp-X.Y.Z-windows-x64.zip` dans les releases, décompresser, lancer
  `alpagamp.exe`. Au premier lancement, Windows avertit (application non signée) : « Informations
  complémentaires » puis « Exécuter quand même ».
- **macOS** : `Alpagamp-X.Y.Z.dmg` (en attendant la compilation automatique, `scripts/construire-mac.sh` le
  construit sur un Mac). Application non signée : clic droit sur l'application, « Ouvrir », puis « Ouvrir ».
- **Android** (Pixel 8 et Android 8 ou plus récent, processeur 64 bits) : ouvrir `Alpagamp-X.Y.Z.apk` sur le
  téléphone, autoriser l'installation depuis cette source, installer. Au premier lancement, autoriser l'accès à la
  musique. Appui long n'importe où : le menu ; double-tap sur un morceau : le jouer.

## Utiliser

Glisser des fichiers ou des dossiers sur la fenêtre principale (lecture) ou sur la playlist (ajout). Glisser un
`.wsz` change de skin. Clic droit : le menu complet.

| Touche | Action |
|---|---|
| Z X C V B | Précédent, Lecture, Pause, Stop, Suivant |
| ← → / ↑ ↓ | Reculer, avancer de 5 s / volume |
| L, Maj+L | Ouvrir des fichiers, un dossier |
| J | Aller au morceau (recherche dans la playlist) |
| Alt+G, Alt+E | Afficher l'égaliseur, la playlist |
| Ctrl+D, Ctrl+W | Double taille, bandeau |
| S, R | Aléatoire, répétition |
| Suppr, Alt+↑ ↓ | Retirer, déplacer la sélection (playlist) |

Sur Mac, Ctrl devient Cmd et Alt devient Option.

## Compiler

Rust stable récent (édition 2024).

```sh
cargo run --release                      # le lecteur
cargo test --workspace                   # les tests
cargo xtask render-windows apercu.png    # aperçu des fenêtres (avec un .wsz en second argument)
cargo build --release --target x86_64-pc-windows-gnu && cargo xtask dist-windows   # paquet Windows (mingw-w64)
cargo build --release && cargo xtask bundle-mac target/release/alpagamp           # .app et .dmg (sur macOS)
cargo xtask apk                                                                   # APK (ANDROID_HOME, NDK)
cargo run -p alpagamp-android --bin phone-sim                                     # simulateur de téléphone
```

Organisation : `crates/skin` (skins), `crates/core` (playlist, formats, réglages, langues), `crates/audio` (décodage,
radios, égaliseur, sortie), `crates/ui` (fenêtres dessinées au pixel près), `crates/player` (contrôleur de lecture
commun), `app/desktop` (Mac, Windows, Linux), `app/android` (téléphone), `xtask` (outillage).

## English

Alpagamp is a from-scratch, clean-room music player in Rust that looks and behaves like the classic three-window
skinnable players: main window, 10-band equalizer, playlist editor, snapping windows, windowshade and double-size
modes, classic `.wsz` skins, built-in spectrum analyzer and oscilloscope. Spotify (Premium, via librespot),
MilkDrop presets and WebAssembly plugins are on the roadmap. Targets: macOS, Android, Windows. MIT licensed.
Not affiliated with Winamp or Nullsoft.

## Licence

MIT — voir [`LICENSE`](LICENSE) et les composants tiers dans [`THIRD_PARTY.md`](THIRD_PARTY.md).
