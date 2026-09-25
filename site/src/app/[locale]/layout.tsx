import type { Metadata } from "next";
import type { ReactNode } from "react";
import { NextIntlClientProvider } from "next-intl";
import { getTranslations } from "next-intl/server";
import { setRequestLocale } from "next-intl/server";
import Link from "next/link";

export const dynamic = "force-static";
export function generateStaticParams() {
  return [{ locale: "en" }, { locale: "fr" }];
}

export async function generateMetadata({
  params,
}: {
  params: Promise<{ locale: string }>;
}): Promise<Metadata> {
  const { locale } = await params;
  return {
    title: "Breezer — native Deezer client",
    description:
      locale === "fr"
        ? "Client Deezer natif, ultra-léger et open-source."
        : "Ultra-lightweight, open-source, native Deezer client.",
  };
}

export default async function LocaleLayout({
  children,
  params,
}: {
  children: ReactNode;
  params: Promise<{ locale: string }>;
}) {
  const { locale } = await params;
  setRequestLocale(locale);
  const t = await getTranslations({ locale });

  const links = [
    { href: "", label: t("nav.home") },
    { href: "download", label: t("nav.download") },
    { href: "themes", label: t("nav.themes") },
    { href: "docs", label: t("nav.docs") },
  ];

  return (
    <html lang={locale}>
      <body style={{ margin: 0, fontFamily: "system-ui, sans-serif" }}>
        <NextIntlClientProvider>
          <nav
            style={{
              display: "flex",
              gap: 16,
              padding: "14px 24px",
              alignItems: "center",
              background: "#0f1012",
              color: "#f2f3f5",
            }}
          >
            <strong style={{ color: "#a238ff" }}>Breezer</strong>
            {links.map((l) => (
              <Link
                key={l.href}
                href={`/${locale}/${l.href}`}
                style={{ color: "#f2f3f5", textDecoration: "none", fontSize: 14 }}
              >
                {l.label}
              </Link>
            ))}
            <span style={{ flex: 1 }} />
            <Link
              href={locale === "en" ? "/fr" : "/en"}
              style={{ color: "#9aa0a8", fontSize: 13, textDecoration: "none" }}
            >
              {locale === "en" ? "FR" : "EN"}
            </Link>
          </nav>
          {children}
          <footer
            style={{
              padding: "20px 24px",
              textAlign: "center",
              color: "#9aa0a8",
              fontSize: 13,
              borderTop: "1px solid #1f2127",
            }}
          >
            {t("footer")}
          </footer>
        </NextIntlClientProvider>
      </body>
    </html>
  );
}