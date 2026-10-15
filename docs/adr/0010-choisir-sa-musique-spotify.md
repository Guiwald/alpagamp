# ADR 0010 — Choisir sa musique Spotify, sans configuration

- Statut : acceptée (13/10/2026), révisée par l'ADR 0022 (15/10/2026, 0.6.2 : navigation par les canaux internes,
  option avancée reportée)

## Contexte

Trois façons de choisir la musique Spotify ont été retenues. Guillaume veut aussi que l'installation reste
**simple pour d'autres personnes**. Or une appli développeur Spotify en mode développement est limitée à
5 utilisateurs ajoutés à la main par son propriétaire, et le passage en quota étendu est réservé aux
organisations à 250 000 utilisateurs actifs : on ne peut pas faire reposer le lecteur sur l'appli développeur
de Guillaume.

## Décision

1. **Fenêtre « Bibliothèque »** (habillée par le skin, style des fenêtres génériques) : Mes playlists, Titres likés,
   Albums, Recherche, Radios (annuaire libre radio-browser.info), Dossiers locaux. Double-clic = jouer,
   glisser = ajouter à la playlist. Embryon de la future Media Library.
2. **Spotify dans la playlist** : entrées Spotify mêlées aux fichiers locaux ; LIST › Charger propose les playlists
   Spotify ; ADD › URL accepte un lien Spotify ; les playlists enregistrées gardent les URI `spotify:`.
3. **Spotify Connect** : le lecteur apparaît comme enceinte dans l'appli Spotify.

**Zéro configuration par défaut** : une seule connexion (« Se connecter à Spotify » → navigateur → « Autoriser »).
- L'audio et Spotify Connect passent par la session librespot.
- La navigation (playlists, titres likés, recherche) utilise le jeton de cette même connexion ; ce qui peut passer
  par les canaux internes de librespot (liste des playlists, contenu des playlists) y passe.
- **Option avancée** : renseigner l'identifiant de sa propre appli développeur Spotify, si Spotify limite les
  requêtes. Guillaume pourra s'en créer une ; personne d'autre n'y sera obligé.

## Conséquences

- La faisabilité exacte (quels appels passent avec le jeton de la session) est vérifiée par un prototype au début
  du jalon Spotify ; l'architecture isole ces appels derrière une interface pour pouvoir changer de canal.
