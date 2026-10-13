# ADR 0002 — Spotify via librespot

- Statut : acceptée (13/10/2026)

## Contexte

L'API Web officielle de Spotify ne fournit jamais l'audio ; depuis février 2026 son mode développeur exige
Premium et limite les utilisateurs et plusieurs endpoints. Deux voies : piloter l'appli Spotify officielle
(conforme, mais le son ne passe pas par notre lecteur) ou devenir soi-même un client Spotify avec librespot
(bibliothèque Rust open source, non officielle, utilisée par spotifyd, ncspot, Raspotify, Music Assistant).

## Décision

On utilise **librespot**. L'audio Spotify est décodé dans notre chaîne audio, comme un fichier local : égaliseur,
visualiseur intégré, MilkDrop et plugins DSP s'appliquent à Spotify.

## Conséquences

- Compte **Premium** obligatoire (Guillaume l'a).
- Usage **non autorisé par les conditions de Spotify** : risque théorique de suspension du compte, accepté
  par Guillaume pour un usage personnel.
- librespot casse parfois quand Spotify change son backend (ex. décembre 2025, réparé en 0.8.0) : suivre
  ses versions, isoler son usage derrière une interface pour pouvoir le mettre à jour vite.
- Aucune publication sur les stores ; installation à la main (APK, DMG, ZIP).
- Le lecteur est aussi une **enceinte Spotify Connect** (ADR 0010).
