import { useTranslations } from "next-intl";

const platforms = [
  { key: "linux", icon: "🐧" },
  { key: "macos", icon: "🍎" },
  { key: "windows", icon: "🪟" },
] as const;

export default function DownloadPage() {
  const t = useTranslations("download");
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
      <h1 style={{ fontSize: 30 }}>⬇ {t("title")}</h1>
      <p style={{ color: "#9aa0a8", maxWidth: 620, margin: "0 auto 32px" }}>{t("intro")}</p>
      <a
        href="https://github.com/Breezer-App/breezer/releases/latest"
        target="_blank"
        rel="noreferrer"
        style={{
          display: "inline-block",
          background: "#a238ff",
          color: "#fff",
          padding: "12px 22px",
          borderRadius: 8,
          textDecoration: "none",
          fontWeight: 600,
          marginBottom: 40,
        }}
      >
        {t("latest")}
      </a>
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))",
          gap: 16,
          maxWidth: 900,
          margin: "0 auto",
        }}
      >
        {platforms.map((p) => (
          <div
            key={p.key}
            style={{ background: "#17181c", borderRadius: 8, padding: 24 }}
          >
            <div style={{ fontSize: 28 }}>{p.icon}</div>
            <div style={{ fontWeight: 600, margin: "8px 0" }}>{t(p.key)}</div>
            <div style={{ color: "#9aa0a8", fontSize: 13 }}>{t("platformMissing")}</div>
          </div>
        ))}
      </div>
    </main>
  );
}