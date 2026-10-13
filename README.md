# Alpagamp

**Un lecteur de musique qui a le look des années 2000, refait de zéro en Rust.**
Tu glisses tes chansons dessus, il les joue — avec l'égaliseur, la playlist, les skins rétro… Il sait aussi jouer
ta musique **Spotify** et des milliers de **radios**. Il marche sur **Mac**, **Windows** et **Android**.

C'est gratuit, sans pub, sans compte à créer, et tout le code est ici, lisible par tout le monde.

*English speakers: jump to the [English section](#english) at the bottom.*

> Alpagamp n'a aucun lien avec Winamp, Nullsoft ou leurs propriétaires. Il s'en inspire, mais n'utilise ni leur code
> ni leurs images : tout a été réécrit, et son apparence par défaut est originale
> ([pourquoi et comment](docs/adr/0001-reimplementation-propre.md)).

## Sommaire

1. [C'est quoi, exactement ?](#cest-quoi-exactement-)
2. [Ce qui marche aujourd'hui](#ce-qui-marche-aujourdhui)
3. [Installer](#installer) — [Mac](#sur-mac) · [Windows](#sur-windows) · [Android](#sur-android) · [Linux](#sur-linux)
4. [Premiers pas](#premiers-pas)
5. [Spotify](#spotify)
6. [Les radios](#les-radios)
7. [Changer de skin](#changer-de-skin)
8. [L'égaliseur](#légaliseur)
9. [Les raccourcis clavier](#les-raccourcis-clavier)
10. [Sur le téléphone](#sur-le-téléphone)
11. [Ça ne marche pas ?](#ça-ne-marche-pas-)
12. [Vie privée](#vie-privée)
13. [Pour les curieux et les développeurs](#pour-les-curieux-et-les-développeurs)
14. [English](#english)

---

## C'est quoi, exactement ?

Au début des années 2000, le lecteur de musique le plus populaire sur ordinateur ressemblait à une petite chaîne
hi-fi : trois petites fenêtres qu'on collait ensemble (le lecteur, l'égaliseur, la liste des morceaux), des
**skins** pour lui donner n'importe quelle apparence, et des visualisations qui dansaient avec la musique.

Alpagamp, c'est cette expérience, refaite aujourd'hui :

- **Les trois fenêtres** qui s'aimantent entre elles, qu'on peut replier en « bandeau » ou afficher en double taille.
- **Les skins classiques** (fichiers `.wsz`) : il en existe des dizaines de milliers, gratuits.
- **Tes fichiers audio** : MP3, FLAC, OGG, AAC/M4A, WAV, ALAC…
- **Les radios internet** : des milliers, avec une recherche par nom ou par style.
- **Spotify** : tes playlists, tes titres likés, tes albums, la recherche — et tu peux même choisir Alpagamp comme
  « enceinte » depuis l'appli Spotify de ton téléphone. Il faut un compte **Spotify Premium**.
- **Un égaliseur 10 bandes**, le fondu enchaîné entre deux morceaux, le volume harmonisé entre les morceaux
  (ReplayGain), des signets…

## Ce qui marche aujourd'hui

La version actuelle est la **0.4** : c'est une préversion. Elle marche, mais il manque encore des choses.

| Version | Ce qu'elle apporte | Où on en est |
|---|---|---|
| 0.1 | La fenêtre principale, la lecture de fichiers | fait |
| 0.2 | L'égaliseur, la playlist, les fenêtres aimantées, la recherche de morceau | fait |
| 0.3 | Les radios, le fondu enchaîné, ReplayGain, les signets, la première version Android | fait |
| 0.4 | Spotify : connexion, Bibliothèque, Spotify Connect | **version actuelle** |
| 0.5 | Les visualisations façon MilkDrop (plein écran), un navigateur de skins intégré | en cours |
| 0.6 | Des extensions (plugins) qu'on peut ajouter, avec trois exemples | à venir |
| 0.7 | Android complet (musique qui continue écran éteint, commandes sur l'écran verrouillé), 7 langues partout | à venir |
| 1.0 | Version finale, testée sur Mac, Android et Windows | à venir |

Ce qu'il faut savoir avant d'installer :

- **Android** : pour l'instant, la musique s'arrête si tu quittes l'appli. Ça arrive en 0.7.
- **Mac** : le fichier de la version 0.4.0 marche sur les Mac à puce Apple (M1, M2, M3, M4…). Pour un Mac à
  processeur Intel, attends la 0.5 (elle marchera sur les deux) ou [compile-le toi-même](#pour-les-curieux-et-les-développeurs).
- **iPhone** : pas encore, c'est prévu plus tard.
- Les applis ne sont **pas signées** (Apple et Microsoft font payer ça chaque année). Ton ordinateur va donc
  afficher un avertissement la première fois : c'est normal, et on t'explique quoi faire juste en dessous.

---

## Installer

### Étape 1 : télécharger le bon fichier

Va sur la page des versions : **<https://github.com/Guiwald/alpagamp/releases/latest>**

Tout en bas de la version, dans **Assets**, prends le fichier qui correspond à ton appareil :

| Ton appareil | Le fichier à télécharger |
|---|---|
| Mac à puce Apple (M1, M2, M3, M4…) | `Alpagamp-0.4.0.dmg` |
| PC Windows 10 ou 11 | `Alpagamp-0.4.0-windows-x64.zip` |
| Téléphone Android 8 ou plus récent | `Alpagamp-0.4.0.apk` |

*Comment savoir si mon Mac a une puce Apple ?* Menu Pomme (la pomme en haut à gauche) → **À propos de ce Mac**. S'il y a
écrit « Puce Apple M… », c'est bon. S'il y a écrit « Processeur Intel », c'est un Mac Intel.

### Sur Mac

1. Double-clique sur `Alpagamp-0.4.0.dmg`. Une fenêtre s'ouvre avec l'icône d'Alpagamp et un dossier Applications.
2. **Glisse Alpagamp sur le dossier Applications.** C'est installé !
3. Ouvre ton dossier **Applications**, puis fais un **clic droit** (ou Ctrl + clic) sur Alpagamp → **Ouvrir** →
   encore **Ouvrir** dans la fenêtre qui apparaît. Il ne faut le faire qu'une seule fois : ensuite, Alpagamp
   s'ouvre normalement, depuis le Launchpad ou le Dock.

**Pourquoi cet avertissement ?** Apple demande aux développeurs de payer 99 $ par an pour « signer » leurs
applis. Alpagamp est un projet gratuit, il n'est donc pas signé. Ce n'est pas un virus : tout son code est sur
cette page.

**Si macOS refuse quand même** (message « Apple ne peut pas vérifier… » sans bouton Ouvrir) :
ouvre **Réglages Système** → **Confidentialité et sécurité**, descends tout en bas : il y a une ligne qui parle
d'Alpagamp avec un bouton **Ouvrir quand même**. Clique dessus et confirme avec ton mot de passe ou Touch ID.

**Si macOS dit que l'appli est « endommagée »** : ouvre l'appli **Terminal**, colle cette ligne puis appuie sur
Entrée, et rouvre Alpagamp :

```sh
xattr -dr com.apple.quarantine /Applications/Alpagamp.app
```

### Sur Windows

1. Fais un **clic droit** sur `Alpagamp-0.4.0-windows-x64.zip` → **Extraire tout…** → **Extraire**.
2. Dans le dossier qui s'ouvre, double-clique sur **`alpagamp.exe`**.
3. La première fois, un écran bleu « Windows a protégé votre ordinateur » apparaît : clique sur
   **Informations complémentaires**, puis sur **Exécuter quand même**. (Même raison que sur Mac : l'appli n'est
   pas signée, Microsoft fait payer ça aussi.)

Il n'y a pas d'installation : tu peux ranger le dossier où tu veux (par exemple dans Documents). Pour l'avoir sous
la main, fais un clic droit sur `alpagamp.exe` → **Épingler à la barre des tâches**. Pour le désinstaller, supprime
simplement le dossier.

### Sur Android

1. Sur ton téléphone, ouvre la page des versions dans Chrome et touche **`Alpagamp-0.4.0.apk`** pour le télécharger.
2. Ouvre le fichier téléchargé (la notification « Téléchargement terminé », ou l'appli **Fichiers** →
   **Téléchargements**).
3. Android te demande d'autoriser Chrome (ou Fichiers) à **installer des applis inconnues** : touche
   **Paramètres**, active l'interrupteur, puis reviens en arrière.
4. Touche **Installer**. Si Play Protect affiche un avertissement, touche **Plus de détails** → **Installer quand même**.
5. Au premier lancement, **autorise l'accès à la musique** : c'est ce qui permet à Alpagamp de trouver les morceaux
   rangés sur ton téléphone.

Pour mettre à jour plus tard, installe simplement le nouveau fichier `.apk` par-dessus : tes réglages restent.

### Sur Linux

Il n'y a pas encore de fichier tout prêt : il faut le compiler (c'est facile, voir
[Pour les curieux et les développeurs](#pour-les-curieux-et-les-développeurs)).

---

## Premiers pas

### Les trois fenêtres

- **La fenêtre principale** : l'afficheur (le temps, le titre qui défile, le petit visualiseur), les boutons
  Précédent, Lecture, Pause, Stop, Suivant et Éjecter (pour ouvrir des fichiers), le volume, la balance et la
  barre de position.
- **L'égaliseur** : pour régler les graves, les médiums et les aigus.
- **La playlist** : la liste des morceaux. Tu peux l'agrandir en tirant son coin en bas à droite.

Attrape une fenêtre par sa barre de titre pour la déplacer : elle **s'aimante** aux autres fenêtres et aux bords
de l'écran. Quand tu déplaces la fenêtre principale, les fenêtres collées à elle suivent. (Garde **Maj** appuyée
pour déplacer sans aimanter.) Double-clic sur une barre de titre : la fenêtre se replie en **bandeau**.

### Mettre de la musique

- **Glisse des fichiers ou des dossiers** sur la fenêtre principale : ils remplacent la playlist et la lecture
  commence. Glissés sur la playlist, ils s'ajoutent à la fin.
- Ou clique sur **Éjecter** (ou appuie sur **L**) pour choisir des fichiers ; **Maj + L** pour un dossier entier.
- Dans la playlist, **double-clique** sur un morceau pour le jouer. Appuie sur **J** pour chercher un morceau en
  tapant une partie de son nom.

### Le menu

**Clic droit n'importe où** sur Alpagamp : tout est dans ce menu (ouvrir, lecture, options, skins, Spotify,
visualisation, langue…). Sur Mac, on retrouve aussi ces menus dans la barre en haut de l'écran.

Alpagamp se règle dans ta langue tout seul ; pour en changer : clic droit → **Options** → **Langue**.

---

## Spotify

Il faut un compte **Spotify Premium** (c'est une condition de Spotify pour les lecteurs autres que les siens).

**Se connecter :**

1. Clic droit → **Spotify** → **Se connecter à Spotify…**
2. Ton navigateur s'ouvre sur le site de Spotify : connecte-toi et accepte.
3. Reviens dans Alpagamp : la fenêtre **Bibliothèque** (Alt + L) affiche tes playlists, tes titres likés, tes
   albums, et une recherche. Double-clic sur un élément pour le jouer, bouton **Ajouter** pour le mettre dans la
   playlist.

Ton mot de passe Spotify ne passe jamais par Alpagamp : tu le tapes sur le site de Spotify. Alpagamp garde
ensuite une autorisation sur ton ordinateur seulement, pour ne pas te redemander de te connecter. Pour l'effacer :
clic droit → Spotify → **Se déconnecter de Spotify**.

**Spotify Connect (Alpagamp comme enceinte) :** quand Alpagamp est ouvert et connecté, ouvre l'appli Spotify sur
ton téléphone, touche l'icône des appareils (en bas de l'écran de lecture) et choisis **Alpagamp (Mac)** (ou
**Alpagamp (PC)**). La musique sort de ton ordinateur, et les boutons d'Alpagamp pilotent Spotify.

*À savoir :* Alpagamp utilise [librespot](https://github.com/librespot-org/librespot), une bibliothèque libre qui
parle à Spotify comme le fait l'appli officielle. Spotify ne l'autorise pas officiellement ; le risque est faible,
mais il existe ([détails](docs/adr/0002-spotify-via-librespot.md)).

## Les radios

Ouvre la **Bibliothèque** (Alt + L) → onglet **Radios** : les radios les plus écoutées du monde s'affichent. Tape
un nom ou un style (« jazz », « FIP », « rock »…) puis **Entrée** pour chercher. Double-clic pour écouter.

Tu connais l'adresse d'une radio ? **Ctrl + L** (Cmd + L sur Mac) et colle-la.

## Changer de skin

1. Va sur le **Winamp Skin Museum** : <https://skins.webamp.org> (des dizaines de milliers de skins gratuits).
2. Choisis un skin et télécharge-le : c'est un fichier qui finit par `.wsz`.
3. **Glisse le fichier `.wsz` sur Alpagamp.** C'est tout ! (Ou clic droit → **Skins** → **Charger un skin…**)

Les skins que tu as chargés restent dans le menu **Skins**. Pour revenir à l'apparence d'origine : **Skins** →
**Skin par défaut**. (En version 0.5, un navigateur de skins sera intégré à Alpagamp.)

## L'égaliseur

- **ON** allume ou éteint l'égaliseur.
- Les 10 curseurs règlent les fréquences, des graves (à gauche) aux aigus (à droite). Le curseur tout à gauche
  (préampli) monte ou baisse tout d'un coup.
- **Presets** : des réglages tout faits (Rock, Pop, Classique…), ou tes propres réglages enregistrés.
- **Auto** : Alpagamp retient un réglage par morceau et le recharge tout seul.

## Les raccourcis clavier

Sur Mac, **Ctrl** devient **Cmd** et **Alt** devient **Option**.

| Touche | Ce que ça fait |
|---|---|
| Z, X, C, V, B | Précédent, Lecture, Pause, Stop, Suivant |
| ← → | Reculer / avancer de 5 secondes |
| ↑ ↓ | Monter / baisser le volume |
| L / Maj + L | Ouvrir des fichiers / un dossier |
| Ctrl + L | Ajouter une adresse (radio, lien Spotify) |
| J | Aller à un morceau (recherche dans la playlist) |
| S / R | Lecture aléatoire / répétition |
| Alt + G / Alt + E / Alt + L | Afficher l'égaliseur / la playlist / la Bibliothèque |
| Ctrl + D | Double taille |
| Ctrl + W (Cmd + Maj + W sur Mac) | Replier la fenêtre en bandeau |
| Ctrl + T | Temps écoulé / temps restant |
| Ctrl + A | Toujours au premier plan |
| Maj + V / Ctrl + V | Arrêt en fondu / arrêter après ce morceau |
| Ctrl + Alt + B | Ajouter le morceau aux signets |
| Alt + 3 | Infos du fichier |
| Suppr / Alt + ↑ ↓ | Dans la playlist : retirer / déplacer les morceaux sélectionnés |

## Sur le téléphone

Les trois fenêtres sont empilées et agrandies à la largeur de l'écran ; tout se touche comme on clique.

- **Appui long n'importe où** : le menu (Bibliothèque, musique du téléphone, radios, options…).
- **Double-tap sur un morceau** de la playlist : le jouer.
- **Glisser** dans la playlist : la faire défiler.
- Le bouton **Retour** ferme les menus.

---

## Ça ne marche pas ?

**Mac : « impossible d'ouvrir », « endommagée »** → voir [Sur Mac](#sur-mac) (clic droit → Ouvrir, ou
« Ouvrir quand même » dans les Réglages, ou la ligne à coller dans le Terminal).

**Windows : mon antivirus bloque `alpagamp.exe`** → c'est souvent le cas pour un programme récent et non signé
(on appelle ça un « faux positif »). Tu peux l'autoriser dans ton antivirus.

**Pas de son** → vérifie le volume d'Alpagamp (le curseur sous l'afficheur) et la sortie son de ton ordinateur.
Si tu as branché un casque après avoir lancé Alpagamp, relance-le.

**Spotify : « Spotify Premium requis »** → ton compte est gratuit : Spotify ne permet pas aux autres lecteurs
de jouer sa musique avec un compte gratuit.

**Spotify : Alpagamp n'apparaît pas dans les appareils** → vérifie qu'Alpagamp est ouvert et connecté avec le
**même compte** Spotify que ton téléphone, puis attends quelques secondes.

**Une radio ne joue pas** → certaines radios utilisent un format qu'Alpagamp ne lit pas encore (« HLS ») ; essaie
une autre radio.

**Android : « Application non installée »** → si tu avais une ancienne version installée autrement, désinstalle-la
d'abord.

**Je veux tout remettre à zéro** → ferme Alpagamp et supprime son dossier de réglages :
- Mac : `~/Library/Application Support/fr.Guiwald.Alpagamp`
- Windows : `%APPDATA%\Guiwald\Alpagamp`
- Linux : `~/.config/alpagamp`

**Un autre souci, une idée ?** Ouvre un ticket dans l'onglet
[Issues](https://github.com/Guiwald/alpagamp/issues) : explique ce que tu as fait, ce qui s'est passé, et sur quel
appareil.

## Vie privée

Alpagamp ne contient **ni publicité, ni pisteur, ni statistiques**. Il ne se connecte à internet que pour ce que
tu lui demandes : Spotify (si tu t'y connectes), l'annuaire des radios (radio-browser.info) et les radios que tu
écoutes. Tes réglages et ta connexion Spotify restent sur ton appareil.

---

## Pour les curieux et les développeurs

Alpagamp est écrit en [Rust](https://www.rust-lang.org/). Tout est fait main : le dessin des fenêtres au pixel
près, la lecture des skins, l'égaliseur, le client des radios… La lecture des formats audio vient de
[Symphonia](https://github.com/pdeljanov/Symphonia), Spotify de [librespot](https://github.com/librespot-org/librespot).

### Compiler toi-même

1. Installe Rust : <https://rustup.rs> (une ligne à copier dans le Terminal).
2. Sur Linux, installe aussi la bibliothèque son : `sudo apt install libasound2-dev pkg-config`.
3. Puis :

```sh
git clone https://github.com/Guiwald/alpagamp
cd alpagamp
cargo run --release
```

La première compilation prend quelques minutes.

### Les commandes utiles

```sh
cargo run --release                      # le lecteur
cargo test --workspace                   # les tests
cargo xtask render-windows apercu.png    # image des fenêtres (avec un .wsz en second argument)
cargo build --release && cargo xtask bundle-mac target/release/alpagamp           # .app et .dmg (sur macOS)
cargo build --release && cargo xtask dist-windows target/release/alpagamp.exe     # zip Windows (sur Windows)
cargo xtask apk                                                                   # APK (ANDROID_HOME, NDK)
cargo run -p alpagamp-android --bin phone-sim                                     # simulateur de téléphone
```

### Comment c'est rangé

| Dossier | Contenu |
|---|---|
| `crates/skin` | Lecture des skins `.wsz`, skin par défaut |
| `crates/core` | Playlist, formats de fichiers, réglages, langues |
| `crates/audio` | Décodage, radios, égaliseur, fondu enchaîné, sortie son |
| `crates/net` | Client HTTP/HTTPS |
| `crates/spotify` | Connexion Spotify, Bibliothèque, Spotify Connect, annuaire des radios |
| `crates/ui` | Les fenêtres, dessinées au pixel près |
| `crates/player` | Le contrôleur de lecture commun |
| `app/desktop` | L'appli Mac, Windows et Linux |
| `app/android` | L'appli Android |
| `xtask` | L'outillage (paquets, APK, icônes, versions) |

Les choix importants sont expliqués dans [`docs/adr`](docs/adr/README.md), le vocabulaire dans
[`GLOSSARY.md`](GLOSSARY.md), les changements dans [`CHANGELOG.md`](CHANGELOG.md).

### Compilation automatique et versions

À chaque envoi sur GitHub, [GitHub Actions](.github/workflows/ci.yml) compile Alpagamp pour Linux, Windows, macOS
et Android et lance les tests. Pour publier une version : `cargo xtask bump minor` (ou `patch`), compléter le
CHANGELOG, faire un commit, puis envoyer une étiquette `vX.Y.Z` (`git tag v0.5.0 && git push origin v0.5.0`) :
[le flux de publication](.github/workflows/release.yml) fabrique le `.dmg` (Mac universel), le `.zip` Windows et
l'`.apk`, et les met sur la page des versions.

---

## English

**Alpagamp is an old-school music player, rewritten from scratch in Rust.** It looks and feels like the classic
skinnable players of the 2000s: a main window, a 10-band equalizer and a playlist editor that snap together,
windowshade and double-size modes, and classic `.wsz` skins. It plays your local files (MP3, FLAC, OGG, AAC, WAV,
ALAC…), thousands of internet radios, and your **Spotify** music (Premium account required, including Spotify
Connect). It runs on **macOS**, **Windows** and **Android**. Free, no ads, no tracking, MIT licensed. Not
affiliated with Winamp or Nullsoft.

**Install:** download the file for your device from the
[latest release](https://github.com/Guiwald/alpagamp/releases/latest):

- **Mac** (Apple silicon for 0.4.0): open the `.dmg`, drag Alpagamp to Applications, then right-click it → Open →
  Open (the app isn't notarized). If macOS still refuses: System Settings → Privacy & Security → Open Anyway.
- **Windows 10/11**: extract the `.zip`, run `alpagamp.exe`; on the SmartScreen warning click "More info" →
  "Run anyway".
- **Android 8+**: download the `.apk` on the phone, allow installing unknown apps for your browser, install, and
  allow music access on first launch.

**Use:** drag files or folders onto the main window (play) or the playlist (add); right-click anywhere for the
full menu; Alt+L opens the Library (Spotify, radios); drop a `.wsz` skin (from <https://skins.webamp.org>) to
change the look. Classic shortcuts: Z X C V B for previous/play/pause/stop/next.

**Build from source:** install Rust (<https://rustup.rs>), then `git clone https://github.com/Guiwald/alpagamp`,
`cd alpagamp`, `cargo run --release` (on Linux, first `sudo apt install libasound2-dev pkg-config`).

## Licence

MIT — voir [`LICENSE`](LICENSE). Les composants tiers et leurs licences sont listés dans
[`THIRD_PARTY.md`](THIRD_PARTY.md).
