# ADR 0007 — Plateformes et définition du « livrable »

- Statut : acceptée (13/10/2026)

## Décision

| Plateforme | Livrable v1 | Test |
|---|---|---|
| **macOS** | `Nom-X.Y.Z.dmg` | sur le Mac de Guillaume (Apple Silicon) |
| **Android** | `Nom-X.Y.Z.apk` | sur son Pixel 8 |
| **Windows** | `Nom-X.Y.Z-windows-x64.zip` | CI + Naku, un ami de Guillaume |
| **iPhone** | mis de côté pour l'instant | — |

- **Windows sort tôt** (dès les premiers jalons jouables) pour que Guillaume puisse le montrer à Naku.
- Ordre : Mac → Android → Windows en parallèle dès que possible ; iPhone plus tard.
- Le code reste compatible iOS (pas de choix qui l'exclurait), pour pouvoir le reprendre sans réécriture.
