#!/bin/bash
# Publie Alpagamp sur GitHub : crée le dépôt public (la première fois), envoie le code et les étiquettes,
# enregistre la clé de signature Android dans les secrets du dépôt, et publie la version avec ses fichiers.
#
# Usage, depuis le dossier qui contient alpagamp.bundle :   bash publier-sur-github.sh
#
# La connexion à GitHub se fait dans le navigateur, avec ton compte (aucun jeton à copier).
set -euo pipefail
ICI="$(cd "$(dirname "$0")" && pwd)"
cd "$ICI"
DEPOT="alpagamp"

echo "== 1/6 Outil GitHub (gh)"
GH=""
if command -v gh >/dev/null 2>&1; then
  GH="gh"
elif command -v brew >/dev/null 2>&1; then
  brew install gh
  GH="gh"
else
  case "$(uname -m)" in arm64) ARCH=arm64 ;; *) ARCH=amd64 ;; esac
  URL="$(curl -fsSL https://api.github.com/repos/cli/cli/releases/latest | grep -o "https://[^\"]*_macOS_${ARCH}\.zip" | head -1 || true)"
  if [ -z "$URL" ]; then
    echo "Impossible de télécharger l'outil gh. Installe-le depuis https://cli.github.com puis relance ce script."
    exit 1
  fi
  rm -rf "$ICI/.outils/gh" && mkdir -p "$ICI/.outils/gh"
  curl -fsSL "$URL" -o "$ICI/.outils/gh.zip"
  unzip -q "$ICI/.outils/gh.zip" -d "$ICI/.outils/gh"
  GH="$(find "$ICI/.outils/gh" -type f -name gh -perm -u+x | head -1)"
fi
echo "gh : $("$GH" --version | head -1)"

echo "== 2/6 Connexion à GitHub"
if ! "$GH" auth status -h github.com >/dev/null 2>&1; then
  echo "gh va afficher un code de 8 caractères puis ouvrir GitHub dans ton navigateur :"
  echo "appuie sur Entrée, colle le code dans la page, puis clique sur « Authorize »."
  "$GH" auth login -h github.com -p https -w -s workflow
elif ! "$GH" auth status -h github.com 2>&1 | grep -q "workflow"; then
  # Il faut le droit « workflow » pour envoyer les fichiers de compilation automatique (.github/workflows).
  "$GH" auth refresh -h github.com -s workflow
fi
"$GH" auth setup-git -h github.com
MOI="$("$GH" api user -q .login)"
echo "Connecté en tant que $MOI"

echo "== 3/6 Code source"
if [ -d alpagamp/.git ]; then
  git -C alpagamp fetch -q "$ICI/alpagamp.bundle" main
  git -C alpagamp checkout -q -B main FETCH_HEAD
else
  git clone -q -b main "$ICI/alpagamp.bundle" alpagamp
fi
git -C alpagamp fetch -q --tags "$ICI/alpagamp.bundle"
VERSION="$(cat alpagamp/VERSION)"
TAG="v$VERSION"
echo "Alpagamp $VERSION"

echo "== 4/6 Dépôt https://github.com/$MOI/$DEPOT"
if ! "$GH" repo view "$MOI/$DEPOT" >/dev/null 2>&1; then
  "$GH" repo create "$MOI/$DEPOT" --public \
    --description "Lecteur de musique à skins classiques, en Rust : Mac, Windows, Android, Spotify, radios" \
    --homepage "https://github.com/$MOI/$DEPOT/releases/latest"
fi
cd alpagamp
git remote remove origin >/dev/null 2>&1 || true
git remote add origin "https://github.com/$MOI/$DEPOT.git"
git push -u origin main
git push origin --tags
cd "$ICI"

echo "== 5/6 Clé de signature Android"
CLE="$ICI/signature-android/alpagamp-release.p12"
MDP="$ICI/signature-android/mot-de-passe.txt"
if [ -f "$CLE" ] && [ -f "$MDP" ]; then
  base64 -i "$CLE" | tr -d '\n' | "$GH" secret set ANDROID_KEYSTORE_B64 -R "$MOI/$DEPOT"
  printf %s "$(cat "$MDP")" | "$GH" secret set ANDROID_KEYSTORE_PASSWORD -R "$MOI/$DEPOT"
  echo "Clé enregistrée dans les secrets du dépôt (GitHub signera les prochains APK avec elle)."
else
  echo "Pas de dossier signature-android : secrets inchangés."
fi

echo "== 6/6 Version $VERSION"
FICHIERS=()
for f in "$ICI/dist/Alpagamp-$VERSION.dmg" "$ICI/publication/Alpagamp-$VERSION-windows-x64.zip" "$ICI/publication/Alpagamp-$VERSION.apk"; do
  if [ -f "$f" ]; then FICHIERS+=("$f"); fi
done
if "$GH" release view "$TAG" -R "$MOI/$DEPOT" >/dev/null 2>&1; then
  if [ ${#FICHIERS[@]} -gt 0 ]; then
    "$GH" release upload "$TAG" "${FICHIERS[@]}" --clobber -R "$MOI/$DEPOT"
  fi
  echo "La version $VERSION existait déjà : fichiers mis à jour."
elif [ ${#FICHIERS[@]} -gt 0 ]; then
  NOTES="$(mktemp)"
  awk -v v="$VERSION" 'index($0, "## [" v "]") == 1 {f=1; next} /^## \[/ {if (f) exit} f' alpagamp/CHANGELOG.md > "$NOTES"
  if [ -e "$ICI/dist/Alpagamp-$VERSION.dmg" ] && [ "$(uname -m)" = "arm64" ]; then
    printf '\n> **Mac** : ce .dmg a été construit sur un Mac à puce Apple ; il ne marche que sur ces Mac (M1, M2, M3, M4…).\n> Les versions suivantes, fabriquées par GitHub, marcheront aussi sur les Mac Intel.\n' >> "$NOTES"
  fi
  cat alpagamp/scripts/notes-installation.md >> "$NOTES"
  "$GH" release create "$TAG" "${FICHIERS[@]}" -R "$MOI/$DEPOT" --title "Alpagamp $VERSION" --notes-file "$NOTES"
  rm -f "$NOTES"
else
  # Pas de fichiers prêts ici : GitHub les fabrique (flux « Publication »).
  "$GH" workflow run release.yml -R "$MOI/$DEPOT" -f tag="$TAG"
  echo "GitHub fabrique les fichiers de la version (environ 20 minutes)."
fi

echo
echo "Terminé !"
echo "  Le code :              https://github.com/$MOI/$DEPOT"
echo "  Les téléchargements :  https://github.com/$MOI/$DEPOT/releases/latest"
echo "  La compilation auto :  https://github.com/$MOI/$DEPOT/actions"
open "https://github.com/$MOI/$DEPOT/releases/latest" >/dev/null 2>&1 || true
