# Contributing to Breezer

Thanks for helping out! 🎧 Everything below is short on purpose.

## Build & run

```bash
# Linux system deps
sudo apt-get install -y libasound2-dev libxkbcommon-dev cmake clang

cd software
cargo check -p breezer     # quick check
cargo run -p breezer       # run the app
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --all -- --check
```

## Commit conventions

Releases are automated with [release-plz](https://release-plz.dev), which reads
**Conventional Commits**. Format:

```
<type>(<scope>): <summary>

<optional body>
```

- `feat(ui)`, `fix(player)`, `feat(api)`, `docs`, `chore`, ... (types)
- `feat*` bumps the minor version, `fix*` the patch.

`feat:` and `fix:` commits should include a short summary of *what* changed
(a mutable note on *why*).

## i18n (FR/EN)

Two channels — keep both in sync:

1. **Slint UI strings** use `@tr("English default")`. Add the French translation
   in `software/breezer/translations/fr/LC_MESSAGES/breezer.po`.
2. **Rust-side messages** (status bars, errors) use `i18n.t("key")`.
   Add the key to `software/breezer/src/i18n/en.json` **and** `fr.json`.

Run `cargo build -p breezer` to verify the `.po` parses (bundled at build time).

## Themes

See [docs/THEMES.md](docs/THEMES.md) — a theme is a small JSON manifest in `themes/`.

## Documentation & screenshots

Main README points to `docs/*.md`. If you submit screenshots,
see [docs/SCREENSHOTS.md](docs/SCREENSHOTS.md).

## License

By contributing you agree your work is released under **AGPL-3.0-or-later**.