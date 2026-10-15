#!/bin/bash
# Notes d'une version pour sa page GitHub : la section de CHANGELOG.md (français), puis celle de CHANGELOG.en.md
# (anglais), puis scripts/notes-installation.md (français et anglais).
#
# Usage : bash scripts/notes-de-version.sh 0.6.2 [dossier du dépôt]
# ALPAGAMP_NOTE_MAC=1 ajoute la note sur le premier .dmg (construit sur un Mac à puce Apple, remplacé par GitHub).
set -euo pipefail
V="$1"
RACINE="${2:-$(cd "$(dirname "$0")/.." && pwd)}"

# La section « ## [V] … » d'un journal, sans son titre ni les lignes vides du début.
section() {
  [ -f "$1" ] || return 0
  awk -v v="$V" 'index($0, "## [" v "]") == 1 {f=1; next} /^## \[/ {if (f) exit} f' "$1" | sed '/./,$!d'
}

FR="$(section "$RACINE/CHANGELOG.md")"
EN="$(section "$RACINE/CHANGELOG.en.md")"
if [ -z "$FR" ]; then
  echo "Pas de section [$V] dans CHANGELOG.md" >&2
  exit 1
fi

if [ -n "$EN" ]; then
  printf '*English below.*\n\n'
fi
printf '%s\n' "$FR"
if [ -n "$EN" ]; then
  printf '\n---\n\n## English\n\n%s\n' "$EN"
fi
if [ "${ALPAGAMP_NOTE_MAC:-0}" = 1 ]; then
  printf '\n> **Mac** : le premier .dmg mis en ligne est construit sur un Mac à puce Apple (M1, M2, M3, M4…) ; GitHub le\n> remplace en une vingtaine de minutes par un .dmg universel, qui marche aussi sur les Mac Intel.\n>\n> **Mac**: the first .dmg uploaded is built on an Apple silicon Mac (M1, M2, M3, M4…); GitHub replaces it within\n> about twenty minutes with a universal .dmg, which also works on Intel Macs.\n'
fi
cat "$RACINE/scripts/notes-installation.md"
