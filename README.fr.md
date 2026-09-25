<div align="center">

# 🎧 Breezer

**Client Deezer natif, ultra-léger et open-source.**

Linux · macOS · Windows — construit avec **Rust** et **Slint**, minimaliste par passion.

![License](https://img.shields.io/badge/licence-AGPL--3.0-bleu) ![Rust](https://img.shields.io/badge/Rust-1.85+-orange) ![Plateformes](https://img.shields.io/badge/plateformes-Linux%20%7C%20macOS%20%7C%20Windows-lightgrey)

</div>

---

## ✨ Fonctionnalités

- 🧊 **Léger** — petit binaire, petite empreinte RAM. Pas d'Electron, pas de bloat.
- 🎨 **Hautement personnalisable** — surcharge manuelle des couleurs, **système de thèmes** complet (palette + layout + typographie), thèmes communautaires.
- 🧩 **Layout modulaire** — panneaux ancrables (docks), glisser-déposer, espaces de travail (*Écoute*, *Parcourir*, *Compact*) + vos profils personnalisés.
- 🔌 **Prêt pour les plugins** — API publique stable dès le premier jour (logique + points d'extension du shell).
- 🔎 Recherche Deezer, playlists, favoris, métadonnées (API publique + API authentifiée).
- 🎵 Lecture réelle des pistes gratuites (basée sur l'ARL, déchiffrement AES isolé et auditable).
- 🌍 **i18n** : français & anglais (d'autres langues à venir).
- 🔄 **Mise à jour automatique** depuis les GitHub Releases, versionnage sémantique.
- 🔓 **AGPL-3.0** — un logiciel libre, toujours.

## 📸 Captures d'écran

> 🖼️ **Capture d'écran demandée — appel à la communauté.**
> Nous cherchons des volontaires pour faire tourner Breezer sur leur plateforme (Linux, macOS, Windows) et partager des captures (thème par défaut + au moins un thème communautaire). Chaque capture sera créditée dans le README et sur le site.
> Voir [docs/SCREENSHOTS.md](docs/SCREENSHOTS.md) pour la liste exacte et comment contribuer (FR/EN bienvenus).

*Les captures apparaîtront ici dès qu'elles seront fournies.*

## 🚀 Démarrage rapide (depuis les sources)

```bash
# Prérequis : Rust stable (1.85+), Linux : libasound2-dev, libxkbcommon-dev, cmake + clang
cd software
cargo run --release
```

Au premier lancement : ouvrez l'app, collez votre **ARL** Deezer dans la barre latérale
(voir [docs/ARL.md](docs/ARL.md) — 2 minutes, outils développeur du navigateur), puis cliquez sur *Se connecter*.

## 📦 Téléchargements

Les binaires pré-compilés (AppImage/deb, dmg, msi/nsis + archives portables) sont publiés sur la page
[GitHub Releases](https://github.com/Breezer-App/breezer/releases), avec mise à jour automatique intégrée.

## 🧱 Structure du dépôt

```
├── software/   # Le client Rust (workspace Cargo : breezer + breezer-plugin-api)
├── site/       # Le site web : monolithe Bun (Next.js + Elysia.js)
├── themes/     # Manifests de thèmes officiels + communautaires (marketplace)
├── docs/       # ARL, thèmes, plugins, captures d'écran…
└── .github/workflows/  # CI, versionning (release-plz), releases (cargo-packager)
```

## 🛠 Développement

```bash
cargo check -p breezer            # vérification rapide
cargo run -- --selftest           # autotest de la sortie audio (sans GUI)
cargo run -p breezer              # lancer l'application
cargo test -p breezer
```

Voir [docs/THEMES.md](docs/THEMES.md) (créer & soumettre un thème),
[docs/PLUGINS.md](docs/PLUGINS.md) (API des plugins), [docs/SCREENSHOTS.md](docs/SCREENSHOTS.md).

## 🌍 Site web

`site/` est un serveur Bun monolithe : **Next.js** (App Router, FR/EN via `next-intl`) + **Elysia.js** (`/api/*` — proxy du marketplace de thèmes, métadonnées des releases, stats de téléchargement). Voir `site/README.md`.

## ⚖️ Licence & aspects légaux

**AGPL-3.0-or-later**. Breezer est un client indépendant, tiers. Il **n'est pas** affilié à Deezer SAS.
La lecture dépend des droits de votre compte Deezer ; l'application ne contourne jamais le DRM
(le contenu protégé par Widevine n'est pas supporté, par conception). Utilisez-le de manière éthique,
conformément aux Conditions Générales de Deezer et à votre législation locale.

---

**Fait avec ❤️ et Rust.** Contributions, thèmes, traductions et traductions de ce README sont les bienvenues !