# Changelog

Toutes les évolutions notables d'Alpagamp. Format [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/),
numérotation [SemVer](https://semver.org/lang/fr/) adaptée (voir `docs/versions.md`). Chaque entrée commence par
le composant concerné (« Audio : … »).

## [Non publié]

## [0.1.0] - 2026-10-13

> Mise en service : première préversion, pour Windows seulement (`Alpagamp-0.1.0-windows-x64.zip`) : décompresser,
> lancer `alpagamp.exe` ; au premier lancement, « Informations complémentaires » puis « Exécuter quand même ».
> Les versions Mac et Android arrivent ensuite. Pas encore de fenêtres Égaliseur et Playlist (0.2).

### Ajouté
- Interface : fenêtre principale complète au pixel près — barre de titre, clutterbar (O A I D V), temps écoulé ou
  restant (clic), titre défilant, débit et fréquence, mono/stéréo, volume et balance avec messages d'état, barre de
  position, transport, aléatoire, répétition, mode bandeau (double-clic sur le titre), double taille, toujours au
  premier plan.
- Interface : visualiseur intégré — analyseur de spectre (19 barres épaisses ou 75 fines, chute et crêtes) et
  oscilloscope ; un clic fait tourner Analyseur → Oscilloscope → Éteint.
- Skins : lecture des skins classiques `.wsz` (glisser un fichier sur la fenêtre, ou menu Skins), repli fichier par
  fichier sur le skin par défaut, décodeur BMP tolérant (RLE4/RLE8), `viscolor.txt`, `pledit.txt`, `region.txt`.
- Skins : skin par défaut original (métal graphite, afficheur LCD menthe, alpaga).
- Audio : lecture des fichiers locaux MP3, FLAC, OGG Vorbis, WAV, AIFF, AAC/M4A, ALAC ; enchaînement sans blanc entre
  les morceaux ; rééchantillonnage vers la sortie ; ReplayGain prêt ; arrêt en fondu (Maj + Stop) et arrêt après le
  morceau en cours (Ctrl + Stop).
- Lecture : playlist interne (dossiers parcourus, M3U/M3U8/PLS), aléatoire sans répétition, répétition, avance
  manuelle ; la playlist et les réglages sont gardés d'une session à l'autre.
- Interface : raccourcis classiques (Z X C V B, flèches, L, Maj+L, R, S, Ctrl+D, Ctrl+T…), molette pour le volume,
  menu complet au clic droit, infos du fichier.
- Interface : menus en anglais, français, espagnol, allemand, italien, valencien et irlandais (langue du système).
- Windows : exécutable autonome avec icône.
- Docs : référence technique de Winamp, décisions d'architecture (ADR 0001 à 0015), glossaire.
