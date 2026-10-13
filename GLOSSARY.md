# Glossaire

Le vocabulaire commun du projet. Un terme = un sens ; le code, les docs et les conversations utilisent ces mots-là.

| Terme | Sens |
|---|---|
| **Alpagamp** | Le lecteur (ADR 0009). |
| **Fenêtre principale** | La fenêtre de 275×116 avec l'afficheur, les boutons de transport, volume, balance, barre de position. |
| **Égaliseur** (EQ) | La fenêtre de 275×116 avec les 10 curseurs de bandes et le préampli. |
| **Playlist** | La fenêtre redimensionnable (pas de 25×29) qui liste les morceaux. Aussi : la liste elle-même. |
| **Bandeau** (windowshade) | Mode replié d'une fenêtre, 14 px de haut. |
| **Double taille** | Affichage ×2 au plus proche voisin (fenêtre principale et égaliseur). |
| **Aimantation** (snap) | Collage automatique des fenêtres entre elles et aux bords de l'écran (10 px). |
| **Fenêtres attachées** | Fenêtres dont les bords se touchent exactement ; elles suivent la fenêtre principale quand on la déplace. |
| **Fenêtre générique** | Fenêtre skinnée avec `gen.bmp` / `genex.bmp` : « Aller au morceau », « Ajouter une adresse », plus tard Bibliothèque et plugins (ADR 0016). |
| **Préréglage automatique** | Préréglage d'égaliseur rattaché à un fichier ; chargé à chaque lecture de ce fichier quand « AUTO » est allumé. |
| **Clutterbar** | La colonne de lettres O A I D V à gauche de la fenêtre principale. |
| **Titre défilant** (marquee) | Le texte « N. Titre (m:ss) » qui défile dans la fenêtre principale. |
| **Visualiseur intégré** | La zone 76×16 de la fenêtre principale : analyseur de spectre ou oscilloscope. |
| **Skin classique** | Un fichier `.wsz` (zip de BMP + `viscolor.txt`, `pledit.txt`, `region.txt`, curseurs). |
| **Skin moderne** | Un fichier `.wal` (XML + scripts MAKI). Non supporté. |
| **Skin par défaut** | Le skin original livré avec le lecteur (ADR 0001). |
| **Sprite** | Un rectangle d'une planche BMP du skin, à coordonnées fixes. |
| **Preset MilkDrop** | Un fichier `.milk` : réglages + code d'équations + shaders, rendu par le moteur MilkDrop. |
| **Preset AVS** | Un fichier `.avs` binaire (v2). |
| **Preset d'EQ** | 10 bandes + préampli ; fichier `.eqf` ou bibliothèque. |
| **Plugin** | Un module WebAssembly chargé par le lecteur (ADR 0003). Trois sortes : **visualisation**, **DSP** (effet audio), **général**. |
| **Mod** | Terme générique côté utilisateur : skin, preset ou plugin. |
| **Source** | D'où vient un morceau : fichier local, flux radio (URL), Spotify. |
| **Entrée de playlist** | Une ligne de la playlist : une référence vers une source + titre et durée connus. |
| **librespot** | La bibliothèque Rust qui fait du lecteur un client Spotify (ADR 0002). |
| **Spotify Connect** | Le mécanisme qui permet de choisir, depuis l'appli Spotify, l'appareil qui joue. |
| **Chaîne audio** | décodeur → préampli/ReplayGain → égaliseur → limiteur → plugin DSP → sortie (volume, balance). |
| **Trame de visualisation** | 2 canaux × 576 valeurs de spectre + 2 × 576 échantillons de forme d'onde, envoyés aux visualisations. |
| **Mode fidèle / mode adapté** | Les deux présentations sur téléphone (ADR 0006). |
| **Bibliothèque** | La fenêtre qui parcourt Spotify (playlists, titres likés, albums, recherche), les radios et les dossiers locaux (ADR 0010). |
| **Pack de presets** | *Cream of the Crop*, téléchargé à la demande depuis le dépôt projectM (ADR 0012). |
| **Navigateur de skins** | La fenêtre (Alt+S) qui cherche et télécharge des skins du Skin Museum (ADR 0012). |
