#!/bin/bash
# Construit Alpagamp pour macOS (Apple Silicon ou Intel) à partir de alpagamp.bundle, placé à côté de ce script.
# Usage : bash construire-mac.sh
set -euo pipefail
ICI="$(cd "$(dirname "$0")" && pwd)"
cd "$ICI"

echo "== 1/4 Outils d'Apple"
if ! xcode-select -p >/dev/null 2>&1; then
  xcode-select --install || true
  echo "Une fenêtre propose d'installer les outils de ligne de commande : accepte, attends la fin, puis relance ce script."
  exit 1
fi

echo "== 2/4 Rust"
if [ -f "$HOME/.cargo/env" ]; then . "$HOME/.cargo/env"; fi
if ! command -v cargo >/dev/null 2>&1; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
  . "$HOME/.cargo/env"
fi
rustup update stable --no-self-update >/dev/null 2>&1 || true

echo "== 3/4 Code source"
if [ -d alpagamp/.git ]; then
  git -C alpagamp fetch -q "$ICI/alpagamp.bundle" main
  git -C alpagamp checkout -q -f -B main FETCH_HEAD
else
  git clone -q -b main "$ICI/alpagamp.bundle" alpagamp
fi
cd alpagamp
git fetch -q "$ICI/alpagamp.bundle" 'refs/tags/*:refs/tags/*' || true
VERSION="$(cat VERSION)"
echo "Alpagamp $VERSION"

echo "== 4/4 Compilation (5 à 10 minutes la première fois)"
cargo build --release -p alpagamp
mkdir -p "$ICI/dist"
if [ -e "$ICI/dist/Alpagamp-$VERSION.dmg" ]; then
  echo "dist/Alpagamp-$VERSION.dmg existe déjà : rien à refaire."
else
  cargo xtask bundle-mac target/release/alpagamp "$ICI/dist"
fi
echo
echo "Terminé : $ICI/dist/Alpagamp-$VERSION.dmg"
echo "Ouvre le .dmg et glisse Alpagamp dans Applications."
open "$ICI/dist" || true
