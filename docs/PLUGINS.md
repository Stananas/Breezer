# Breezer Plugin API

Breezer exposes a **stable, public plugin API** from day one. Plugins provide **logic** and hook into
existing **shell extension points** (toolbar, menus, actions) — they do not ship arbitrary UI in v0.1
(dynamic Slint compilation at runtime is a documented roadmap option, disabled by default to keep the
binary and RAM footprint low).

## The contract

The public trait lives in the `breezer-plugin-api` crate (workspace member)
so plugins can link against it without pulling in the whole app:

```rust
pub trait Plugin: Send + Sync {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn version(&self) -> &'static str;
    fn description(&self) -> &'static str { "" }

    // Hooks
    fn on_track_change(&self, track: &TrackMeta) {}
    fn on_theme_change(&self, theme_id: &str) {}
    fn on_layout_change(&self, layout_json: &str) {}
    fn on_auth_change(&self, logged_in: bool, username: &str) {}

    // Shell extension points (v0.1)
    fn menu_items(&self) -> Vec<MenuItem> { Vec::new() }
}
```

## Extension points (v0.1)

- **Menus**: `menu_items()` adds entries (id + label) to the app menu/extensions toolbar.
- **Hooks**: track/theme/layout/auth change notifications.
- **Roadmap**: `toolbar actions`, keybindings, then optional **dynamic loading**
  (sidecar JSON-RPC over stdio, or WASM sandbox) — the trait above is the stable ABI
  these loaders will instantiate.

## ABI stability

- The `Plugin` trait must remain source- and (near-term) layout-compatible.
  Breaking changes are only allowed on major version bumps (0.x → 1.x etc.).
- Do not leak crate-private types into the trait signatures.

## Adding a builtin plugin

1. Implement `breezer_plugin_api::Plugin`.
2. Register it in the `PluginRegistry` (`software/breezer/src/plugins/mod.rs`).
3. Wire its `menu_items()` to the app menu.

Example: the built-in `system` plugin declares the *À propos* menu entry.

---

# API des plugins Breezer

Breezer expose une **API publique et stable** dès le premier jour. Les plugins fournissent de la
**logique** et s'accrochent aux **points d'extension du shell** existants (toolbar, menus, actions) —
ils ne livrent pas d'UI arbitraire en v0.1 (la compilation Slint dynamique au runtime est une option
documentée du programme, désactivée par défaut pour garder le binaire et la RAM légers).

## Le contrat

Le trait public vit dans le crate `breezer-plugin-api` (membre du workspace) pour que les plugins
puissent linker dessus sans tirer toute l'app (voir exemple ci-dessus, section EN).

## Points d'extension (v0.1)

- **Menus** : `menu_items()` ajoute des entrées (id + libellé) au menu de l'app / à la toolbar d'extensions.
- **Hooks** : notifications de changement de piste / thème / layout / auth.
- **Au programme** : `actions de toolbar`, raccourcis clavier, puis **chargement dynamique**
  (sidecar JSON-RPC via stdio, ou sandbox WASM) — le trait ci-dessus est l'ABI stable que ces
  chargeurs instancieront.

## Stabilité ABI

- Le trait `Plugin` doit rester compatible en source et (à court terme) en agencement mémoire.
  Les ruptures ne sont autorisées qu'aux versions majeures (0.x → 1.x, etc.).
- Ne faites pas fuiter de types privés du crate dans les signatures du trait.

## Ajouter un plugin intégré

1. Implémentez `breezer_plugin_api::Plugin`.
2. Enregistrez-le dans le `PluginRegistry` (`software/breezer/src/plugins/mod.rs`).
3. Branchez ses `menu_items()` au menu de l'app.

Exemple : le plugin intégré `system` déclare l'entrée de menu *À propos*.