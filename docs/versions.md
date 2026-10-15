# Versions d'Alpagamp

Convention commune aux outils de Guillaume (projet Outils, `conventions-versions.md`), appliquée ici.

## Numéro

`MAJEUR.MINEUR.CORRECTIF`, source unique : le fichier `VERSION` à la racine.

- **MAJEUR** : une action de l'utilisateur est nécessaire (désinstaller, refaire un réglage), ou une ancienne version
  cesse de marcher. Pour Alpagamp : aussi tout changement incompatible de l'API des plugins.
- **MINEUR** : nouvelle fonction, rien à faire.
- **CORRECTIF** : correction de bug uniquement.

Les préversions sont en `0.x` ; la `1.0.0` sort quand tout le périmètre de l'ADR 0005 est livré et testé.

## Règles

1. Une version publiée ne change jamais : les commandes de paquet refusent d'écraser un fichier existant de `dist/`.
2. Les fichiers livrés portent la version : `Alpagamp-X.Y.Z.dmg`, `Alpagamp-X.Y.Z.apk`,
   `Alpagamp-X.Y.Z-windows-x64.zip`.
3. `CHANGELOG.md` au format Keep a Changelog ; chaque version porte un paragraphe « Mise en service » (ce qu'il faut
   faire pour l'installer) et chaque entrée commence par son composant. `CHANGELOG.en.md` en est la traduction
   anglaise, section par section, avec les mêmes titres de version (« Rollout » pour « Mise en service ») : les
   notes de chaque version sur GitHub donnent le français puis l'anglais (`scripts/notes-de-version.sh`).
4. Android : `versionCode = MAJEUR×10000 + MINEUR×100 + CORRECTIF`. macOS : `CFBundleVersion` idem.
5. Page des versions : artefact claude.ai privé généré depuis `CHANGELOG.md` par `cargo xtask changelog-page`,
   republié à la même URL à chaque version : [Versions d'Alpagamp](https://claude.ai/artifact/D4bgt4MWwXFCZo7SRu6F54)
   (depuis une autre conversation, passer cette URL comme `url` à la publication).

## Sortir une version

1. `cargo xtask bump patch|minor|major` (met à jour `VERSION`, les `Cargo.toml`, date la section du CHANGELOG).
2. Compléter le paragraphe « Mise en service » de la nouvelle section, et la même section en anglais dans
   `CHANGELOG.en.md` (« Rollout », Added / Changed / Fixed).
3. Construire les paquets (`cargo xtask dist-windows`, `cargo xtask bundle-mac …`, APK) ou laisser la CI le faire sur
   l'étiquette `vX.Y.Z`.
4. Étiqueter (`git tag vX.Y.Z`), pousser, joindre les paquets à la release GitHub.
5. `cargo xtask changelog-page` puis republier la page des versions.
