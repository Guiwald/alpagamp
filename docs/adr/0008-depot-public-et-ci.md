# ADR 0008 — Dépôt GitHub public, compilation par GitHub Actions

- Statut : acceptée (13/10/2026)

## Contexte

Les applis Mac ne se compilent que sur macOS ; l'environnement de travail de Claude est un Linux et le shell
disponible sur le Mac de Guillaume est une VM Linux. GitHub Actions fournit des machines macOS, Windows et Linux :
gratuites et illimitées pour un dépôt public, 2 000 minutes/mois pour un dépôt privé (les minutes macOS coûtant
environ 6 fois plus).

## Décision

- Un **dépôt séparé et public** `Guiwald/<nom>` (hors du dépôt du vault).
- CI : Linux, Android, Windows et macOS compilés et testés à chaque push ; les versions publiées comme
  **GitHub Releases** avec des fichiers nommés `Nom-X.Y.Z.ext`.
- Précautions liées au caractère public :
  - aucun secret dans le dépôt (jetons, identifiants : jamais versionnés) ;
  - **aucun skin, preset ni fichier audio tiers versionné** : le skin par défaut est original ; les presets MilkDrop
    et skins tiers sont téléchargés à la demande ou importés par l'utilisateur ; les tests utilisent des fichiers
    générés ;
  - commits signés de l'adresse « noreply » de GitHub pour ne pas exposer l'adresse e-mail personnelle.
- Convention de versions du projet Outils : `VERSION`, `CHANGELOG.md` (Keep a Changelog), SemVer, versionCode
  Android dérivé, page des versions en artefact.
