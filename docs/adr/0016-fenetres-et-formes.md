# ADR 0016 — Fenêtres : groupe aimanté, fenêtres génériques skinnées, formes reportées

- Statut : acceptée (13/10/2026)

## Contexte

La 0.2 ajoute l'égaliseur, la playlist et les fenêtres annexes. Winamp 2 les dessine comme des fenêtres séparées
qui s'aimantent, se suivent quand on déplace la fenêtre principale, et se décalent quand une fenêtre passe en
bandeau. Certaines de ses boîtes (« Aller au morceau », « Ouvrir une adresse ») étaient des boîtes Windows natives,
qui n'existent pas sur macOS ni sur Android. Enfin, un skin peut donner une forme non rectangulaire à ses fenêtres
(`region.txt`) ; la bibliothèque d'affichage retenue (softbuffer) ne gère pas la transparence.

## Décision

1. **Une fenêtre système par fenêtre Alpagamp** (principale, égaliseur, playlist, fenêtre générique), sans
   bordure, dessinée au pixel près. La position de chaque fenêtre est tenue par l'application (pas relue au système
   à chaque mouvement), ce qui rend l'aimantation et les décalages déterministes et testables (`layout.rs`).
2. **Aimantation à 10 px skin** contre les bords des autres fenêtres et de l'écran ; Maj désactive l'aimantation.
   Déplacer la fenêtre principale entraîne toutes les fenêtres qui la touchent, de proche en proche. Une fenêtre
   qui change de taille (bandeau, double taille, redimensionnement) pousse celles empilées sous elle ou collées à
   sa droite.
3. **La double taille s'applique à la fenêtre principale et à l'égaliseur**, pas à la playlist (comme Winamp 2).
4. **Sous Windows**, les fenêtres secondaires appartiennent à la fenêtre principale : une seule entrée dans la barre
   des tâches, réduction commune.
5. **Les boîtes qui étaient natives deviennent des fenêtres génériques skinnées** (`gen.bmp`, `genex.bmp`) : même
   aspect partout, et socle des futures fenêtres (Bibliothèque Spotify, plugins, AVS).
6. **Les formes de fenêtres (`region.txt`) sont reportées** : le fichier est lu, mais les fenêtres restent
   rectangulaires. À reprendre avec un rendu qui gère la transparence (ou les API de forme de chaque système),
   au plus tard avec le navigateur de skins (0.5).
7. **Les curseurs de souris des skins** (`*.cur`) sont reportés avec les formes.

## Conséquences

- Les fenêtres se comportent comme dans Winamp sur Mac, Windows et Linux (X11) ; sous Wayland, le système interdit
  de placer les fenêtres soi-même (Linux n'est qu'un environnement de développement).
- Les skins à coins arrondis montrent des coins carrés (pixels du fond) tant que le point 6 n'est pas fait.
