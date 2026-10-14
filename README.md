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
8. [La visualisation (MilkDrop)](#la-visualisation-milkdrop)
9. [Les plugins](#les-plugins)
10. [L'égaliseur](#légaliseur)
11. [Les raccourcis clavier](#les-raccourcis-clavier)
12. [Sur le téléphone](#sur-le-téléphone)
13. [Ça ne marche pas ?](#ça-ne-marche-pas-)
14. [Vie privée](#vie-privée)
15. [Pour les curieux et les développeurs](#pour-les-curieux-et-les-développeurs)
16. [English](#english)

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
- **Les visualisations façon MilkDrop** : des images qui dansent avec la musique, en fenêtre ou en plein écran,
  avec six visualisations d'Alpagamp et un pack de près de 10 000 à télécharger d'un clic.
- **Un navigateur de skins** : des dizaines de milliers de skins du Winamp Skin Museum, à essayer d'un clic.
- **Un égaliseur 10 bandes**, le fondu enchaîné entre deux morceaux, le volume harmonisé entre les morceaux
  (ReplayGain), des signets…

## Ce qui marche aujourd'hui

La version actuelle est la **0.6** : c'est une préversion. Elle marche, mais il manque encore des choses.

| Version | Ce qu'elle apporte | Où on en est |
|---|---|---|
| 0.1 | La fenêtre principale, la lecture de fichiers | fait |
| 0.2 | L'égaliseur, la playlist, les fenêtres aimantées, la recherche de morceau | fait |
| 0.3 | Les radios, le fondu enchaîné, ReplayGain, les signets, la première version Android | fait |
| 0.4 | Spotify : connexion, Bibliothèque, Spotify Connect | fait |
| 0.5 | Les visualisations façon MilkDrop (plein écran), le navigateur de skins, la taille des fenêtres | fait |
| 0.6 | Des extensions (plugins) qu'on peut ajouter, avec trois exemples | **version actuelle** |
| 0.7 | Android complet (musique qui continue écran éteint, commandes sur l'écran verrouillé), 7 langues partout | à venir |
| 1.0 | Version finale, testée sur Mac, Android et Windows | à venir |

Ce qu'il faut savoir avant d'installer :

- **Android** : pour l'instant, la musique s'arrête si tu quittes l'appli. Ça arrive en 0.7.
- **Mac** : le fichier marche sur les Mac à puce Apple (M1, M2, M3, M4…) et sur les Mac à processeur Intel
  (macOS 11 ou plus récent).
- **Visualisation** : il faut une carte graphique qui gère Vulkan, Metal (tous les Mac récents) ou DirectX 12
  (presque tous les PC depuis 2015). Sans elle, tout le reste marche.
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
| Mac (puce Apple ou Intel) | `Alpagamp-0.5.0.dmg` |
| PC Windows 10 ou 11 | `Alpagamp-0.5.0-windows-x64.zip` |
| Téléphone Android 8 ou plus récent | `Alpagamp-0.5.0.apk` |

(Le numéro change à chaque version : prends simplement les fichiers de la dernière.)

### Sur Mac

1. Double-clique sur `Alpagamp-0.5.0.dmg`. Une fenêtre s'ouvre avec l'icône d'Alpagamp et un dossier Applications.
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

1. Fais un **clic droit** sur `Alpagamp-0.5.0-windows-x64.zip` → **Extraire tout…** → **Extraire**.
2. Dans le dossier qui s'ouvre, double-clique sur **`alpagamp.exe`**.
3. La première fois, un écran bleu « Windows a protégé votre ordinateur » apparaît : clique sur
   **Informations complémentaires**, puis sur **Exécuter quand même**. (Même raison que sur Mac : l'appli n'est
   pas signée, Microsoft fait payer ça aussi.)

Il n'y a pas d'installation : tu peux ranger le dossier où tu veux (par exemple dans Documents). Pour l'avoir sous
la main, fais un clic droit sur `alpagamp.exe` → **Épingler à la barre des tâches**. Pour le désinstaller, supprime
simplement le dossier.

### Sur Android

1. Sur ton téléphone, ouvre la page des versions dans Chrome et touche **`Alpagamp-0.5.0.apk`** pour le télécharger.
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

**Trop petit ?** Clic droit → **Options** → **Taille des fenêtres** : ×1, ×2 ou ×3 (par défaut, Alpagamp choisit
selon ton écran). **Ctrl + D** (Cmd + D sur Mac) passe de ×1 à ×2.

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

Le plus simple : **Alt + S** (Option + S sur Mac), ou clic droit → **Skins** → **Navigateur de skins…**

- Onglet **Musée** : les skins du [Winamp Skin Museum](https://skins.webamp.org), avec un aperçu. Tape un nom dans
  la case (« zelda », « matrix », « metal »…) puis **Entrée** pour chercher. **Un clic sur un skin le télécharge et
  l'applique.** Fais défiler : la liste continue.
- Onglet **Mes skins** : ceux que tu as déjà, et le skin d'origine d'Alpagamp.

Tu as un fichier `.wsz` ? **Glisse-le sur Alpagamp**, ou bouton **Ouvrir…** du navigateur. Certains skins ont des
coins arrondis ou des formes bizarres, et leurs propres curseurs de souris : Alpagamp les respecte.

## La visualisation (MilkDrop)

Ouvre la fenêtre de **visualisation** : **Ctrl + Maj + K** (Cmd + Maj + K sur Mac), ou **double-clic** sur le petit
visualiseur de la fenêtre principale, ou clic droit → **Visualisation** → **Fenêtre de visualisation**. Des images
dansent avec la musique ; elles changent toutes seules toutes les 20 secondes, en fondu.

| Touche (fenêtre de visualisation) | Ce que ça fait |
|---|---|
| Double-clic, Entrée ou F11 | Plein écran (Échap pour en sortir) |
| Espace / H | Visualisation suivante (avec fondu / d'un coup) |
| Retour arrière | La précédente |
| L (ou Alt + K) | Choisir une visualisation dans la liste (avec une recherche) |
| R | Ordre au hasard ou dans l'ordre |
| K | Garder celle-ci (ne plus changer tout seul) |

Alpagamp en contient six. Pour en avoir près de **10 000** : clic droit dans la fenêtre → **Presets** →
**Télécharger le pack de presets** (*Cream of the Crop*, 30 Mo environ, une seule fois). Tu peux aussi ajouter tes
propres fichiers `.milk` : **Presets** → **Ouvrir le dossier des presets**, mets-les dedans, puis **Recharger les
presets**. Le changement automatique (de 10 secondes à 2 minutes) et la durée du fondu se règlent dans le même menu.

## Les plugins

Un **plugin**, c'est un petit programme qu'on ajoute au lecteur pour lui apprendre un truc nouveau. Alpagamp en
a trois, déjà installés mais éteints au départ. Clic droit → **Plugins** → le nom du plugin :

- **Barres plein écran** — un gros analyseur de spectre rétro, en petites diodes, aux couleurs de ton skin. Coche
  **Afficher dans la fenêtre de visualisation** : il prend la place des images MilkDrop (décoche pour les
  retrouver). Il marche aussi en plein écran. Réglages : nombre de barres, couleurs (skin, feu, glace,
  arc-en-ciel), crêtes, vitesse de chute.
- **Enhancer** — rend le son plus « gros » : stéréo plus large, basses plus profondes, un peu plus de clarté. Un
  limiteur empêche que ça sature. Chaque effet se règle (ou s'éteint) dans le menu du plugin.
- **Minuterie de sommeil** — pour s'endormir en musique : choisis la durée (5 min à 1 h 30), le fondu (le son baisse
  doucement à la fin) et ce qui se passe à la fin (arrêt ou pause). Le compte à rebours part quand tu l'actives, et
  le titre défilant te prévient à 5 minutes et à 1 minute de la fin. Elle s'éteint quand tu fermes Alpagamp.

Le ✓ devant un nom veut dire que le plugin est allumé. **À propos…** dit qui l'a fait et ce qu'il fait.

**Ajouter un plugin** : un plugin, c'est un fichier `.wasm`. **Glisse-le sur Alpagamp**, ou **Plugins** →
**Installer un plugin…** : il est copié dans le dossier des plugins et s'allume tout de suite.

**C'est sans danger ?** Un plugin tourne dans une boîte fermée (un « bac à sable ») : il ne peut ni lire tes
fichiers, ni aller sur Internet, ni bloquer le lecteur. S'il plante ou tourne en rond, Alpagamp l'arrête, l'éteint
et te le dit dans le titre défilant ; la musique continue. Installe quand même seulement des plugins de gens à
qui tu fais confiance.

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
| Ctrl + D | Double taille (toutes les fenêtres) |
| Ctrl + Maj + K | Fenêtre de visualisation |
| Alt + K | Choisir une visualisation |
| Alt + S | Navigateur de skins |
| Ctrl + W (Cmd + Maj + W sur Mac) | Replier la fenêtre en bandeau |
| Ctrl + T | Temps écoulé / temps restant |
| Ctrl + A | Toujours au premier plan |
| Maj + V / Ctrl + V | Arrêt en fondu / arrêter après ce morceau |
| Ctrl + Alt + B | Ajouter le morceau aux signets |
| Alt + 3 | Infos du fichier |
| Suppr / Alt + ↑ ↓ | Dans la playlist : retirer / déplacer les morceaux sélectionnés |

## Sur le téléphone

Les trois fenêtres sont empilées et agrandies à la largeur de l'écran ; tout se touche comme on clique.

- **Appui long n'importe où** : le menu (Bibliothèque, visualisation, skins, musique du téléphone, radios,
  options…).
- **Visualisation plein écran** : menu → **Visualisation plein écran**, ou double-tap sur le petit visualiseur.
  Glisse vers la gauche ou la droite pour changer d'image, double-tap pour revenir, appui long pour son menu.
- **Plugins** : menu → **Plugins**, comme sur l'ordinateur. *Barres plein écran* s'affiche dans la visualisation
  plein écran (et marche même sans carte graphique compatible).
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

**La visualisation ne s'ouvre pas** (« aucune carte graphique compatible ») → ta carte graphique ou son pilote
ne gère ni Vulkan, ni Metal, ni DirectX 12. Mets à jour le pilote de la carte graphique.

**Un plugin s'est éteint tout seul** (« … arrêté : … » dans le titre défilant) → il a planté ou mis trop de
temps à répondre ; Alpagamp l'a éteint pour protéger la musique. Tu peux le rallumer dans le menu **Plugins** ;
si ça recommence, c'est le plugin qui a un problème.

**Téléphone : l'écran s'affiche mal** → menu → **Options** → décoche **Affichage par la carte graphique**, puis
quitte et relance Alpagamp.

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
tu lui demandes : Spotify (si tu t'y connectes), l'annuaire des radios (radio-browser.info), les radios que tu
écoutes, le Winamp Skin Museum (quand tu ouvres le navigateur de skins) et GitHub (quand tu télécharges le pack
de visualisations). Tes réglages et ta connexion Spotify restent sur ton appareil.

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
cargo xtask plugins                      # recompile les plugins d'exemple (rustup target add wasm32-unknown-unknown)
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
| `crates/milkdrop` | La visualisation : moteur [lactea](https://github.com/M0Rf30/lactea), GPU (wgpu), presets |
| `crates/museum` | Le navigateur de skins (Winamp Skin Museum) |
| `crates/plugins` | Les plugins WebAssembly : bac à sable ([wasmi](https://github.com/wasmi-labs/wasmi)), catalogue, effets audio |
| `plugin-examples` | Le kit pour écrire un plugin en Rust, et les trois plugins livrés |
| `crates/player` | Le contrôleur de lecture commun |
| `app/desktop` | L'appli Mac, Windows et Linux |
| `app/android` | L'appli Android |
| `xtask` | L'outillage (paquets, APK, icônes, versions) |

Les choix importants sont expliqués dans [`docs/adr`](docs/adr/README.md), le vocabulaire dans
[`GLOSSARY.md`](GLOSSARY.md), les changements dans [`CHANGELOG.md`](CHANGELOG.md).

### Écrire un plugin

Un plugin est un module WebAssembly qui suit l'[API des plugins](docs/adr/0020-api-des-plugins.md) : trois
sortes (visualisation, effet audio, plugin général), une description en JSON (nom, réglages, traductions), et
quelques fonctions que le lecteur appelle. Le plus simple : copier un des exemples de `plugin-examples`, qui
utilisent le petit kit `alpagamp-plugin` :

```rust
use alpagamp_plugin::{General, Plugin, host};

struct Bonjour;
impl Plugin for Bonjour {
    const INFO: &'static str = r#"{"name": "Bonjour", "kind": "general", "author": "Moi", "version": "1.0"}"#;
    fn new() -> Self {
        host::notify("Bonjour !");
        Bonjour
    }
}
impl General for Bonjour {
    fn tick(&mut self, _ms: i64) {}
}
alpagamp_plugin::general_plugin!(Bonjour);
```

Puis `cargo build --release --target wasm32-unknown-unknown` et glisser le `.wasm` obtenu sur Alpagamp. N'importe
quel langage qui produit du WebAssembly (C, Zig, AssemblyScript…) convient, tant qu'il respecte l'API.

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
Connect). It shows MilkDrop-style visualizations (window or full screen, ~10,000 presets downloadable in one
click), has a built-in skin browser for the Winamp Skin Museum, and runs sandboxed WebAssembly plugins (three
included: a full-screen LED spectrum analyzer, a stereo/bass enhancer and a sleep timer). It runs on **macOS**, **Windows** and **Android**. Free, no ads, no tracking, MIT licensed. Not
affiliated with Winamp or Nullsoft.

**Install:** download the file for your device from the
[latest release](https://github.com/Guiwald/alpagamp/releases/latest):

- **Mac** (Apple silicon and Intel): open the `.dmg`, drag Alpagamp to Applications, then right-click it → Open →
  Open (the app isn't notarized). If macOS still refuses: System Settings → Privacy & Security → Open Anyway.
- **Windows 10/11**: extract the `.zip`, run `alpagamp.exe`; on the SmartScreen warning click "More info" →
  "Run anyway".
- **Android 8+**: download the `.apk` on the phone, allow installing unknown apps for your browser, install, and
  allow music access on first launch.

**Use:** drag files or folders onto the main window (play) or the playlist (add); right-click anywhere for the
full menu; Alt+L opens the Library (Spotify, radios); drop a `.wsz` skin (from <https://skins.webamp.org>) to
change the look, or browse skins with Alt+S. Ctrl+Shift+K opens the visualization (double-click for full screen).
Options › Window size sets ×1/×2/×3. Right-click › Plugins turns plugins on and sets their options; drop a
`.wasm` file to install one. Classic shortcuts: Z X C V B for previous/play/pause/stop/next.

**Build from source:** install Rust (<https://rustup.rs>), then `git clone https://github.com/Guiwald/alpagamp`,
`cd alpagamp`, `cargo run --release` (on Linux, first `sudo apt install libasound2-dev pkg-config`).

## Licence

MIT — voir [`LICENSE`](LICENSE). Les composants tiers et leurs licences sont listés dans
[`THIRD_PARTY.md`](THIRD_PARTY.md).
