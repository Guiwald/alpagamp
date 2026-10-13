# Winamp classique — référence technique

*Deep dive du 13/10/2026, pour le clone en Rust. Trois recherches parallèles, recoupées entre elles. Sources principales : le code source de Winamp 5.9 (publié en 2024, lu pour les faits seulement), le code de Webamp (MIT), le « Skinner's Atlas 1.5 », le skin de base `base-2.91.wsz`, le SDK de plugins, le code de MilkDrop 2 (BSD), AVS (BSD), projectM, Audacious.*

---

## 0. L'essentiel en 12 points

1. **Winamp, c'est trois fenêtres en pixel art** : principale (275×116), égaliseur (275×116), éditeur de playlist (redimensionnable par pas de 25×29 px). Chacune a un mode « windowshade » (bandeau de 14 px). Elles s'aimantent entre elles (10 px) et la principale entraîne les autres quand on la déplace. Mode double taille (×2 au plus proche voisin).
2. **Un skin classique (.wsz) = un zip de ~20 BMP + 3 fichiers texte.** Chaque BMP est une planche de sprites à des coordonnées fixes. Tout fichier manquant retombe sur le skin de base. C'est un format simple, entièrement documenté (Webamp le rend au pixel près dans un navigateur). Plus de 70 000 skins existent (Winamp Skin Museum).
3. **Les skins « modernes » (.wal, Winamp 3/5)** sont du XML + un bytecode de script (MAKI) avec ~65 classes : même Webamp a abandonné. À exclure.
4. **Les « mods » de Winamp étaient des DLL Windows 32 bits** (`in_`, `out_`, `vis_`, `dsp_`, `gen_`…). **Impossible de les charger sur Mac, Android, iPhone ou ARM**, et même sur Windows 64 bits il faudrait un processus-pont 32 bits.
5. **Ce qui est portable, ce sont les formats de données** : les skins `.wsz`, les presets **MilkDrop** `.milk` (texte + code + shaders), les presets **AVS** `.avs` (binaire), les presets d'égaliseur `.eqf`, les playlists M3U/PLS.
6. **MilkDrop** (Ryan Geiss, 2001 ; v2 en 2007 avec shaders) : la visualisation star. Code source BSD. Réimplémentations : projectM (C++, LGPL, OpenGL), Butterchurn (WebGL, MIT, utilisé par Webamp), et **lactea**, un moteur pur Rust/wgpu publié… il y a deux jours (non éprouvé). ~10 000 presets dans le pack « Cream of the Crop ».
7. **Les données envoyées aux visualisations** : à chaque trame, 2 canaux × 576 octets de spectre + 2 × 576 octets de forme d'onde (8 bits). MilkDrop recalcule sa propre FFT et en tire `bass/mid/treb`.
8. **Visualiseur intégré** (76×16 px dans la fenêtre principale) : analyseur de spectre (19 barres de 3 px ou 75 barres fines, chute et crêtes réglables, 3 styles de couleur) ou oscilloscope (points / lignes / plein). Un clic fait tourner Analyseur → Oscilloscope → Éteint. Les 24 couleurs viennent du skin (`viscolor.txt`).
9. **Égaliseur 10 bandes, ±12 dB**, libellés 60/170/310/600/1k/3k/6k/12k/14k/16k (Winamp 5 filtre en réalité à 70/180/320/600/1k/3k/6k/12k/14k/16k), préampli, boutons ON/AUTO/PRESETS, 18 presets d'usine, format de fichier `.eqf` documenté, chargement automatique d'un preset par fichier.
10. **Playlist** : boutons ADD / REM / SEL / MISC / LIST avec menus en bande, tri, aléatoire, « Jump to file » (touche J, recherche multi-mots), M3U/M3U8/PLS, temps total `sélection/total`.
11. **Radio Internet SHOUTcast** (1999) : protocole ICY = HTTP + métadonnées insérées dans le flux tous les N octets (`StreamTitle='Artiste - Titre';`). Toujours d'actualité.
12. **Juridique** : la marque « Winamp » et le lama appartiennent à Winamp Group SA (ex-Llama Group). Le code source publié en 2024 est sous une licence qui interdit de redistribuer des versions modifiées : **on s'en sert pour lire des chiffres, jamais pour copier du code**. Les bitmaps du skin de base sont à eux aussi : **le clone doit avoir son propre nom et son propre skin par défaut**, et charge les skins que l'utilisateur lui donne.

---

## 1. Histoire en bref

