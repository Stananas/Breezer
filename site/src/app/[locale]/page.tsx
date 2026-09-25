import { getTranslations } from "next-intl/server";
import { setRequestLocale } from "next-intl/server";
import Link from "next/link";

export default async function HomePage({
  params,
}: {
  params: Promise<{ locale: string }>;
}) {
  const { locale } = await params;
  setRequestLocale(locale);
  const t = await getTranslations({ locale });

  return (
    <main style={{ background: "#0f1012", color: "#f2f3f5", minHeight: "70vh" }}>
      <section style={{ textAlign: "center", padding: "80px 24px 48px" }}>
        <h1 style={{ fontSize: 44, margin: "0 0 12px" }}>🎧 {t("hero.title")}</h1>
        <p style={{ fontSize: 18, color: "#9aa0a8", maxWidth: 640, margin: "0 auto 28px" }}>
          {t("hero.subtitle")}
        </p>
        <div style={{ display: "flex", gap: 12, justifyContent: "center", flexWrap: "wrap" }}>
          <Link
            href={`/${locale}/download`}
            style={{
              background: "#a238ff",
              color: "#fff",
              padding: "12px 22px",
              borderRadius: 8,
              textDecoration: "none",
              fontWeight: 600,
            }}
          >
            {t("hero.cta")}
          </Link>
          <a
            href="https://github.com/Breezer-App/breezer"
            target="_blank"
            rel="noreferrer"
            style={{
              border: "1px solid #1f2127",
              color: "#f2f3f5",
              padding: "12px 22px",
              borderRadius: 8,
              textDecoration: "none",
            }}
          >
            {t("hero.ctaAlt")}
          </a>
        </div>
        <p style={{ color: "#9aa0a8", fontSize: 13, marginTop: 16 }}>
          {t("hero.platforms")}
        </p>
      </section>

      <section
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))",
          gap: 16,
          padding: "0 24px 64px",
          maxWidth: 980,
          margin: "0 auto",
        }}
      >
        {[
          t("features.lightweight"),
          t("features.themes"),
          t("features.privacy"),
          t("features.open"),
        ].map((f, i) => (
          <div
            key={i}
            style={{
              background: "#17181c",
              borderRadius: 8,
              padding: 20,
              fontSize: 15,
              color: "#f2f3f5",
            }}
          >
            {f}
          </div>
        ))}
      </section>
    </main>
  );
}