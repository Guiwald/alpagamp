# ADR 0020 — API des plugins WebAssembly (version 1)

- Statut : acceptée (14/10/2026)
- Précise les ADR 0003 et 0013.

## Contexte

L'ADR 0003 retient des plugins en WebAssembly : un seul fichier `.wasm` qui marche partout (Mac, Windows,
Android, plus tard iPhone), dans un bac à sable. Il faut un moteur WebAssembly en Rust (ADR 0004) qui fonctionne
aussi là où la compilation à la volée est interdite (iOS), et une interface stable.

## Décision

1. **Moteur : wasmi** (interpréteur en Rust pur, MIT/Apache-2.0), modules validés au chargement. Chaque appel
   reçoit un budget de « carburant » (proportionnel au travail demandé) et la mémoire d'un plugin est plafonnée
   à 64 Mo : un plugin qui boucle sans fin, plante ou déborde est arrêté et désactivé, le lecteur continue.
2. **Trois sortes de plugins** : `vis` (visualisation : une image par trame), `dsp` (effet audio, en place sur des
   échantillons stéréo `f32`), `general` (réagit au temps et pilote le lecteur).
3. **Le module exporte** :
   - `memory` ;
   - `alp_api() -> i32` : la version de l'API (1) ;
   - `alp_info() -> i32` : adresse d'un texte JSON (UTF-8) terminé par un octet nul : `name`, `kind` (`vis`,
     `dsp` ou `general`), `author`, `version`, `description`, `options` (liste de
     `{ "id", "label", "values": [{ "value", "label" }], "default" }`) et `remember` (facultatif, `false` : le
     plugin n'est pas réactivé au démarrage suivant, comme la minuterie). Chaque texte peut être une chaîne ou un
     objet par langue (`{ "en": …, "fr": …, "es": …, "de": …, "it": …, "ca": …, "ga": … }`, repli sur `en`) ;
   - `alp_alloc(octets: i32) -> i32` : réserve de la mémoire où l'hôte écrit ses données (l'hôte la réutilise) ;
   - facultatif : `alp_init()`, puis `alp_option(rang: i32, valeur: i32)` pour chaque option (rang dans
     `options`), au démarrage et à chaque changement ; `alp_quit()` quand le plugin est désactivé ou que le
     lecteur se ferme (la minuterie y rend le volume) ;
   - `dsp` : `alp_dsp(adresse: i32, trames: i32, frequence: i32)` — échantillons entrelacés gauche/droite ;
   - `vis` : `alp_vis(largeur: i32, hauteur: i32, temps_ms: i32, spectre: i32, onde: i32) -> i32` — `spectre` :
     2 × 576 `f32` (gauche puis droite), amplitude 0–1 par bande de fréquence linéaire (bande `k` ≈
     `k × fréquence / 1152` Hz) ; `onde` : 2 × 576 `f32` entre −1 et 1. La fonction renvoie l'adresse de l'image :
     `largeur × hauteur` pixels `0xAARRGGBB`. L'hôte choisit une taille modeste (≤ 320 × 240 environ) et
     l'agrandit à l'écran : les gros pixels font partie du style ;
   - `general` : `alp_tick(temps_ms: i64)` (temps depuis l'activation), appelé dix fois par seconde.
4. **L'hôte fournit** (module d'import `alpagamp`) : `log(adresse, longueur)`, `notify(adresse, longueur)` (message
   dans le titre défilant), `player_state() -> i32` (0 arrêt, 1 lecture, 2 pause), `command(n)` (1 lecture,
   2 pause, 3 stop, 4 suivant, 5 précédent), `volume() -> i32` et `set_volume(v)` (0–255),
   `skin_color(i) -> i32` (couleurs du visualiseur du skin, 0–23, `0xRRGGBB`), `lang(adresse, place) -> i32`
   (écrit le code de la langue de l'interface, `fr`, `en`…, et renvoie sa longueur).
5. **Trois plugins d'exemple** (ADR 0013), écrits en Rust avec un petit kit (`plugin-examples/sdk`), compilés en
   `wasm32-unknown-unknown` (`cargo xtask plugins`) et livrés dans le lecteur : *Barres plein écran* (vis),
   *Enhancer* (dsp), *Minuterie de sommeil* (general). Les `.wasm` construits sont versionnés dans
   `assets/plugins/`. Les plugins de l'utilisateur se déposent dans le dossier des plugins (un fichier du même
   nom qu'un plugin livré le remplace).
6. Interface : menu **Plugins** (activer, options de chaque plugin, à propos, installer, ouvrir le dossier) ; une
   visualisation de plugin s'affiche dans la fenêtre de visualisation à la place de MilkDrop (une seule à la
   fois) ; les effets audio s'appliquent après l'égaliseur, dans l'ordre alphabétique.

## Conséquences

- L'API est publique et versionnée : une version 2 ajoutera des exports sans casser la 1 (`alp_api`).
- Un effet audio interprété coûte plus cher qu'en natif : l'hôte le fait tourner dans le fil de décodage (pas dans
  celui de la carte son), par blocs de plusieurs centaines de trames.
- Les plugins ne voient ni le disque ni le réseau.
