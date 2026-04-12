import { createContext, useCallback, useContext, useEffect, useState } from "react";
import en, { type Translations } from "./locales/en";

export type Locale = "en" | "zh-CN";

const STORAGE_KEY = "aineer-locale";

const localeModules: Record<Locale, () => Promise<{ default: Translations }>> = {
  en: () => Promise.resolve({ default: en }),
  "zh-CN": () => import("./locales/zh-CN"),
};

export function getStoredLocale(): Locale {
  try {
    const v = localStorage.getItem(STORAGE_KEY);
    if (v === "en" || v === "zh-CN") return v;
  } catch {
    // ignore
  }
  return "en";
}

function storeLocale(locale: Locale) {
  try {
    localStorage.setItem(STORAGE_KEY, locale);
  } catch {
    // ignore
  }
}

type I18nContextType = {
  locale: Locale;
  t: Translations;
  setLocale: (locale: Locale) => void;
};

export const I18nContext = createContext<I18nContextType>({
  locale: "en",
  t: en,
  setLocale: () => {},
});

export function useI18n() {
  return useContext(I18nContext);
}

export function useI18nState() {
  const [locale, setLocaleState] = useState<Locale>(getStoredLocale);
  const [translations, setTranslations] = useState<Translations>(en);

  const loadTranslations = useCallback(async (loc: Locale) => {
    const mod = await localeModules[loc]();
    setTranslations(mod.default);
  }, []);

  useEffect(() => {
    loadTranslations(locale);
  }, [locale, loadTranslations]);

  const setLocale = useCallback((loc: Locale) => {
    storeLocale(loc);
    setLocaleState(loc);
  }, []);

  return { locale, t: translations, setLocale };
}

export type { Translations };
export { en };
