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

## Révision (0.5, 14/10/2026)

- **Formes (`region.txt`)** : faites. Windows : `SetWindowRgn` (polygones orientés dans le même sens, règle
  « non nul » : leur union, comme Winamp) ; macOS : fenêtre transparente et masque `CAShapeLayer` ; Linux
  (développement) : rectangulaire.
- **Curseurs des skins** (`*.cur`, format ICO : 1, 4, 8, 24 et 32 bits, masque de transparence) : faits, choisis
  selon la zone survolée comme dans Winamp 2 (`close.cur`, `posbar.cur`, `eqslid.cur`, `psize.cur`…).
- **Taille des fenêtres** : l'agrandissement (×1, ×2, ×3, ou automatique selon la largeur de l'écran) s'applique
  à **toutes** les fenêtres — écart assumé au point 3 : sur les écrans d'aujourd'hui, la seule fenêtre principale
  agrandie était trop petite à côté d'une playlist à ×1. Ctrl+D (Cmd+D) passe de ×1 à ×2.
- **Disposition en points sur macOS** : positions et tailles du groupe aimanté sont tenues en points (espace
  global continu entre un écran Retina et un écran externe), en pixels physiques ailleurs. Corrige les fenêtres
  qui se décollaient en passant d'un écran à l'autre.
