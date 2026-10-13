# Tiers

Alpagamp est une réimplémentation propre (voir `docs/adr/0001-reimplementation-propre.md`). Aucun code de Winamp
n'y figure. Quelques faits viennent de projets sous licence permissive, et la visualisation utilise des bibliothèques
qui en contiennent :

## Webamp — MIT

Les coordonnées des sprites des skins classiques (`crates/skin/src/sprites.rs`) recoupent celles documentées par
Webamp (`packages/webamp/js/skinSprites.ts`).

```
Copyright (c) 2015 Jordan Eldredge

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated
documentation files (the "Software"), to deal in the Software without restriction, including without limitation the
rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit
persons to whom the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial portions of the
Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE
WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR
OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
```

## Marques

« Winamp » est une marque de Winamp Group SA. Alpagamp n'est ni affilié à Winamp Group, ni approuvé par lui.
Les skins `.wsz` tiers appartiennent à leurs auteurs ; aucun n'est distribué avec Alpagamp.

## Liberation Sans — SIL Open Font License 1.1

Police de la playlist et des fenêtres génériques (métriques compatibles Arial, la police par défaut des skins) :
`assets/fonts/LiberationSans-Regular.ttf`, licence complète dans `assets/fonts/LICENSE-OFL.txt`.
Copyright (c) 2010 Google Corporation ; Copyright (c) 2012 Red Hat, Inc. (Reserved Font Name « Liberation »).

## Visualisation : lactea — MIT OR Apache-2.0, avec du code de MilkDrop 2 (BSD-3-Clause)

Le moteur de la visualisation est lactea 0.2.1 (https://github.com/M0Rf30/lactea, ADR 0019), sous licence
MIT OR Apache-2.0. Il contient un portage du rendu de MilkDrop 2 et son fichier `include.fx`, sous la licence
suivante :

```
Copyright 2005-2013 Nullsoft, Inc.
All rights reserved.

Redistribution and use in source and binary forms, with or without modification,
are permitted provided that the following conditions are met:

  * Redistributions of source code must retain the above copyright notice,
    this list of conditions and the following disclaimer.

  * Redistributions in binary form must reproduce the above copyright notice,
    this list of conditions and the following disclaimer in the documentation
    and/or other materials provided with the distribution.

  * Neither the name of Nullsoft nor the names of its contributors may be used to
    endorse or promote products derived from this software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY EXPRESS OR
IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND
FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT OWNER OR
CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER
IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT
OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
```

lactea contient aussi des portages de BeatDrop (Maxim Volskiy et contributeurs, BSD-3-Clause, même texte avec
« Copyright (c) 2018 Maxim Volskiy and individual contributors »), de stb_image (Sean Barrett, domaine public / MIT)
et des versions modifiées de ns-eel2 (Cockos / Nullsoft, licence de type zlib : « Expression Evaluator Library
(NS-EEL) v2 — Copyright (C) 2004-2008 Cockos Incorporated, Copyright (C) 1999-2003 Nullsoft, Inc. »). Détails :
`THIRD_PARTY_NOTICES.md` de lactea.

## Presets de visualisation

Les six presets « Alpagamp - … » sont originaux (licence MIT du projet). Le pack *Cream of the Crop* (sélection
d'ISOSCELES, dépôt `projectM-visualizer/presets-cream-of-the-crop`) n'est jamais distribué : l'appli le télécharge
à la demande ; ses presets appartiennent à leurs auteurs.

« MilkDrop » est cité pour décrire la compatibilité des presets ; Alpagamp n'est affilié ni à Nullsoft, ni à
Ryan Geiss, ni au projet projectM.
