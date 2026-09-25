<p align="right">
🇫🇷 <b>Français</b> &nbsp;·&nbsp; <a href="README.md">🇬🇧 <b>English</b></a>
</p>

<div align="center">

# 🎧 Breezer

**Le client Deezer natif, ultra-léger.** Rust + Slint. Pas d'Electron. Pas de bloat.

Linux · macOS · Windows

![Licence](https://img.shields.io/badge/licence-AGPL--3.0%20ou%20ultérieure-bleu)
![Rust](https://img.shields.io/badge/Rust-1.85+-orange)
![Plateformes](https://img.shields.io/badge/plateformes-Linux%20%7C%20macOS%20%7C%20Windows-lightgrey)

</div>

---

## C'est quoi Breezer ?

Un lecteur musical **natif, petit et rapide** pour **Deezer** : recherche, streaming, navigation dans vos playlists et votre historique d'écoute, avec une interface propre et personnalisable. Il reprend le même flux d'authentification et de streaming que [tui-dzr](https://github.com/dunderdoo/tui-dzr) (ARL, déchiffrement Blowfish).

## ✨ Fonctionnalités clés

- 🧊 **Léger** — binaire de quelques Mo, empreinte RAM réduite.
- 🎵 **Streaming réel** des pistes gratuites (auth ARL, déchiffrement isolé et auditable).
- 🏠 Pages façon Deezer : **Accueil** (reprendre la lecture, récemment écoutés), **Explorer** (tendances), **Coups de cœur**, **Playlists**.
- 🎨 **Système de thèmes** — sombre / clair / AMOLED intégrés, éditeur de couleurs, thèmes communautaires.
- 🧩 **Panneaux ancrables** — glisser-déposer, espaces de travail sauvegardés.
- 🌍 **i18n** — français & anglais (interface et messages).
- 🔄 **Auto-mise à jour** — vérifie, télécharge et se remplace automatiquement ; un clic pour redémarrer.
- 🔓 **AGPL-3.0** — un logiciel libre.

## 📸 Captures d'écran

| Sombre (défaut) | Clair | AMOLED |
|---|---|---|
| [![Accueil – sombre](assets/screenshots/home-dark.png)](assets/screenshots/home-dark.png) | [![Accueil – clair](assets/screenshots/home-light.png)](assets/screenshots/home-light.png) | [![Accueil – amoled](assets/screenshots/home-amoled.png)](assets/screenshots/home-amoled.png) |

[![Explorer – tendances](assets/screenshots/explorer-dark.png)](assets/screenshots/explorer-dark.png)

## 🚀 Démarrage rapide

```bash
# Prérequis : Rust 1.85+, Linux : libasound2-dev libxkbcommon-dev cmake clang
cd software
cargo run --release
```

Au premier lancement : collez votre **cookie ARL** Deezer (voir [docs/ARL.md](docs/ARL.md), ~2 min) puis connectez-vous.

## 📦 Téléchargements

Les installateurs pré-compilés (AppImage/deb, dmg, nsis) sont publiés sur la page **[Releases](https://github.com/Stananas/Breezer/releases)**,
avec mises à jour automatiques.

## 🧱 Structure du dépôt

```
software/   # Client Rust (workspace Cargo : breezer + plugin-api)
site/       # Site web (Bun : Next.js + Elysia.js)
themes/     # Manifests de thèmes officiels & communautaires
docs/       # ARL, thèmes, plugins, captures d'écran
assets/     # Captures du README & éléments de marque
```

## 🛠 Développement

```bash
cargo check -p breezer        # vérification rapide
cargo run -p breezer          # lancer l'app (alias : cargo run)
cargo run -- --selftest       # autotest audio, sans GUI
cargo test -p breezer
```

Docs : [Thèmes](docs/THEMES.md) · [Plugins](docs/PLUGINS.md) · [Captures](docs/SCREENSHOTS.md) · [Contribuer](CONTRIBUTING.md)

## ⚖️ Licence & aspects légaux

**AGPL-3.0-or-later.** Breezer est un client tiers indépendant, **non affilié** à Deezer SAS.
La lecture dépend des droits de votre compte Deezer ; le contenu protégé par DRM (Widevine) n'est jamais contourné.
Utilisez-le conformément aux Conditions Générales de Deezer et à votre législation locale.

---

**Fait avec ❤️ et Rust.** Contributions, thèmes et traductions bienvenues — la
[version anglaise de ce README](README.md) est disponible.