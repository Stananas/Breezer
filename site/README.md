# Breezer website (site/)

The official Breezer website — a **Bun monolith**: **Next.js** (App Router,
static export, FR/EN via `next-intl`) served by the same process as the
**Elysia.js** API (`/api/*`). One process, one port, no micro-services.

## Stack

- **Bun** — runtime (`bun install`, `bun run …`)
- **Next.js 15** — `output: "export"` (fully static build, no Node server needed)
- **next-intl** — i18n (`src/i18n/messages/{en,fr}.json`)
- **Elysia.js** — monolith API (`src/server/api.ts`: `/api/health`, `/api/releases`,
  `/api/releases/latest`, `/api/themes`, `/api/stats/downloads`)

## Development

```bash
bun install
bun run dev          # next dev  (web only)
bun run dev:api      # bun --watch src/server/index.ts  (API + static fallback)
```

## Production (the monolith)

```bash
bun run build        # next build → out/
bun start            # one Bun process: serves out/ + /api/* on :3000
```

## Layout

```
src/
├── app/[locale]/      # pages (home, download, themes, docs) — FR/EN
├── i18n/              # next-intl config + messages
└── server/            # Elysia monolith (api.ts + index.ts)
```

## API endpoints

| Route | Description |
|---|---|
| `GET /api/health` | liveness |
| `GET /api/releases` | GitHub release feed (cached 5 min) |
| `GET /api/releases/latest` | latest release or `null` |
| `GET /api/themes` | official theme gallery (marketplace proxy) |
| `GET /api/stats/downloads` | download stats (v0.1 counts assets) |

The repo keeps `themes/` as the source of truth for the theme marketplace;
`/api/themes` will proxy it straight from GitHub in v0.2.