# Changelog

Toutes les évolutions notables d'Alpagamp. Format [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/),
numérotation [SemVer](https://semver.org/lang/fr/) adaptée (voir `docs/versions.md`). Chaque entrée commence par
le composant concerné (« Audio : … »).

## [Non publié]

## [0.4.0] - 2026-10-13

> Mise en service : Windows (`Alpagamp-0.4.0-windows-x64.zip`) — remplacer l'ancien dossier. Android
> (`Alpagamp-0.4.0.apk`) — installer par-dessus la 0.3. Pour Spotify : Bibliothèque (Alt+L, ou menu ›
> Bibliothèque sur téléphone) › « Se connecter à Spotify… », autoriser dans le navigateur, revenir dans Alpagamp.
> Spotify Premium nécessaire.

### Ajouté
- Spotify : connexion en un clic (navigateur, sans configuration), gardée d'une session à l'autre ; lecture des
  morceaux Spotify dans la chaîne audio d'Alpagamp (égaliseur, visualiseur, fondu enchaîné), 320 kbit/s ;
  liens `open.spotify.com` et URI `spotify:` dans la playlist (les liens de playlists et d'albums ajoutent leurs
  morceaux). Spotify Premium nécessaire.
- Spotify Connect : Alpagamp apparaît comme appareil dans l'appli Spotify (« Alpagamp (Mac) », « Alpagamp
  (Android) »…) ; ce qui y est lancé passe par Alpagamp, et les boutons de transport pilotent Spotify.
- Interface : Bibliothèque (Alt+L) — onglets Playlists (avec Titres likés), Likés, Albums, Recherche et
  Radios (annuaire radio-browser.info, recherche par nom ou genre) ; double-clic pour ouvrir ou jouer, boutons
  Lire et Ajouter. Sur téléphone, dans le menu.
- Docs : ADR 0018 (Spotify dans le moteur, TLS de librespot).

## [0.3.0] - 2026-10-13

> Mise en service : Windows (`Alpagamp-0.3.0-windows-x64.zip`) — remplacer l'ancien dossier, réglages gardés.
> Android (`Alpagamp-0.3.0.apk`) — ouvrir le fichier sur le téléphone, autoriser l'installation depuis cette source,
> installer, puis autoriser l'accès à la musique au premier lancement. Les prochaines versions Android
> s'installeront par-dessus (même clé de signature).

### Ajouté
- Audio : radios Internet (SHOUTcast, Icecast ; HTTP et HTTPS ; MP3, AAC, OGG) avec le titre en cours dans le
  titre défilant et la playlist, le nom de la station dans les infos ; playlists de radios `.pls` / `.m3u`
  suivies ; mise en mémoire tampon sans bloquer l'interface. Ajout par « Ajouter une adresse » (Ctrl+L), en
  glissant un lien, ou dans une playlist.
- Audio : fondu enchaîné réglable (1 à 10 s) entre les morceaux, à puissance constante.
- Audio : ReplayGain (morceau ou album), préampli, protection contre la saturation.
- Lecture : signets — ajouter le morceau en cours (Ctrl+Alt+B), les rejouer depuis le menu Signets, les retirer.
- Interface : toutes les options du visualiseur — style de l'analyseur (normal, feu, ligne), barres épaisses ou
  fines, crêtes, vitesses de chute, style de l'oscilloscope (points, lignes, plein), fréquence d'affichage.
- Interface : infos du fichier avec la station et les valeurs ReplayGain.
- Android : première version (`Alpagamp-0.3.0.apk`) — fenêtre principale, égaliseur et playlist empilés à la
  largeur de l'écran, au toucher (appui long pour le menu, double-tap pour jouer, glisser pour faire défiler la
  playlist) ; musique du téléphone (autorisation demandée au premier lancement) ; radios prêtes à écouter et
  adresses à taper ; mêmes réglages qu'au bureau (égaliseur, fondu enchaîné, ReplayGain, visualiseur, langues).
- Outils : `cargo xtask apk` (APK signé sans Gradle) et simulateur de téléphone sur ordinateur.
- Docs : ADR 0017 (Android sans Java).

### Modifié
- Réseau : certificats racines du système pris en compte (proxys d'entreprise), mandataire `HTTPS_PROXY`
  respecté ; chiffrement par graviola (Rust, sans C).

## [0.2.0] - 2026-10-13

> Mise en service : Windows (`Alpagamp-0.2.0-windows-x64.zip`) — fermer Alpagamp, remplacer l'ancien dossier par
> le nouveau ; réglages, playlist et skins sont gardés. L'égaliseur et la playlist s'ouvrent sous la fenêtre
> principale (Alt+G, Alt+E pour les masquer). La version Mac arrive avec la compilation automatique.

### Ajouté
- Interface : égaliseur 10 bandes et préampli — curseurs (aimantés sur 0 dB, Maj pour régler finement), clic sur
  +12 / 0 / −12 pour régler toutes les bandes, courbe de réponse, boutons ON et AUTO, valeurs dans le titre
  défilant, mode bandeau avec volume et balance.
- Égaliseur : préréglages d'usine, ouverture et enregistrement de fichiers `.eqf`, préréglage automatique par
  morceau (bouton AUTO), remise à plat.
- Interface : playlist redimensionnable (pas de 25×29) — titres et durées lus en arrière-plan, sélection au clic
  (Maj, Ctrl), glisser pour réordonner, molette et ascenseur, menus ADD / REM / SEL / MISC / LIST, mini-transport,
  durée de la sélection et de la liste, bandeau, menu au clic droit.
- Interface : fenêtres aimantées entre elles et aux bords de l'écran (Maj pour ne pas aimanter) ; la fenêtre
  principale entraîne les fenêtres collées ; bandeau, double taille et redimensionnement décalent les fenêtres
  empilées.
- Interface : « Aller au morceau » (J) et « Ajouter une adresse » (Ctrl+L) dans des fenêtres skinnées.
- Interface : raccourcis Alt+G (égaliseur), Alt+E (playlist), J ; dans la playlist : flèches, Pg↑ / Pg↓, Début /
  Fin, Entrée, Suppr, Ctrl+A, Ctrl+I, Alt+↑ / ↓, Ctrl+N / O / S.
- Playlist : enregistrer (M3U8, M3U, PLS) et ouvrir une playlist, trier (titre, nom de fichier, chemin), inverser,
  mélanger, retirer les fichiers introuvables ou les doublons.
- Skins : couleurs et lettres des fenêtres génériques (`genex.bmp`, `gen.bmp`) ; texte de la playlist net sur écran
  Retina (police Liberation Sans).
- Docs : README, ADR 0016 (fenêtres).

### Modifié
- Interface : sous Windows, une seule entrée dans la barre des tâches ; les fenêtres secondaires suivent la
  principale.
- Menus : entrées Égaliseur et Éditeur de playlist ; barre de menus macOS avec Fichier et Fenêtres.

### Corrigé
- Interface : l'application redessinait ses fenêtres sans pause, ce qui occupait le processeur (surtout sous
  Windows).
- Skins : les lettres de titre des fenêtres génériques du skin par défaut n'étaient pas lisibles.

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
