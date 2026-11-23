import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { ES } from "./es";

/**
 * Translation: English is the source text. Wrap any user-facing string in
 * `t("...")`; Spanish comes from `ES`, keyed by that exact English text.
 * A string with no Spanish yet falls back to English, and
 * `bun run i18n:check` lists any that are missing.
 *
 * Placeholders: t("{n} modules", { n: 3 }).
 */
export type Lang = "en" | "es";

export const LANGS: { id: Lang; label: string; name: string }[] = [
  { id: "en", label: "EN", name: "English" },
  { id: "es", label: "ES", name: "Español" },
];

const KEY = "apexdiag.lang";

function initial(): Lang {
  try {
    const saved = localStorage.getItem(KEY);
    if (saved === "en" || saved === "es") return saved;
  } catch {
    /* storage blocked */
  }
  return typeof navigator !== "undefined" && navigator.language?.toLowerCase().startsWith("es") ? "es" : "en";
}

type Vars = Record<string, string | number>;
export type T = (text: string, vars?: Vars) => string;

const I18nContext = createContext<{ lang: Lang; setLang: (l: Lang) => void; t: T } | null>(null);

export function translate(lang: Lang, text: string, vars?: Vars): string {
  let out = lang === "es" ? (ES[text] ?? text) : text;
  if (vars) for (const [k, v] of Object.entries(vars)) out = out.replaceAll(`{${k}}`, String(v));
  return out;
}

export function I18nProvider({ children }: { children: ReactNode }) {
  const [lang, setLangState] = useState<Lang>(initial);
  const setLang = useCallback((l: Lang) => {
    setLangState(l);
    try {
      localStorage.setItem(KEY, l);
    } catch {
      /* storage blocked */
    }
  }, []);
  useEffect(() => {
    document.documentElement.lang = lang;
  }, [lang]);
  const t = useCallback<T>((text, vars) => translate(lang, text, vars), [lang]);
  const value = useMemo(() => ({ lang, setLang, t }), [lang, setLang, t]);
  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n() {
  const ctx = useContext(I18nContext);
  if (!ctx) throw new Error("useI18n outside I18nProvider");
  return ctx;
}

export function useT(): T {
  return useI18n().t;
}

/** Translates a short fixed label in JSX: <Tr>Connect</Tr>. */
export function Tr({ children }: { children: string }) {
  return <>{useT()(children)}</>;
}

/** Locale for dates and numbers. */
export function useLocale() {
  return useI18n().lang === "es" ? "es" : undefined;
}
