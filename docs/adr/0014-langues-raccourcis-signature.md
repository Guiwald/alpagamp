# ADR 0014 — Langues, raccourcis Mac, mode adapté, signature des applis

- Statut : acceptée (13/10/2026)

## Langues

Menus, préférences et messages en **anglais, français, espagnol, allemand, italien, valencien et irlandais**,
choisis selon la langue du système et modifiables dans les réglages. Les libellés dessinés dans les skins restent
tels quels. Traductions faites par Claude ; une relecture par un locuteur natif est souhaitable pour le valencien et
l'irlandais.

## Raccourcis sur macOS

Touches seules identiques à Winamp (Z X C V B, L, J, R, S, chiffres et Q…P de l'égaliseur) ; Ctrl → Cmd ; Alt → Option ;
les combinaisons réservées par macOS (Cmd+Q, Cmd+H, Cmd+M, Cmd+W) gardent leur sens système, et la fonction Winamp
correspondante est déplacée (ex. bandeau : Cmd+Shift+W). Vraie barre de menus macOS reprenant le menu de Winamp.

## Mode adapté sur téléphone

La v1 livre le mode fidèle et l'interrupteur dans les réglages ; le **mode adapté arrive en 1.1** (ADR 0006).

## Signature

- Pas de certificat payant pour l'instant (Apple 99 $/an, Windows) : sur Mac, premier lancement via Réglages
  Système › Confidentialité et sécurité › « Ouvrir quand même » ; sur Windows, « Informations complémentaires ›
  Exécuter quand même ».
- Android : APK signé avec une clé à nous, stockée chiffrée dans les secrets GitHub ; **copie de secours chez
  Guillaume** (gestionnaire de mots de passe). Perdre la clé = devoir désinstaller pour mettre à jour.
