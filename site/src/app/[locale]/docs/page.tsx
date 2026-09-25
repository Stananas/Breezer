import { getTranslations } from "next-intl/server";
import { setRequestLocale } from "next-intl/server";

export default async function DocsPage({
  params,
}: {
  params: Promise<{ locale: string }>;
}) {
  const { locale } = await params;
  setRequestLocale(locale);
  const t = await getTranslations({ locale });

  const docs = [
    {
      label: t("docs.arl"),
      href: "https://github.com/Stananas/Breezer/blob/main/docs/ARL.md",
    },
    {
      label: t("docs.plugins"),
      href: "https://github.com/Stananas/Breezer/blob/main/docs/PLUGINS.md",
    },
    {
      label: t("docs.themesGuide"),
      href: "https://github.com/Stananas/Breezer/blob/main/docs/THEMES.md",
    },
  ];

  return (
    <main
      style={{
        background: "#0f1012",
        color: "#f2f3f5",
        minHeight: "70vh",
        padding: "48px 24px",
        maxWidth: 720,
        margin: "0 auto",
      }}
    >
      <h1 style={{ fontSize: 30 }}>📖 {t("docs.title")}</h1>
      <p style={{ color: "#9aa0a8" }}>{t("docs.intro")}</p>
      <ul style={{ listStyle: "none", padding: 0 }}>
        {docs.map((d) => (
          <li key={d.href} style={{ margin: "10px 0" }}>
            <a
              href={d.href}
              target="_blank"
              rel="noreferrer"
              style={{ color: "#a238ff", textDecoration: "none" }}
            >
              {d.label} →
            </a>
          </li>
        ))}
      </ul>
    </main>
  );
}