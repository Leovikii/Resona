import i18n from "i18next";
import { initReactI18next } from "react-i18next";

import { resources } from "./resources";

export type LocalePreference = "system" | "zh-CN" | "en";

export function resolveLocale(preference: LocalePreference) {
  if (preference !== "system") return preference;
  return navigator.language.toLowerCase().startsWith("zh") ? "zh-CN" : "en";
}

void i18n.use(initReactI18next).init({
  resources,
  lng: resolveLocale("system"),
  fallbackLng: "en",
  interpolation: { escapeValue: false },
  returnNull: false,
});

export default i18n;
