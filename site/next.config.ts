import type { NextConfig } from "next";
import createNextIntlPlugin from "next-intl/plugin";

// next-intl resolves ./src/i18n/request.ts by default.
const withNextIntl = createNextIntlPlugin();

// Static export: the Elysia/Bun server (src/server/index.ts) serves both
// the generated `out/` directory and the `/api/*` routes — one process,
// one port (the monolith).
const nextConfig: NextConfig = {
  output: "export",
  trailingSlash: true,
  images: { unoptimized: true },
};

export default withNextIntl(nextConfig);