| Date | Événement |
|---|---|
| 21/04/1997 | WinAMP 0.20a (Justin Frankel) |
| 1998 | 1.80 : premiers skins · Nullsoft fondée, shareware à 10 $ |
| 08/09/1998 | Winamp 2.0 |
| 1999 | SHOUTcast · AOL rachète Nullsoft (~80 M$) · 2.50 devient gratuit |
| 2000 | AVS (2.61) |
| 2001 | MilkDrop 1.0 (Ryan Geiss), plugin tiers |
| 09/08/2002 | Winamp3 (framework Wasabi, skins modernes) — échec |
| 2003 | 2.90/2.91 (Media Library, nouvel EQ) · déc. : **Winamp 5** (= 2 + 3, classique + moderne) |
| 2005–2006 | 5.2 (gapless MP3), 5.3 (ReplayGain, 24 bits) · AVS et MilkDrop 1.04 open source (BSD) |
| 10/10/2007 | 5.5 : MilkDrop 2 intégré (shaders) |
| 11/2013 | 5.66 / 5.666, AOL annonce la fermeture |
| 14/01/2014 | Radionomy rachète Winamp et SHOUTcast |
| 2018 | 5.8 · 2022 : 5.9 · 2023 : 5.9.2 |
| 24/09/2024 | Publication du code source (licence restrictive « WCL ») — dépôt supprimé mi-octobre 2024 |
| 23/10/2025 | Llama Group devient **Winamp Group SA** (propriétaire de la marque) |
| 2024–2027 | App mobile « Winamp » officielle : lecteur générique, **sans skins classiques** ; partenariat Deezer annoncé pour 2027 |

---

## 2. Les fenêtres

| Fenêtre | Taille | Windowshade | Particularités |
|---|---|---|---|
| Principale | 275×116 | 275×14 | double-clic sur la barre de titre (x<244) ou bouton shade |
| Égaliseur | 275×116 | 275×14 | graphismes du shade dans `eq_ex.bmp` |
| Playlist | min 275×116, largeur 275+25·n, hauteur 116+29·m | hauteur 14, largeur toujours redimensionnable | lignes à partir de y=22 ; mini-visualiseur 75×38 en bas à droite si la principale est fermée et la playlist ≥ 350 px |
| Minibrowser | (2.x) | | `mb.bmp` — à ignorer |
| Vidéo, AVS, fenêtres génériques | | | `video.bmp`, `avs.bmp`, `gen.bmp` — génériques utiles pour héberger une visualisation |

- **Double taille** : principale (et EQ si option, défaut oui), pas la playlist. ×2 au plus proche voisin, coordonnées souris divisées par 2. Exception : le visualiseur se dessine nativement en 152×32.
- **Toujours au premier plan** : lettre A de la clutterbar.
- **Aimantation** : 10 px par défaut (Shift inverse), bord à bord avec les autres fenêtres et les bords d'écran. Deux fenêtres sont « attachées » si leurs bords se touchent exactement. Déplacer la principale entraîne toutes les fenêtres attachées (transitivement) ; déplacer l'EQ ou la playlist ne bouge qu'elle. On déplace une fenêtre en la saisissant n'importe où hors contrôle (« EasyMove »).

---

## 3. Fenêtre principale : carte des éléments

Coordonnées en pixels à l'échelle 1, R(x,y,l,h) = rectangle à l'écran.

| Élément | Où | Sprite | Comportement |
|---|---|---|---|
| Barre de titre | (0,0,275,14) | `titlebar.bmp` x=27 ; y=0 active, 15 inactive, 29/42 shade | easter egg en tapant NULLSOFT |
| Bouton menu | (6,3,9,9) | titlebar (0,0)/(0,9) | double-clic = quitter |
| Réduire / Shade / Fermer | (244,3), (254,3), (264,3), 9×9 | titlebar | |
| **Clutterbar** O A I D V | (10,22,8,43) | titlebar x≥304 | **O** menu options · **A** toujours devant · **I** infos fichier · **D** double taille · **V** menu visualisation |
| Icône lecture/pause/stop | (26,28,9,9) | `playpaus.bmp` | + colonne d'état 3 px (synchro / mise en mémoire tampon) |
| **Temps** | chiffres 9×13 à x=48,60 (min) et 78,90 (s), y=26 | `nums_ex.bmp` (sinon `numbers.bmp`) | clic = écoulé ↔ restant (signe moins) ; masqué à l'arrêt ; clignote en pause |
| **Visualiseur** | (24,43,76,16) | couleurs `viscolor.txt` | voir §6 |
| **Titre défilant** | x 111→264 (154 px), y=27 | `text.bmp` | « N. Titre (m:ss) » ; défile de 5 px toutes les 200 ms avec séparateur `  ***  ` ; déplaçable à la souris |
| kbps / kHz | (111,43) 3 car. / (156,43) 2 car. | `text.bmp` | |
| Mono / Stéréo | (212,41,27,12) / (239,41,29,12) | `monoster.bmp` | |
| **Volume** | (107,57,68,13) | `volume.bmp`, 28 fonds de 15 px | valeur 0–255, défaut 200 ; curseur 14×11 |
| **Balance** | (177,57,38,13) | `balance.bmp` | −127…127, aimanté au centre ±9 |
| Boutons EQ / PL | (219,58,23,12) / (242,58,23,12) | `shufrep.bmp` | ouvrent/ferment les fenêtres |
| **Barre de position** | (16,72,248,10) | `posbar.bmp`, curseur 29×10 | masquée si non seekable (radio) ; le saut se fait au relâchement |
| **Transport** | y=88, h=18 : préc. x16, lecture x39, pause x62, stop x85, suiv. x108 | `cbuttons.bmp` | Shift/Ctrl changent l'action (voir §9) |
| Éjecter | (136,89,22,16) | cbuttons | ouvrir fichier (Shift : dossier, Ctrl : URL) |
| Aléatoire / Répéter | (164,89,47,15) / (210,89,28,15) | `shufrep.bmp` | |
| Logo | zone (253,91)-(265,105) | dans `main.bmp` | « À propos » |

