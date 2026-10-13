# ADR 0018 — Spotify dans le moteur d'Alpagamp ; TLS de librespot

- Statut : acceptée (13/10/2026)

## Contexte

L'ADR 0002 retient librespot ; l'ADR 0010 veut une seule connexion, sans configuration, une fenêtre Bibliothèque,
Spotify dans la playlist et Spotify Connect. librespot pousse son audio vers une « sortie » ; le moteur d'Alpagamp
tire l'audio de ses sources. librespot passe par hyper, reqwest et tungstenite, dont le TLS est soit celui du
système (native-tls), soit rustls avec la cryptographie `ring` (Rust avec du C et de l'assembleur).

## Décision

1. **Chaque morceau Spotify est une source du moteur** : un lecteur librespot par morceau écrit dans un tampon
   (`BufferSink`), la source (`BufferSource`) le lit. Égaliseur, ReplayGain, visualisation, fondu enchaîné et
   enchaînement s'appliquent comme aux fichiers. 320 kbit/s.
2. **Spotify Connect** : un lecteur librespot dédié, piloté par Spirc, écrit dans un autre tampon. Quand l'appli
   Spotify choisit Alpagamp, le lecteur joue cette source « hors playlist » ; les boutons de transport sont
   renvoyés à Spotify ; changer de morceau sur le téléphone remet la position à zéro.
3. **Connexion** : flux OAuth PKCE fait par Alpagamp (navigateur → retour sur `127.0.0.1`), avec l'identifiant
   client public du lecteur de bureau de Spotify, comme librespot ; ouverture du navigateur par le système (et
   par un Intent sur Android). Les identifiants réutilisables sont ensuite gardés par librespot dans le dossier
   de l'appli ; le jeton de renouvellement sert à l'API web.
4. **Bibliothèque** par l'API web (playlists, titres likés, albums, recherche) et radio-browser.info (radios).
5. **TLS de librespot** : celui du système sur macOS et Windows (Security, SChannel : pas de C à compiler) ;
   rustls avec `ring` sur Android et Linux. **Écart assumé à l'ADR 0004** pour Android : `ring` est la seule
   cryptographie que librespot sait utiliser sans bibliothèque système ; corriger librespot pour graviola
   demanderait une copie maintenue. Le reste d'Alpagamp (radios, connexion, API web) garde rustls + graviola.

## Conséquences

- Compte Spotify Premium obligatoire ; un compte gratuit reçoit un message clair.
- Rien de tout ça ne peut être essayé sans compte : les essais se font chez Guillaume (Mac, Pixel 8).
- La bibliothèque native fait ~17 Mo sur Android (librespot, deux versions de Symphonia).
