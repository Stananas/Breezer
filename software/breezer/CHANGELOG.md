# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/Stananas/Breezer/releases/tag/breezer-v0.1.0) - 2026-09-26

### Added

- *(player)* 'Récemment écoutés' toujours à jour
- *(playlists)* page playlist façon Deezer (en-tête + liste de pistes)
- *(ui)* zoom global de l'app — Ctrl + / Ctrl - / Ctrl 0
- *(update)* auto-update réelle — l'app télécharge et se remplace elle-même
- *(home)* page Accueil enrichie type Deezer
- *(ui)* icônes officielles Deezer + layout aligné + shuffle/répéter
- *(ui)* contenu page Accueil + slider durée centré
- *(player)* nouveau layout — cover centrée, transport au-dessus du slider durée, volume droit + tooltip %
- *(ui)* icônes Lucide (PNG blanc teinté) + hover partout + player ancré L/R
- *(ui)* playlists dans la sidebar (sous les onglets, scrollable)
- *(player)* ne plus lancer la musique au démarrage
- *(persist+resume)* ARL auto-login au boot, centrage Y des textes, reprise du dernier son
- *(playlists)* liste + détail des playlists (protocole du client dzr)
- *(streaming)* lecture réelle en suivant le protocole du client dzr
- *(ui)* page Réglages + player fixe redessiné + recherche visible + grille affichée
- Breezer v0.1 skeleton — Rust client (Slint + tokio + rodio/symphonia), Bun monolith site (Next.js + Elysia), themes, CI/release workflows, i18n FR/EN

### Fixed

- *(update)* updater cible les releases breezer-v* + pas de publish crates.io
- *(home)* scroll vertical Accueil fonctionnel
- *(player)* icône Pause officielle Deezer + centrage vertical volume
- *(ui)* plus de 'mur invisible' à droite — contenu pleine largeur
- *(player)* lecture réelle après boot + progression + titre/artiste + slider ↓ + tooltip
- *(ui)* container scroll des playlists réparé
- *(layout)* ancrage ABSOLU pour nav, playlists et player (fini le centrage)
- *(layout)* tous les panneaux en pleine largeur — ancrage gauche/droite OK
- *(ui)* jaquettes des playlists affichées + items alignés à gauche et plus grands
- *(ui)* la sidebar remplit toute la hauteur (le container playlists s'affiche)
- *(ui)* player ancré en dur en bas de fenêtre + contrôles centrés
- *(ui)* slider volume collé à gauche + pseudo connecté
- *(auth)* connexion ARL alignée sur le protocole réel du gateway Deezer
- *(ux)* onboarding 2 étapes + champ ARL évident (où coller ?)
- *(connexion)* connexion obligatoire + bouton Connect jamais verrouillé
- i18n bundlée (Slint @tr), onboarding splash au premier lancement, connexion ARL robuste

### Other

- *(nav)* navigation instantanee + design des reglages refait
- *(release)* formats par défaut par OS + NO_STRIP
- *(release)* rendre cargo-packager fonctionnel
- *(ci)* cargo fmt normalize + GITHUB_TOKEN pour release-plz
- *(repo)* passer les références à github.com/Stananas/Breezer
- *(repo)* préparation release — README global + i18n complet + releases propres
