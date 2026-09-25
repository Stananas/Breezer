import { getRequestConfig } from "next-intl/server";

export default getRequestConfig(async ({ requestLocale }) => {
  const locale = ((await requestLocale) ?? "en").slice(0, 2);
  const accepted = ["en", "fr"];
  const lang = accepted.includes(locale) ? locale : "en";
  return {
    locale: lang,
    messages: (await import(`./messages/${lang}.json`)).default,
  };
});