# Breezer Themes

Themes are simple, versioned, JSON manifests. A theme sets **colors** (palette) and can also bring a
**layout preset** and typography tokens — so a community theme can completely change the character
of the app, not just its colors.

## Anatomy of a theme

```jsonc
{
  "id": "breezer-dark",          // snake-case id (also the folder name)
  "name": "Breezer Dark",        // human readable
  "version": "1.0.0",            // semver
  "author": "Breezer team",
  "license": "AGPL-3.0-or-later",// themes are AGPL by default
  "palette": {                   // color tokens
    "bg": "#0f1012", "surface": "#17181c", "surface2": "#1f2127",
    "primary": "#a238ff", "on-primary": "#ffffff", "accent": "#2a9d8f",
    "text": "#f2f3f5", "text-secondary": "#9aa0a8", "error": "#e5484d",
    "radius": 8, "spacing": 12
  },
  "layout": {                    // optional layout preset
    "left": "sidebar", "right": "", "bottom": "player", "density": "comfortable"
  }
}
```

Schema: [`themes/manifest.schema.json`](../themes/manifest.schema.json).

## User overrides (manual customization)

Any palette token can be overridden by the user without editing the theme:
Breezer stores overrides separately (`config.json` → `theme_overrides`) and applies them on top.
A visual color editor is on the roadmap.

## Working with themes in Breezer

- **Switch theme**: sidebar → *Theme* → pick Dark / Light / AMOLED (any `.theme.json` dropped in
  `$CONFIG/breezer/themes/` is picked up too).
- **Save layout**: sidebar → *Enregistrer le layout* (persists `layout.json`).

## Contributing a community theme

1. Fork the repo.
2. Create `themes/my-theme/theme.json`, following the schema.
3. Add a 1280x720 screenshot in `themes/my-theme/preview.png`.
4. Open a PR. CI validates the JSON against the schema.

Themes accepted in the `themes/` folder will appear in the website gallery and in the in-app
marketplace once the marketplace lands (roadmap).

---

# Thèmes Breezer

Les thèmes sont des manifests JSON simples et versionnés. Un thème définit les **couleurs** (palette)
et peut aussi embarquer un **preset de layout** et des tokens typographiques — un thème communautaire
peut donc changer totalement le caractère de l'app, pas seulement ses couleurs.

## Anatomie d'un thème

Voir l'exemple ci-dessus (section EN). Schéma : [`themes/manifest.schema.json`](../themes/manifest.schema.json).

## Surcharges utilisateur (personnalisation manuelle)

N'importe quel token de palette peut être surchargé par l'utilisateur, sans modifier le thème :
Breezer stocke les surcharges à part (`config.json` → `theme_overrides`) et les applique par-dessus.
Un éditeur de couleurs visuel est au programme.

## Utiliser les thèmes dans Breezer

- **Changer de thème** : barre latérale → *Thème* → choisir Sombre / Clair / AMOLED (tout `.theme.json`
  déposé dans `$CONFIG/breezer/themes/` est aussi détecté).
- **Enregistrer le layout** : barre latérale → *Enregistrer le layout* (persiste dans `layout.json`).

## Contribuer un thème communautaire

1. Forkez le dépôt.
2. Créez `themes/mon-theme/theme.json`, conforme au schéma.
3. Ajoutez une capture 1280x720 dans `themes/mon-theme/preview.png`.
4. Ouvrez une PR. La CI valide le JSON contre le schéma.

Les thèmes acceptés dans `themes/` apparaîtront dans la galerie du site puis dans le marketplace
intégré à l'app (au programme).