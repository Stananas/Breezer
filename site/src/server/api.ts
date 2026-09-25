// Elysia.js API — the "server" half of the monolith.
//
// Served by the same Bun process as the static site (src/server/index.ts).
// v0.1 endpoints: release metadata (from GitHub), theme gallery, download
// stats, health. All cached in-memory with a short TTL.

import { Elysia, t } from "elysia";

const GITHUB_REPO = "Stananas/Breezer";
const GITHUB_API = "https://api.github.com/repos";
const UA = "breezer-site/0.1";

export interface ReleaseAsset {
  name: string;
  url: string;
  size: number;
}

export interface ReleaseInfo {
  tag: string;
  name: string;
  published_at: string;
  notes: string;
  assets: ReleaseAsset[];
}

let releasesCache: ReleaseInfo[] | null = null;
let releasesAt = 0;

async function fetchReleases(): Promise<ReleaseInfo[]> {
  const now = Date.now();
  if (releasesCache && now - releasesAt < 5 * 60_000) return releasesCache;

  const resp = await fetch(`${GITHUB_API}/${GITHUB_REPO}/releases?per_page=10`, {
    headers: { "User-Agent": UA },
  });
  if (!resp.ok) {
    // Repository not published yet → graceful empty list.
    return [];
  }
  const data = (await resp.json()) as any[];
  const out: ReleaseInfo[] = data.map((r) => ({
    tag: r.tag_name ?? "",
    name: r.name ?? r.tag_name ?? "",
    published_at: r.published_at ?? "",
    notes: (r.body ?? "").slice(0, 2000),
    assets: (r.assets ?? []).map((a: any) => ({
      name: a.name,
      url: a.browser_download_url,
      size: a.size ?? 0,
    })),
  }));
  releasesCache = out;
  releasesAt = now;
  return out;
}

const OFFICIAL_THEMES = ["breezer-dark", "breezer-light", "breezer-amoled"];

export const api = new Elysia({ prefix: "/api" })
  .get("/health", () => ({ ok: true }), {
    response: t.Object({ ok: t.Boolean() }),
  })
  .get("/releases", () => fetchReleases())
  .get("/releases/latest", async () => {
    const releases = await fetchReleases();
    return releases[0] ?? null;
  })
  .get("/themes", () =>
    OFFICIAL_THEMES.map((id) => ({
      id,
      version: "1.0.0",
      license: "AGPL-3.0-or-later",
      // v0.2: proxied from the `themes/` folder of the repo (raw GitHub), with
      // preview screenshots exposed here once contributed.
      preview: null as string | null,
    })),
  )
  .get("/stats/downloads", async () => {
    const releases = await fetchReleases();
    const total = releases.reduce(
      (acc, r) => acc + r.assets.reduce((a, b) => a + (b.size > 0 ? 1 : 0), 0),
      0,
    );
    // v0.2: real download counters (GitHub API doesn't expose them publicly —
    // we'll use a small self-hosted counter table instead).
    return { releases: releases.length, assetsCounted: total, downloaded: null };
  });