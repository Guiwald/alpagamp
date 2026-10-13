# ADR 0015 — YouTube Music : visualiser le son du système d'abord, intégration complète plus tard

- Statut : acceptée (13/10/2026)

## Contexte

Naku, l'ami de Guillaume qui testera la version Windows, est abonné à YouTube Music et pas à Spotify (librespot
exige Spotify Premium). YouTube Music n'a aucune API officielle donnant l'audio. L'accès non officiel passe par
l'API interne de YouTube : contraire à ses conditions, jetons anti-robot exigés depuis août 2024, adresses de flux
chiffrées par du JavaScript à exécuter, ruptures fréquentes. La seule bibliothèque Rust sérieuse (RustyPipe) est
sous GPL-3.0 et dépend d'un moteur JavaScript en C : incompatible avec les ADR 0004 (pas de C) et la licence MIT.

## Décision

1. **Version 1.1 : mode « visualiser le son de l'ordinateur ».** Alpagamp capte l'audio que joue la machine
   (YouTube Music dans le navigateur, Deezer, n'importe quoi) et l'anime avec le visualiseur intégré et MilkDrop.
   Captation par les API système (WASAPI en boucle de retour sur Windows, Core Audio / ScreenCaptureKit sur macOS).
   Pas d'égaliseur, pas d'entrée de playlist : c'est un mode de visualisation.
2. **Plus tard, après la 1.0 : intégration complète de YouTube Music**, non officielle, précédée d'un prototype
   qui mesure la fragilité (client InnerTube maison, moteur JavaScript en Rust pur type Boa). Gardée « sous le
   coude » avec la Media Library.

## Conséquences

- Pour Naku en v1 : fichiers locaux, radios, et le mode « son de l'ordinateur » dès la 1.1.
- L'architecture garde la source audio abstraite (`PcmSource`), ce qui permettra d'ajouter YouTube Music comme
  une source de plus.