**Mode shade de la principale** (275×14) : mini-visualiseur 38×5, temps en petite police, zones de transport invisibles, mini barre de position 17×7.

**Égaliseur** : boutons ON (14,18), AUTO (39,18), PRESETS (217,18) ; petit graphe de la courbe (86,17,113,19) ; curseurs 14×63 à y=38 : préampli x=21, bandes x=78+18·i ; 28 positions graphiques par curseur.

---

## 4. Le format de skin classique (.wsz)

### Le conteneur
- Un `.wsz` est un **zip** (Winamp accepte aussi `.zip` et un simple dossier).
- S'il contient un `skin.xml`, c'est un skin **moderne** → autre moteur (à refuser chez nous).
- On ne garde que les `.bmp`, `.txt`, `.cur`, `.ini` ; **les sous-dossiers sont aplatis**, **la casse est ignorée**, et **la dernière entrée du zip gagne** en cas de doublon.
- **Tout fichier manquant ou illisible retombe sur l'équivalent du skin de base**, fichier par fichier. Cas particuliers : `nums_ex.bmp` prioritaire sur `numbers.bmp` ; pas de `balance.bmp` → on prend `volume.bmp`.
- **BMP** : le skin de base est en 8 bits, la plupart **compressés RLE8**. Le décodeur doit gérer RLE8, RLE4, 1/4/8/24 bits, de haut en bas et de bas en haut. Winamp classique ne lit jamais de PNG.

### Les fichiers

| Fichier | Taille | Contenu |
|---|---|---|
| `main.bmp` | 275×116 | fond de la fenêtre principale |
| `titlebar.bmp` | 344×87 | barres de titre, boutons de fenêtre, clutterbar, mini barre de position |
| `cbuttons.bmp` | 136×36 | boutons de transport (ligne normale / enfoncée) + éjecter |
| `shufrep.bmp` | 92×85 | répéter, aléatoire, boutons EQ/PL |
| `posbar.bmp` | 307×10 | fond de la barre de position + curseurs |
| `volume.bmp` | 68×433 | 28 fonds + curseurs |
| `balance.bmp` | 47 ou 68×433 | idem |
| `monoster.bmp` | 56×24 | mono / stéréo, allumé / éteint |
| `playpaus.bmp` | 42×9 | icônes d'état |
| `numbers.bmp` / `nums_ex.bmp` | 99×13 / 108×13 | chiffres 9×13 (+ blanc et moins pour nums_ex) |
| `text.bmp` | 155×18 utiles | police 5×6 |
| `eqmain.bmp` | 275×315 | toute la fenêtre EQ |
| `eq_ex.bmp` | 275×56 | EQ en mode shade |
| `pledit.bmp` | 280×186 | morceaux de la playlist (coins, tuiles, barre de défilement, boutons de menu) |
| `gen.bmp` + `genex.bmp` | ~190×105 / ~112×59 | fenêtres génériques (police du titre à largeur variable ; couleurs sur la ligne y=0 de genex) |
| `video.bmp`, `mb.bmp`, `avs.bmp` | | fenêtres vidéo, minibrowser, AVS |
| `*.cur` (24 utilisés) | | curseurs par zone (volume, barre de position, titre, playlist…) — parfois des curseurs animés |
| `readme.txt` | | affiché par le navigateur de skins |

