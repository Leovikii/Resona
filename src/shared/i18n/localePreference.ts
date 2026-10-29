export type LocalePreference = "system" | "zh-CN" | "en";

export function resolveLocalePreference(preference: LocalePreference, languages: readonly string[]) {
  if (preference !== "system") return preference;
  for (const language of languages) {
    const primary = language.trim().toLowerCase().split(/[_.@-]/)[0];
    if (primary === "zh") return "zh-CN";
    if (primary === "en" || primary === "c" || primary === "posix") return "en";
  }
  return "en";
}
