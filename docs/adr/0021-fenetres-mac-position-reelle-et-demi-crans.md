# ADR 0021 — Fenêtres sur macOS : position réelle, recollage au démarrage, premier plan, demi-crans

- Statut : acceptée (14/10/2026, 0.6.1)
- Révise : ADR 0016 (points 1 et 4, révision 0.5)

## Contexte

Essai de la 0.6.0 sur le Mac de Guillaume, le 14/10 :

1. Au démarrage, une fenêtre se retrouve décalée de son groupe : la playlist recouvre l'égaliseur, ou l'égaliseur
   et la playlist en bandeau s'ouvrent loin sous la fenêtre principale.
2. Glisser la fenêtre principale tout en haut de l'écran : elle s'arrête sous la barre de menus, mais les autres
   fenêtres partent ailleurs (jusque derrière le Dock).
3. Au lancement, l'égaliseur et la playlist restent derrière les autres applications.
4. ×1 est trop petit, ×2 un peu trop grand : il faut un cran au milieu.

Causes trouvées :

1. Chaque fenêtre naissait à la taille du skin en pixels physiques (¼ de sa taille sur Retina), puis était
   agrandie. Or macOS change la taille d'une fenêtre **en gardant son coin bas-gauche** : la fenêtre montait de ¾
   de sa hauteur, sans événement « déplacée ». La principale et l'égaliseur montaient de 174 points, une playlist
   de 174 px skin de 261 points (elle recouvrait l'égaliseur de 87 points), les bandeaux de 21 points (d'où un écart
   de 153 points). Le modèle gardait la position demandée et l'enregistrait.
2. Le déplacement suit la souris *dans* la fenêtre. Quand macOS bloque la fenêtre, l'écart grandit à chaque
   mouvement ; le modèle (position demandée, jamais relue) dérive et emmène le groupe.
3. macOS n'a pas l'équivalent des fenêtres « possédées » de Windows.
4. L'agrandissement était un entier partout.

## Décision

1. **La fenêtre naît à sa taille finale** (taille logique sur macOS), et tout changement de taille remet le coin
   haut-gauche où il était. Sur macOS, **la position réelle est relue** après chaque déplacement : si le système
   l'a corrigée, le modèle la reprend et le reste du groupe suit la position réelle de la fenêtre saisie. Écart au
   point 1 de l'ADR 0016 (position jamais relue), limité à macOS : sous X11 la relecture est asynchrone, sous
   Windows rien ne corrige nos positions.
2. **Le groupe ne monte pas sous la barre de menus** : avant de déplacer, le décalage vertical est borné au haut de
   la zone utile (`NSScreen.visibleFrame`) de l'écran sous chaque fenêtre. L'aimantation aux bords d'écran se fait
   aussi sur la zone utile (sous la barre de menus, au-dessus du Dock).
3. **Recollage au démarrage** : la taille réelle de chaque fenêtre est enregistrée avec sa position. Au démarrage,
   l'arbre des fenêtres collées est reconstruit à partir de la disposition enregistrée ; la principale s'ouvre (à
   la position que macOS lui accorde), puis chaque fenêtre reprend sa place contre celle à laquelle elle était
   collée, à sa taille d'aujourd'hui. Les réglages des versions précédentes (sans taille enregistrée) sont relus
   avec l'agrandissement de l'époque.
4. **Premier plan** : sur macOS, quand une fenêtre d'Alpagamp est activée, toutes les autres fenêtres ouvertes
   passent juste derrière elle (`orderWindow:relativeTo:`), sauf la visualisation en plein écran. Les fenêtres
   « enfants » de macOS (`addChildWindow`) sont écartées : macOS les déplacerait lui-même, en concurrence avec
   l'aimantation et le groupe.
5. **Demi-crans** : ×1, ×1,5, ×2, ×2,5, ×3, et automatique. Réglage en pour cent (`window_scale` : 100 à 300) ;
   l'ancien `window_zoom` est relu s'il est seul et tenu à jour (partie entière) pour les anciennes versions.
   - Disposition en **demi-points** sur macOS (275 × 1,5 = 412,5 points) ; les positions des anciennes versions,
     en points, sont converties une fois (`layout_units`).
   - Sur un écran Retina, ×1,5 = ×3 pixels réels : net. Sur un écran non Retina (ou Windows à 100 %), ×1,5 reste
     proposé, au plus proche voisin, avec des pixels inégaux (choix de Guillaume, plutôt que griser ou lisser).
   - **Automatique** : ×1 sous 1 400 points de large ; ×1,5 jusqu'à 2 100 points sur un écran où ×1,5 tombe sur
     des pixels entiers (Retina), ×2 sinon ; ×2 jusqu'à 2 800 ; ×3 au-delà.
   - **Cmd+D (Ctrl+D)** reste la « double taille » de Winamp : ×1 ↔ ×2 ; depuis un autre agrandissement, retour à
     ×1. La coche « Double taille » et le bouton D sont allumés dès que les fenêtres sont agrandies.

## Conséquences

- Les fenêtres restent collées d'un lancement à l'autre, même après un changement de taille.
- La relecture des positions ne vaut que sur macOS ; ailleurs, le modèle reste la référence.
- Revenir à la 0.6.0 après la 0.6.1 peut ouvrir les fenêtres ailleurs une fois (positions en demi-points) : elles
  reviennent à leur place par défaut si elles tombent hors de l'écran.
