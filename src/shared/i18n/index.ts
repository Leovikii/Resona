import i18n from "i18next";
import { initReactI18next } from "react-i18next";

import { resources } from "./resources";
import { readSystemLanguages } from "../bridge/systemLanguages";
import { resolveLocalePreference, type LocalePreference } from "./localePreference";

export type { LocalePreference } from "./localePreference";
let systemLanguages: readonly string[] | null = null;

export async function loadSystemLanguages() {
  try {
    systemLanguages = await readSystemLanguages();
  } catch (error) {
    console.error("Unable to read native system languages; using WebView languages", error);
    systemLanguages = null;
  }
}

export function resolveLocale(preference: LocalePreference) {
  const browser = navigator.languages.length > 0 ? navigator.languages : [navigator.language];
  return resolveLocalePreference(preference, systemLanguages ?? browser);
}

void i18n.use(initReactI18next).init({
  resources,
  lng: resolveLocale("system"),
  fallbackLng: "en",
  interpolation: { escapeValue: false },
  returnNull: false,
});

export default i18n;
