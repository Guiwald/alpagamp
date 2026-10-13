# ADR 0001 — Réimplémentation propre, sans rien emprunter à Winamp

- Statut : acceptée (13/10/2026)

## Contexte

La marque « Winamp » et le lama appartiennent à Winamp Group SA. Le code source publié en 2024 est sous la
Winamp Collaborative License, qui interdit de redistribuer des versions modifiées ; il contient aussi du code
tiers propriétaire. Les bitmaps du skin de base (base-2.91) sont la propriété de Winamp. Le format de skin
classique, lui, est un format de fichier documenté publiquement (Webamp, Skinner's Atlas).

## Décision

- Le lecteur a **son propre nom** : Alpagamp (ADR 0009) ; ni « Winamp », ni lama, ni logo éclair copié.
- **Aucune ligne de code** n'est reprise du code Winamp, ni de code GPL (Audacious) : on lit ces sources pour
  en tirer des faits (dimensions, coordonnées, formats, comportements), puis on écrit notre code.
  Code permissif (MIT/BSD/Apache : Webamp, MilkDrop 2.25c, AVS 2005) : réutilisable avec attribution.
- Le **skin par défaut est une création originale** dans l'esprit classique. Les skins de l'utilisateur
  (.wsz du Skin Museum, y compris l'original s'il le télécharge lui-même) se chargent normalement.
- La **disposition** (275×116, emplacements des éléments) est conservée : c'est le format de skin lui-même,
  indispensable pour charger les skins existants.

## Conséquences

- Les algorithmes (égaliseur, analyseur) sont réécrits à partir de formules standard (filtres RBJ, FFT).
- Les skins et presets tiers ne sont pas versionnés dans le dépôt (voir ADR 0008).
