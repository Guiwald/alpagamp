# ADR 0006 — Interface sur téléphone : fidèle par défaut, mode adapté en option

- Statut : acceptée (13/10/2026)

## Contexte

L'interface classique fait 275 px de large. À l'échelle d'un téléphone (×3,5 à ×4), les boutons de transport font
~5 mm (utilisables), mais les petits boutons de la barre de titre et la clutterbar ~2 mm (trop petits au doigt).

## Décision

- **Mode A, « fidèle » (par défaut)** : fenêtre principale, égaliseur et playlist empilés et mis à l'échelle de la
  largeur, la playlist remplissant le reste de l'écran. Zones tactiles agrandies de façon invisible, appui long =
  clic droit (menus), double-tap sur le visualiseur = MilkDrop en plein écran.
- **Mode B, « adapté »** : gros contrôles construits à partir des sprites du skin. Proposé comme **option de
  réglage** pour basculer selon les goûts (calendrier à trancher).
- Dans les deux modes : lecture en arrière-plan, contrôles dans la notification et sur l'écran verrouillé, boutons
  du casque.