**`viscolor.txt`** — 24 lignes `R,G,B` (commentaire `//` permis) :
- 0 = fond, 1 = points de la grille ;
- 2 à 17 = dégradé de l'analyseur (2 en haut, 17 en bas) ;
- 18 à 22 = oscilloscope (18 au centre, plus loin vers les bords) ;
- 23 = crêtes de l'analyseur.
- Winamp **arrête de lire à la première ligne qui ne commence pas par un nombre** (le reste garde les valeurs par défaut) ; Webamp est plus tolérant.

**`pledit.txt`** — INI, section `[Text]` : `Normal`, `Current`, `NormalBG`, `SelectedBG` (couleurs `#RRGGBB`), `Font` (nom de police système), `mbFG`/`mbBG`. Défauts : vert #00FF00 / blanc / noir / bleu #0000C6, Arial.

**`region.txt`** — fenêtres non rectangulaires : sections `[Normal]`, `[WindowShade]`, `[Equalizer]`, `[EqualizerWS]` (pas la playlist), `NumPoints=` (sommets par polygone) et `PointList=` (x,y à plat). Si les comptes ne collent pas, la section est ignorée.

### Le texte bitmap (`text.bmp`)
- Glyphes 5×6, grille 31×3 : ligne 0 = A–Z, `"`, `@`, espace (col. 30) ; ligne 1 = 0–9, `…`, puis `. : ( ) - ' ! _ + \ / [ ] ^ & % , = $ #` ; ligne 2 = `Å Ö Ä ? *`.
- Minuscules → majuscules ; accents ramenés à la lettre de base (é→E) ; **tout le reste s'affiche comme une espace**.
- La playlist, elle, utilise une vraie police système (`Font` de pledit.txt).

---

## 5. Skins modernes (.wal) — pourquoi on ne les fait pas

Zip contenant `skin.xml` (racine `WinampAbstractionLayer`), des PNG, des polices, et des scripts **MAKI compilés** (bytecode maison, 7 sections sans en-têtes). Les supporter = un moteur de mise en page XML + une machine virtuelle MAKI + ~65 classes de bibliothèque standard dont le comportement exact n'est documenté nulle part. Webamp a fait une preuve de concept (« webamp-modern ») qui fait tourner quelques dizaines de skins, puis l'a abandonnée. **Recommandation : classiques uniquement.**

---

## 6. Le visualiseur intégré

