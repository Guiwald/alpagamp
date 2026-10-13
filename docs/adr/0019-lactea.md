# ADR 0019 — Moteur MilkDrop : lactea retenu après audit

- Statut : acceptée (13/10/2026)
- Suite de l'ADR 0011.

## Audit

**Licence et provenance.** lactea est sous `MIT OR Apache-2.0`. Ses notices déclarent :

- MilkDrop 2 (Nullsoft, BSD-3-Clause, code publié par Nullsoft en 2013) : le rendu en est un portage, `include.fx`
  est copié tel quel ;
- BeatDrop (BSD-3-Clause), stb_image (domaine public / MIT), ns-eel2 (licence de type zlib) : portages déclarés ;
- projectM (LGPL-2.1) et projectm-eval (MIT) : **oracles de test seulement**, ni liés, ni distribués, ni copiés.

À la lecture, le code cite souvent des fichiers et des lignes de projectM comme *référence de comportement* (« the
maths below is re-derived from reading the source, not copied »). Ce n'est donc pas une salle blanche : c'est une
réécriture en Rust, avec une autre architecture (wgpu, traduction HLSL → WGSL par naga), qui reproduit un
comportement. Le comportement d'un programme n'est pas protégé par le droit d'auteur ; aucune traduction ligne à
ligne n'a été trouvée. **Risque résiduel jugé faible et accepté.** Le code de MilkDrop 2 étant sous BSD, l'ADR 0001
(aucun code de Winamp) n'est pas en cause ; la notice BSD est reproduite dans `THIRD_PARTY.md`.

**Dépendances.** Rien à compiler en C/C++ : lactea → wgpu 30 → Metal (macOS), Vulkan (Android, Windows, Linux),
DX12 (Windows), chargés dynamiquement. OpenGL n'est pas activé.

**Essai sur 200 presets** (un tous les 49 de *Cream of the Crop*, 9 795 presets ; 90 images à 320×240 ; son
synthétique ; GPU logiciel llvmpipe) : 197 rendus, 3 refusés (erreurs de syntaxe dans les équations du preset
lui-même), 1 shader remplacé par le shader par défaut, 8 images uniformes (dont des presets « fondu au noir »),
0 erreur GPU ; 17 ms par image en médiane, 45 ms au 95ᵉ centile, sur le processeur. La planche contact a été
vérifiée à l'œil. L'essai sur Mac, Pixel 8 et Windows fait partie de la recette 1.0.

## Décision

1. lactea est utilisé **à la version figée `=0.2.1`** (somme de contrôle dans `Cargo.lock` ; crates.io ne retire
   jamais une version). La copie sous le compte GitHub de Guillaume sera faite quand l'accès GitHub sera réglé.
2. Il reste caché derrière `Visualizer` (crate `alpagamp-milkdrop`) : on peut en changer sans toucher au reste.
3. Le GPU passe par wgpu ; la fenêtre de visualisation (et, sur Android, tout l'écran) est composée sur le GPU :
   l'image de MilkDrop, puis le cadre de skin dessiné par le CPU, par-dessus.
4. Six presets **originaux** d'Alpagamp sont intégrés ; *Cream of the Crop* est téléchargé à la demande à un commit
   figé du dépôt de projectM (ADR 0012), jamais livré.
5. Le nom « MilkDrop » ne sert qu'à décrire la compatibilité (« presets MilkDrop ») ; la fenêtre s'appelle
   « Visualisation ».

## Conséquences

- Un GPU compatible Vulkan, Metal ou DX12 est nécessaire pour la visualisation ; sans lui, le reste du lecteur
  marche (message clair, visualiseur intégré seulement).
- Binaire plus gros (wgpu, naga, lactea : quelques Mo).
- lactea est jeune (publié le 11/10/2026, un seul auteur) : surveiller les versions, ne monter qu'après un nouvel
  essai sur 200 presets.
