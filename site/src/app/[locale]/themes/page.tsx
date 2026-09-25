import { getTranslations } from "next-intl/server";
import { setRequestLocale } from "next-intl/server";

export default async function ThemesPage({
  params,
}: {
  params: Promise<{ locale: string }>;
}) {
  const { locale } = await params;
  setRequestLocale(locale);
  const t = await getTranslations({ locale });

  // Gallery is fed by the Elysia API (/api/themes) proxying the repo's
  // `themes/` folder — see src/server/api.ts.
  const themes = ["breezer-dark", "breezer-light", "breezer-amoled"];

  return (
    <main
      style={{
        background: "#0f1012",
        color: "#f2f3f5",
        minHeight: "70vh",
        padding: "48px 24px",
        textAlign: "center",
      }}
    >
      <h1 style={{ fontSize: 30 }}>🎨 {t("themes.title")}</h1>
      <p style={{ color: "#9aa0a8", maxWidth: 620, margin: "0 auto 32px" }}>
        {t("themes.intro")}
      </p>
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(auto-fit, minmax(240px, 1fr))",
          gap: 16,
          maxWidth: 900,
          margin: "0 auto",
        }}
      >
        {themes.map((id) => (
          <div
            key={id}
            style={{
              background: "#17181c",
              borderRadius: 8,
              padding: "24px 20px",
              textAlign: "left",
            }}
          >
            <div style={{ fontWeight: 600 }}>{id}</div>
            <div style={{ color: "#9aa0a8", fontSize: 13, marginTop: 6 }}>
              AGPL-3.0 · v1.0.0
            </div>
          </div>
        ))}
      </div>
    </main>
  );
}