- Zone 76×16 (75 colonnes dessinées). Fond couleur 0 + une grille de points couleur 1 (un pixel sur deux, une ligne sur deux). Rien à l'arrêt ; figé en pause.
- **Clic** : Analyseur → Oscilloscope → Éteint. **Double-clic** : démarre/arrête le plugin de visualisation (MilkDrop…).
- **Analyseur** :
  - Barres **épaisses** (défaut) : 19 barres de 3 px + 1 px d'écart ; **fines** : 75 barres de 1 px. Hauteur max 15 px.
  - Styles de couleur : *normal* (dégradé fixé à l'écran), *fire* (dégradé qui suit le sommet de la barre), *line* (barre unicolore selon sa hauteur).
  - Chute des barres : 5 vitesses (0,19 / 0,375 / **0,75** / 1 / 2 px par pas).
  - Crêtes (couleur 23) : remontent avec la barre, retombent en accélérant (facteurs 1,05 / **1,1** / 1,2 / 1,4 / 1,6).
  - Rafraîchissement : 70 / **35** / 18 / 9 i/s.
  - Spectre (5.x) : FFT 512 points → 256 amplitudes → 75 bandes sur une échelle exponentielle (≈ par demi-tons).
- **Oscilloscope** : 75 échantillons, styles points / **lignes** / plein ; couleur selon la distance au centre (18 au centre → 22).
- Mode shade : 38×5 (analyseur 37 colonnes ou oscillo 38 points).

---

## 7. L'égaliseur

- **10 bandes, ±12 dB**, préampli. Libellés : 60, 170, 310, 600, 1k, 3k, 6k, 12k, 14k, 16k Hz. En 5.x, les filtres tournent à 70, 180, 320, 600, 1k, 3k, 6k, 12k, 14k, 16k. Option « ISO » (31,5 → 16k) depuis 5.51.
- Chaque curseur est stocké sur un octet **0–63** (0 = +12 dB, 31 = 0 dB, 63 = −12 dB). Les valeurs 30–32 s'aimantent à 0 dB.
- **Algorithme 5.x** (« 4Front ») : 10 filtres passe-bande en série, chacun ajoute `g·BP(x)` avec `g = 10^(dB/20) − 1` ; Q = 2,82 en boost, 0,705 en coupe ; limiteur à −0,6 dBFS. Option « Constant-Q ». *(À réimplémenter à partir des formules standard RBJ, pas du code d'origine.)*
- Webamp fait plus simple : low-shelf + 8 peaking + high-shelf, Q = 1.
- **Boutons** : ON (touche N), AUTO (A, charge un preset par fichier), PRESETS (S) → Charger / Enregistrer / Supprimer (preset, preset auto, défaut, fichier .eqf). Pendant le réglage, la zone de titre affiche `EQ: 70Hz: +3.0 dB`.
- **Fichiers** : en-tête `"Winamp EQ library file v1.1" 0x1A "!--"` puis des enregistrements de 268 octets (nom sur 257 octets + 10 bandes + préampli). `Winamp.q1` = bibliothèque (18 presets d'usine : Classical, Club, Dance, Flat, Laptop speakers/headphones, Large hall, Party, Pop, Reggae, Rock, Soft, Ska, Full Bass, Soft Rock, Full Treble, Full Bass & Treble, Live, Techno) ; `Winamp.q2` = presets auto par nom de fichier ; `.eqf` = même format.

---

## 8. La playlist

| Bouton | Choix (de bas en haut) |
|---|---|
| **ADD** | fichier (L), dossier (Shift+L), URL (Ctrl+L) |
| **REM** | sélection (Suppr), garder la sélection / crop (Ctrl+Suppr), tout (Ctrl+N), divers : fichiers introuvables (Alt+Suppr), supprimer physiquement |
| **SEL** | tout (Ctrl+A), rien, inverser (Ctrl+I) |
| **MISC** | tri (titre / nom de fichier / chemin, inverser Ctrl+R, mélanger Ctrl+Shift+R), infos fichier (Alt+3), entrée de playlist (Ctrl+E), options |
| **LIST** | charger (Ctrl+O), enregistrer (Ctrl+S), nouvelle |

- Un clic ouvre la bande de choix, un second choisit. Double-clic ou Entrée = lecture. Clic / Shift / Ctrl pour sélectionner ; glisser la sélection la déplace (aussi Alt+↑/↓). Espace = aller au morceau en cours.
- Lignes : « N. Titre » à gauche, m:ss à droite. Temps total `sélection/total` (`+` si des durées inconnues).
- **Formats** : M3U (`#EXTM3U`, `#EXTINF:secondes,titre`), M3U8 (UTF-8), PLS (`[playlist]`, `File1=`, `Title1=`, `Length1=` ; −1 = flux). Détection par la première ligne.
- **Jump to file (J)** : recherche insensible à la casse, tous les mots doivent apparaître, Entrée joue.
- **Formatage des titres** (5.x) : `[%artist% - ]$if2(%title%,$filepart(%filename%))`.

---

## 9. Transport : comportements fins

| Bouton | Clic | Shift | Ctrl |
|---|---|---|---|
| Précédent (Z) | toujours le morceau précédent (boucle seulement si Répéter) | −5 s | début de liste |
| Lecture (X) | joue ; reprend si en pause ; **redémarre** si déjà en lecture | ouvrir fichier | ouvrir URL |
| Pause (C) | bascule | | |
| Stop (V) | stop | **fondu de sortie** (~2 s) | **stop après le morceau en cours** |
| Suivant (B) | suivant | +5 s | fin de liste |
| Éjecter | ouvrir fichier | ouvrir dossier | ouvrir URL |

- **Aléatoire** : une permutation pré-calculée, refaite quand la liste change ; en fin de permutation avec Répéter, elle est « remélangée » partiellement (taux 50 %).
- **Répéter** (R) + **avance manuelle** (Shift+R) : les deux ensemble = répéter un seul morceau ; avance manuelle seule = stop après chaque morceau.
- Barre de position : affiche `Seek To: 01:23/04:56 (28%)` pendant le glissement. Flèches ±5 s.
- Volume : ±4 par touche, molette ; la zone de titre affiche `Volume: 78%`. Balance : `Balance: 34% left`.
- Signets (bookmarks) : des morceaux, pas des positions (Ctrl+Alt+B).

---

## 10. Raccourcis clavier (classiques)

- **Transport** : Z préc. · X lecture · C pause · V stop · B suiv. · Shift+V fondu · Ctrl+V stop après · ←/→ ±5 s · ↑/↓ volume · pavé numérique (4/5/6 transport, 1/3 ±10 morceaux, 0 ouvrir, . jump).
- **Ouvrir** : L fichier · Shift+L dossier · Ctrl+L URL · J aller au morceau · Ctrl+J aller au temps.
- **Modes** : R répéter · S aléatoire · Shift+R avance manuelle · Ctrl+T temps écoulé/restant.
- **Fenêtres** : Alt+W principale · Alt+E playlist · Alt+G égaliseur · Ctrl+D double taille · Ctrl+W shade · Ctrl+A toujours devant · Alt+S skins · F5 recharger le skin · Ctrl+P préférences · Alt+3 infos fichier.
- **Visualisation** : Ctrl+Shift+K démarrer/arrêter · Ctrl+K choisir · Alt+K configurer.
- **Égaliseur** : 1…0 montent les bandes, Q…P les baissent, N on/off, A auto, S presets.
- **Raccourcis globaux** (5.x) : Ctrl+Alt+Inser/Début/Fin/PgPréc/PgSuiv + touches multimédia.

---

## 11. Radio Internet (SHOUTcast / ICY)

1. Le client envoie un GET HTTP avec `Icy-MetaData:1`.
2. Le serveur répond `ICY 200 OK` (SHOUTcast v1) ou `HTTP/1.0 200 OK` (Icecast), avec `icy-name`, `icy-genre`, `icy-br` et **`icy-metaint: N`**.
3. Tous les N octets d'audio : 1 octet L, puis L×16 octets de métadonnées (`StreamTitle='Artiste - Titre';StreamUrl='…';`). L=0 = pas de changement.
4. Winamp affiche StreamTitle (avec option « nom du flux » entre parenthèses), `[Connecting]`, `[Buffer: 45%]`.

Fichiers `.pls` avec `LengthN=-1` pour les flux. SHOUTcast appartient aujourd'hui à Azerion ; l'annuaire existe encore mais son API demande une clé partenaire. SHOUTcast v2 utilise Ultravox (plus rare).

---

## 12. Chaîne audio

- **Ordre du signal (5.x)** : décodeur → [préampli × ReplayGain → EQ 10 bandes → limiteur] → plugin DSP → plugin de sortie (qui applique volume et balance).
- **Les visualisations voient l'audio avant le DSP.**
- **Gapless** : MP3 (5.2), MP4 (5.3), via le tampon de la sortie DirectSound.
- **Fondu enchaîné** : fonction de la sortie DirectSound (fondus par événement : début, fin de morceau = crossfade, pause/stop, saut), pas du cœur.
- **ReplayGain** (5.3) : piste ou album, « appliquer + éviter l'écrêtage » par défaut, −6 dB pour les fichiers sans info.
- **Formats en 5.x** : MP1/2/3, AAC/M4A/ALAC, OGG Vorbis, FLAC, WAV/AIFF, WavPack, MIDI, MOD/S3M/XM/IT, CD audio, WMA, + vidéo.
- Sorties : DirectSound (défaut), waveOut, écriture disque (WAV), WASAPI (5.8).

---

## 13. Le système de plugins

| Préfixe | Rôle | Point d'entrée exporté |
|---|---|---|
| `in_` | décodeur (lit un format, nourrit vis et DSP) | `winampGetInModule2` |
| `out_` | sortie audio | `winampGetOutModule` |
| `vis_` | visualisation (une seule active) | `winampVisGetHeader` |
| `dsp_` | effet sur le PCM (un seul actif) | `winampDSPGetHeader2` |
| `gen_` | usage général (raccourcis, icône de barre, télécommandes…) | `winampGetGeneralPurposePlugin` |
| `ml_`, `pmp_`, `enc_` | Media Library, baladeurs, encodeurs | … |

- Découverte : Winamp parcourt le dossier Plugins, `LoadLibrary`, `GetProcAddress`, et remplit la structure renvoyée (fenêtre parente, instance…).
- **Visualisation** (`winampVisModule`) : `sRate`, `nCh`, `latencyMs`, `delayMs`, `spectrumNch`, `waveformNch`, **`spectrumData[2][576]`**, **`waveformData[2][576]`** (octets), fonctions `Config / Init / Render / Quit`. Un thread dédié appelle `Render` en boucle avec la trame audio la plus proche du temps de lecture + latence.
- **Spectre fourni** (5.x) : 512 échantillons (octet de poids fort), anti-continu, fenêtre de Hann, FFT 512 points → 256 amplitudes interpolées en 512 valeurs linéaires en fréquence.
- **DSP** : `ModifySamples(échantillons, n, bits, canaux, fréquence)` renvoie le nombre d'échantillons produits (0,5× à 2×).
- **Entrée** : `Play / Pause / Stop / GetLength / GetOutputTime / SetOutputTime / SetVolume / SetPan / EQSet…` ; c'est elle qui pousse les données de vis puis le DSP puis la sortie.
- **Sortie** : `Open / Write / CanWrite / IsPlaying / Pause / SetVolume / SetPan / Flush / GetOutputTime / GetWrittenTime`.
- Les plugins généraux pilotaient Winamp par messages Windows (`WM_WA_IPC`, ~120 commandes) et en sous-classant sa fenêtre.
- **Tout ça est du Win32 x86** (handles de fenêtres, DirectX, messages Windows, code natif généré à la volée par l'évaluateur d'expressions). Sur Mac/ARM/Android/iOS : impossible ; sur Windows 64 bits : seulement via un processus-pont 32 bits.

**Plugins célèbres** : MilkDrop, AVS, Geiss (vis) ; DFX, Enhancer (DSP) ; SqrSoft Advanced Crossfading (sortie) ; in_mod, in_sidplay2 (entrée : musiques de modules et de Commodore 64) ; Global Hotkeys, Album List, scrobbler Last.fm (généraux).

---

## 14. MilkDrop

- **Préset `.milk`** = fichier texte INI. En-têtes MilkDrop 2 : `MILKDROP_PRESET_VERSION=201`, `PSVERSION=…`, puis `[preset00]` :
  - réglages : `fDecay`, `zoom`, `rot`, `warp`, `cx/cy`, `dx/dy`, `sx/sy`, vagues (`nWaveMode`, `wave_r/g/b`…), bordures, vecteurs de mouvement, flous…
  - **code** ligne par ligne : `per_frame_init_N`, `per_frame_N`, `per_pixel_N` (exécuté par sommet d'une grille déformée) ;
  - **4 vagues** et **4 formes** personnalisées, chacune avec son code ;
  - **shaders** HLSL (`warp_N`, `comp_N`) en MilkDrop 2.
- **Variables** : `time, fps, frame, bass, mid, treb, bass_att…` en lecture ; `zoom, rot, warp, decay…` en écriture ; `q1…q32` pour passer des valeurs entre étapes.
- **Langage** : « ns-eel », petit langage d'expressions (sin, cos, pow, if, above, below, rand, megabuf…).
- **Audio** : MilkDrop fait sa propre FFT sur les 2×576 échantillons ; bass 200–761 Hz, mid 761–2897 Hz, treb 2897–11025 Hz, normalisés par la moyenne longue.
- **Rendu** : chaque image = image précédente déformée par la grille (warp) + vagues/formes, puis composition (comp) vers l'écran ; c'est la rétroaction qui donne l'effet « liquide ».
- **Versions** : 1.0 (2001), 1.04 open source BSD (2005), 2.0 (2007, shaders), 2.25c open source BSD (2013). Forks : BeatDrop, MilkDrop3.
- **Presets** : ~475 livrés avec Winamp 5.9 ; « Cream of the Crop » = 9 795 presets (pack par défaut de projectM). **Licence** : aucune licence explicite, chaque auteur garde ses droits ; projectM les considère « de facto domaine public » — c'est leur position, pas une garantie. Les charger depuis un dossier fourni par l'utilisateur est sans risque ; les embarquer, c'est un risque assumé.
- **Moteurs disponibles** :
  - **projectM 4.1** : C++, LGPL-2.1, OpenGL 3.3 / GLES 3 (pas de Metal/Vulkan), Android OK, iOS compilable. Bindings Rust officiels pas mis à jour depuis déc. 2024.
  - **Butterchurn** : JavaScript/WebGL, MIT.
  - **lactea** : pur Rust + wgpu (Metal, Vulkan, DX12, GL), MIT/Apache, parse `.milk`, évaluateur ns-eel, traducteur HLSL→WGSL. **Publié le 11–12/10/2026**, revendique 99,8 % de fidélité à projectM sur 4 169 presets. Prometteur mais à évaluer (provenance, maturité).

---

## 15. AVS (Advanced Visualization Studio)

- Justin Frankel, 2000 ; open source BSD en 2005. Rendu logiciel dans un tampon 32 bits, sortie DirectDraw.
- Un preset `.avs` est **binaire** : en-tête `Nullsoft AVS Preset 0.2\x1a`, puis une liste d'effets (identifiant u32 + longueur + données), imbriquables. 46 effets intégrés (Simple Spectrum, SuperScope, Dynamic Movement, Trans/Movement, Blur, Water, StarField…), plus des extensions DLL (« APE ») non portables.
- Réimplémentations : **vis_avs** de grandchild (BSD, compile sous Linux en `libavs.so`, début de frontend Rust), **Webvs** (JS/WebGL, sous-ensemble).

---

## 16. Le reste

- **Menu principal (clic droit)** : À propos · Lecture (fichier, URL, dossier, signets) · Infos fichier · Signets · Fenêtres · Options (préférences, skins, temps, toujours devant, double taille, EasyMove, répéter, aléatoire) · Lecture (toutes les commandes de transport) · Visualisation · Skins (navigateur Alt+S) · Aide · Quitter.
- **Préférences (5.x)** : Général (types de fichiers, playlist, titres, lecture + EQ + ReplayGain, vidéo, langue), Media Library, Skins, Plugins (entrée, sortie, visualisation, DSP, général).
- **Media Library** (2.90 → 5.x) : base de données de la musique, vues, radios, playlists, CD (extraction/gravure), podcasts, baladeurs. Énorme ; c'est ce qui a rendu Winamp 5 « lourd ».
- **Infos fichier** : édition des tags ID3v1/ID3v2/APE.
- **Minibrowser** (2.10), **Winamp Agent** (icône de barre, 2.6) : historiques.

---

## 17. Réimplémentations existantes et ce qu'on peut réutiliser

| Projet | Langage | Licence | Ce qu'on en tire |
|---|---|---|---|
| **Webamp** | JS | MIT | **La** référence pour le rendu des skins (`skinSprites.ts` = toutes les coordonnées de sprites), le parseur `.eqf`, Butterchurn |
| **Audacious** (interface classique) | C++ | GPL | comportement de docking, défilement — à lire, pas à copier (GPL) |
| **OneAmp** | Rust (egui + wgpu, symphonia, cpal) | MIT/Apache | preuve que la pile Rust tient ; jeune (8 étoiles) |
| **WACUP** | C++ fermé | — | le Winamp « continué » par un ancien développeur ; compatible plugins en x86 |
| Code source Winamp 2024 | C++ | WCL (pas de redistribution modifiée) | **lecture des chiffres uniquement** |
| MilkDrop 2.25c, AVS 2005 | C++ | BSD | réutilisables (algorithmes, formats) |
| projectM | C++ | LGPL-2.1 | lien dynamique uniquement ; OpenGL |
| projectm-eval | C | MIT | évaluateur ns-eel portable |
| lactea | Rust | MIT/Apache | candidat MilkDrop pur Rust, à éprouver |

---

## 18. Points non vérifiés et désaccords entre sources

- Aimantation : Winamp 10 px, Webamp 15 px. Défilement du titre : 200 ms (Winamp) vs 220 ms (Webamp).
- Couleurs d'oscilloscope : un tutoriel dit 18 = creux / 22 = crêtes ; le code dit 18 = centre. On suit le code.
- `viscolor.txt` : Winamp s'arrête à la première ligne non numérique, Webamp l'ignore et continue.
- Non vérifiés : mode de remplissage des polygones `region.txt` qui se chevauchent ; règles exactes de repositionnement des fenêtres attachées quand une taille change ; correspondance exacte du spectre en 2.x ; période de clignotement en pause ; format exact du B4S ; existence de certains raccourcis en 2.x (Shift+R, F3, Ctrl+W).

---

## Sources

- Webamp : https://github.com/captbaritone/webamp (skinSprites.ts, skinParser.js, regionParser.ts, VisPainter.ts, winamp-eqf)
- Webamp modern / MAKI : https://jordaneldredge.com/webamp-modern/
- Skin Museum : https://skins.webamp.org
- Tutoriel de skins Nullsoft (copie) : https://winampskins.neocities.org/ · http://wiki.winamp.com/wiki/The_Base_Skin · https://www.bjoreman.com/old/winamp/skins.htm
- Code Winamp 5.9 (miroirs, lecture seule) : https://git.txlyre.website/txlyre/winamp · https://github.com/alexfreud/winamp
- Audacious skins : https://github.com/audacious-media-player/audacious-plugins/tree/master/src/skins
- MilkDrop : https://en.wikipedia.org/wiki/MilkDrop · https://geisswerks.com/about_milkdrop.html · https://sourceforge.net/projects/milkdrop2/
- Presets : https://github.com/projectM-visualizer/presets-cream-of-the-crop
- projectM : https://github.com/projectM-visualizer/projectm · https://github.com/projectM-visualizer/projectm-rs · https://github.com/projectM-visualizer/projectm-eval
- lactea : https://github.com/M0Rf30/lactea
- Butterchurn : https://github.com/jberg/butterchurn
- AVS : https://en.wikipedia.org/wiki/Advanced_Visualization_Studio · https://github.com/grandchild/vis_avs · https://github.com/azeem/webvs
- WACUP : https://getwacup.com/
- OneAmp : https://github.com/all3f0r1/oneamp
- Historique et marque : https://en.wikipedia.org/wiki/Winamp · https://www.tomshardware.com/tech-industry/winamp-owner-deletes-open-source-repository-after-a-bumpy-month-on-github · https://www.llama-group.com/news/llama-group-officially-becomes-winamp-group-sa/
- SHOUTcast : https://en.wikipedia.org/wiki/SHOUTcast · https://www.shoutcast.com/
- Formats de playlist : https://en.wikipedia.org/wiki/M3U · https://en.wikipedia.org/wiki/PLS_(file_format)
