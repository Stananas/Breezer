// Bun monolith server: one process, one port.
//
//  - serves the statically exported site (`next build` → `out/`) with an i18n
//    root redirect (/ → /en),
//  - mounts the Elysia API under /api/* (see ./api.ts),
//  - never proxies to another service — that's the whole point.

import { api } from "./api";
import { dirname, join, normalize, extname } from "node:path";
import { fileURLToPath } from "node:url";

const PORT = Number(process.env.PORT ?? 3000);
// Path to the `next build` output (site/out), relative to this file.
const WEB_ROOT = join(dirname(fileURLToPath(import.meta.url)), "..", "..", "out");

// MIME types for the static site (a tiny subset is enough for a static export).
const MIME: Record<string, string> = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".png": "image/png",
  ".jpg": "image/jpeg",
  ".webp": "image/webp",
  ".ico": "image/x-icon",
  ".svg": "image/svg+xml",
  ".txt": "text/plain; charset=utf-8",
  ".woff2": "font/woff2",
};

Bun.serve({
  port: PORT,
  async fetch(req) {
    const url = new URL(req.url);

    // Elysia API route.
    if (url.pathname.startsWith("/api")) {
      return api.handle(req);
    }

    // i18n root redirect.
    if (url.pathname === "/") {
      return Response.redirect("/en/", 302);
    }

    // Static site.
    let relative = normalize(url.pathname).replace(/^[\\/]+/, "");
    if (relative === "") relative = "index.html";
    if (!extname(relative)) relative = relative.endsWith("/") ? relative + "index.html" : relative + "/index.html";

    const file = Bun.file(join(WEB_ROOT, relative));
    if (await file.exists()) {
      const type = MIME[extname(relative)] ?? "application/octet-stream";
      return new Response(file, { headers: { "Content-Type": type } });
    }

    return new Response("Not found — build the site first with `bun run build`.", {
      status: 404,
    });
  },
});

console.log(`⚡ Breezer monolith serving site + /api on http://localhost:${PORT}`);
console.log(`   (site root: ${WEB_ROOT})`);