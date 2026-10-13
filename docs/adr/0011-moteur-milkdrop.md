# ADR 0011 — Moteur MilkDrop : audit de lactea, sinon moteur maison

- Statut : acceptée (13/10/2026)

## Contexte

projectM est exclu (C++, ADR 0004). lactea (pur Rust, wgpu, MIT/Apache) a été publié le 11–12/10/2026 par une seule
personne et annonce 99,8 % de fidélité à projectM sur 4 169 presets.

## Décision

1. **Audit de lactea** : licence et provenance (s'assurer que ce n'est pas une traduction de code LGPL relicenciée),
   lecture du code, test sur ~200 presets sur macOS, Android et Windows.
2. S'il passe : on l'utilise **à une version figée**, avec une copie (fork) sous le compte de Guillaume pour pouvoir
   corriger.
3. Sinon : **moteur maison en Rust** — presets sans shaders (MilkDrop 1) d'abord, puis shaders MilkDrop 2
   (traduction HLSL → WGSL).

Le moteur est caché derrière une interface (`Visualizer`) : on peut changer de moteur sans toucher au reste.
