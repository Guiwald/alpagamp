# ADR 0017 — Android : NativeActivity sans Java, APK construit sans Gradle

- Statut : acceptée (13/10/2026)

## Contexte

L'ADR 0004 veut toute la logique en Rust et n'admet du Java que s'il est inévitable. Une appli Android a besoin
d'une activité, d'une surface où dessiner, du son, de la permission de lire la musique, d'un clavier pour taper
une adresse ; plus tard (0.7) de la lecture en arrière-plan avec notification.

## Décision

1. **`android.app.NativeActivity`** (fournie par Android) charge `libalpagamp.so` ; winit et softbuffer dessinent
   dans sa surface, cpal joue par AAudio. Pas une ligne de Java dans la 0.3 : permissions, fin de l'activité et
   clavier passent par JNI depuis Rust.
2. **Mode fidèle** (ADR 0006) : les trois fenêtres empilées à la largeur de l'écran, la playlist remplit le
   reste. Appui long = menu ; les menus et boîtes sont des fenêtres génériques skinnées en surimpression.
3. **APK construit par `cargo xtask apk`** sans Gradle : `aapt2` (manifeste, icônes), bibliothèque ajoutée non
   compressée, `zipalign -P 16` (pages de 16 Ko, exigées par Android 15+), `apksigner`. Identifiant
   `fr.guiwald.alpagamp`, `versionCode` dérivé de la version (convention).
4. **`targetSdkVersion` 34** pour l'instant : les applis qui visent 35+ sont dessinées sous les barres du système
   (« edge-to-edge » imposé) ; on passera à la cible récente avec la gestion des marges (0.7).
5. **Musique du téléphone** : permission `READ_MEDIA_AUDIO`, puis lecture directe des dossiers Music, Download,
   Podcasts… Le sélecteur de fichiers du système (SAF) viendra avec la 0.7.
6. **Clé de signature** : clé RSA 4096 propre au projet, créée le 13/10/2026 (empreinte SHA-256 `22:1D:C9:09:…:7B:DC:4A`).
   Elle va dans les secrets GitHub (`ALPAGAMP_KEYSTORE`, `ALPAGAMP_KEYSTORE_PASS`) et en copie de secours chez
   Guillaume. Sans elle, une mise à jour impose de désinstaller l'appli.
7. **Simulateur** : la même appli tourne sur ordinateur (`cargo run -p alpagamp-android --bin phone-sim`, souris
   pour doigt) pour tester la disposition et les gestes sans téléphone.

## Conséquences

- La lecture continue quand l'appli passe en arrière-plan, mais Android peut l'arrêter : le service de lecture
  avec notification (0.7) demandera probablement quelques lignes de Java (ADR 0004).
- Les skins sur téléphone arrivent avec le navigateur de skins (0.5).
