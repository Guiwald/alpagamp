# ADR 0022 — Bibliothèque Spotify par les canaux internes, listes gardées, attente de Spotify, connexion relançable

- Statut : acceptée (15/10/2026, 0.6.2)
- Révise : ADR 0010 (canal de la navigation ; « option avancée » reportée)

## Contexte

Premiers essais de Spotify sur le Mac de Guillaume (0.6.1, 15/10) :

1. Juste après la connexion, la Bibliothèque affiche « Spotify demande de patienter (trop de requêtes) » : c'est la
   réponse **429** de l'API web (`api.spotify.com`), par laquelle passait toute la navigation (playlists, titres
   likés, albums, recherche).
2. Fermer l'onglet du navigateur avant d'avoir autorisé bloque la connexion : `login()` ne fait rien tant que
   l'état est « autorisation en cours », et l'attente dure 5 minutes.
3. Hors Spotify : un morceau sur un disque externe saute (1 à 2 s de silence toutes les 7 à 8 s), et « File info »
   fige le lecteur.

Ce qu'on sait du 429 :

- Le jeton vient de l'identifiant client du lecteur de bureau de Spotify, celui de librespot, partagé par tous les
  lecteurs basés sur librespot. Depuis janvier 2026, Spotify bride les appels de l'API web faits avec ces jetons :
  go-librespot (janvier 2026), spotify-player (septembre 2026), un client PSP qui est passé aux canaux internes
  pour cette raison. Les `Retry-After` vont de quelques secondes à plusieurs heures.
- Alpagamp aggravait les choses : `Retry-After` ignoré, aucune liste gardée (chaque changement d'onglet relançait
  les requêtes), pagination arrêtée à la première erreur.
- Depuis mars 2026, une appli développeur en mode « développement » exige un propriétaire Premium, 5 utilisateurs
  au plus, 10 résultats par recherche, et ne lit que les playlists qu'on possède ou où l'on collabore (pas celles
  qu'on suit). Depuis juillet 2026, ses quotas sont comptés par compte développeur.
- La lecture et Spotify Connect passent par la session librespot : le 429 ne les touche pas.

## Décision

1. **Canaux internes d'abord** (ceux des applis de Spotify, par la session librespot, jeton « login5 » et
   « client-token ») :

   | Liste | Canal |
   |---|---|
   | Mes playlists (avec les dossiers, aplatis) | `/playlist/v2/user/<moi>/rootlist` (protobuf `SelectedListContent`) |
   | Contenu d'une playlist | `/playlist/v2/playlist/<id>` |
   | Titres likés, albums enregistrés | `/collection/v2/paging`, ensemble « collection » (les deux y sont, distingués par l'adresse) |
   | Noms, artistes, durées, morceaux d'un album | `/extended-metadata/v0/extended-metadata`, par paquets de 100 |
   | Recherche de titres | `/context-resolve/v1/spotify:search:<requête>` |
   | Recherche d'albums et de playlists | API web (seul cas restant), si elle ne demande pas de patienter |

   Les messages `collection2v2` (que librespot ne compile pas) sont codés à la main ; les autres viennent de
   `librespot-protocol`, décodés sans exiger les champs « required » (un champ disparu ne casse pas une liste).
2. **Requêtes faites à la main** sur le spclient (adresse, jetons et limite de débit fournis par librespot) pour
   lire `Retry-After` : librespot attend lui-même jusqu'à 10 s, puis jette l'en-tête.
3. **Listes gardées sur disque** (`<dossier Spotify>/bibliotheque/*.json`) : affichées tout de suite, même hors
   ligne ; redemandées seulement si elles ont plus de **15 min**, ou par la ligne **Actualiser** (avec l'âge de la
   liste). Pendant qu'une liste gardée trop vieille est redemandée, elle reste affichée ; la nouvelle la remplace
   sans perdre le défilement ni la sélection. Tout est effacé à la déconnexion.
4. **Attente de Spotify** : un 429 fixe une échéance par canal (« interne », « web » ; 60 s si `Retry-After`
   manque), gardée sur disque. Avant l'échéance, rien n'est redemandé à ce canal, même si on insiste ; la
   Bibliothèque affiche un **compte à rebours** au-dessus de la dernière liste reçue (« réessaie dans 11:42 ») ;
   à l'échéance, la ligne devient « Réessayer maintenant ». Pas de nouvel essai automatique.
5. **Identifiant d'appli personnel** (l'« option avancée » de l'ADR 0010) : **pas dans la 0.6.2**. Avec les canaux
   internes, il ne servirait plus qu'à chercher des albums et des playlists, avec les limites du mode
   développement. Reporté à la 1.1, à reprendre si la recherche limitée aux titres gêne à l'usage.
6. **Connexion relançable** : la tentative en cours garde l'adresse de la page et un drapeau d'annulation vérifié
   toutes les 50 ms. Pendant l'attente, la Bibliothèque et le menu Spotify proposent « Rouvrir la page de
   connexion » (même page, même attente) et « Annuler » ; un nouveau clic sur « Se connecter » rouvre la page.
   Au bout de 5 min sans réponse : retour à « Se connecter », sans message d'erreur. Même chose sur téléphone.
7. **Diagnostic** : `alpagamp --diagnostic-spotify [rapport.txt]` fait une poignée de requêtes en lecture seule
   sur chaque canal ; le rapport ne contient que des codes de réponse, des attentes, des nombres et des durées.
8. **Lecture d'avance des fichiers locaux** (`ReadAhead`) : un fil lit le fichier jusqu'à 16 Mo devant le décodeur
   (le morceau entier, le plus souvent) et garde 2 Mo derrière ; un saut hors de cette fenêtre relance la lecture
   à la nouvelle position. Les lectures de titres et de durées (`probe_file`) restent directes. « File info »
   lit le fichier dans un fil.

## Conséquences

- Les canaux internes ne sont pas documentés : Spotify peut les changer, comme ceux dont librespot dépend déjà pour
  jouer. Le diagnostic dit lequel ne répond plus.
- Jusqu'à 10 000 éléments par liste ; une longue liste s'affiche au fur et à mesure (« Chargement… 500/2 345 »).
- La recherche d'albums et de playlists dépend encore de l'API web : quand elle demande de patienter, la recherche
  ne rend que des titres, avec l'attente affichée.
- Le dépôt gagne `librespot-protocol`, `protobuf`, `http`, `bytes` et `http-body-util`, déjà compilés pour
  librespot : rien de neuf à construire.
