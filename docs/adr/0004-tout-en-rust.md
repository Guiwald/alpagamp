# ADR 0004 — Tout en Rust

- Statut : acceptée (13/10/2026)

## Décision

- **Toute la logique de l'application est en Rust.**
- **Aucune bibliothèque C/C++** en dépendance (donc pas de projectM, pas de FFmpeg) ; les crates qui enveloppent
  les API système (CoreAudio, AAudio, WASAPI, Metal, Vulkan…) sont acceptées.
- Tolérés : fichiers de packaging et de configuration (manifeste Android, Info.plist, workflows CI, scripts de
  build) et, **seulement si c'est inévitable**, quelques dizaines de lignes de Java/Kotlin pour le service de
  lecture en arrière-plan d'Android.

## Conséquences

- Décodage audio en Rust pur (symphonia), TLS en rustls, rendu via wgpu.
- Le moteur MilkDrop est en Rust (choix du moteur : ADR à venir).
