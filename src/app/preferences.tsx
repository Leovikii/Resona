import { createContext, useContext, useEffect, useMemo, useState } from "react";
import type { PropsWithChildren } from "react";
import type { MantineColor } from "@mantine/core";
import { createTheme, localStorageColorSchemeManager, MantineProvider } from "@mantine/core";
import "@mantine/core/styles.css";

import i18n, { resolveLocale } from "../shared/i18n";
import type { LocalePreference } from "../shared/i18n";
import type { CompressionPreset } from "../shared/model/compression";

export const accentColors = ["cyan", "blue", "teal", "green", "orange", "pink"] as const;
export type AccentColor = (typeof accentColors)[number];

interface PreferencesContextValue {
  accentColor: AccentColor;
  setAccentColor: (value: AccentColor) => void;
  locale: LocalePreference;
  setLocale: (value: LocalePreference) => void;
  desktopLyrics: DesktopLyricsPreferences;
  setDesktopLyrics: (value: Partial<DesktopLyricsPreferences>) => void;
  compression: CompressionPreferences;
  setCompression: (value: Partial<CompressionPreferences>) => void;
}

export interface DesktopLyricsPreferences {
  enabled: boolean;
  fontSize: number;
  color: string;
  textOpacity: number;
  backgroundOpacity: number;
}

export interface CompressionPreferences {
  preset: CompressionPreset;
  deleteSource: boolean;
}

export const defaultDesktopLyricsPreferences: DesktopLyricsPreferences = {
  enabled: false,
  fontSize: 28,
  color: "#ffffff",
  textOpacity: 100,
  backgroundOpacity: 0,
};

const defaultCompressionPreferences: CompressionPreferences = {
  preset: "balanced",
  deleteSource: true,
};

const PreferencesContext = createContext<PreferencesContextValue | null>(null);
const colorSchemeManager = localStorageColorSchemeManager({ key: "resona-color-scheme" });

export function AppProvider({ children }: PropsWithChildren) {
  const [accentColor, setAccentColorState] = useState<AccentColor>(() =>
    readPreference("resona-accent", accentColors, "cyan"),
  );
  const [locale, setLocaleState] = useState<LocalePreference>(() =>
    readPreference("resona-locale", ["system", "zh-CN", "en"] as const, "system"),
  );
  const [desktopLyrics, setDesktopLyricsState] = useState<DesktopLyricsPreferences>(() =>
    readDesktopLyricsPreferences(),
  );
  const [compression, setCompressionState] = useState<CompressionPreferences>(() =>
    readCompressionPreferences(),
  );

  const theme = useMemo(
    () =>
      createTheme({
        primaryColor: accentColor as MantineColor,
        defaultRadius: "sm",
        fontFamily: "Segoe UI, system-ui, sans-serif",
        headings: { fontFamily: "Segoe UI, system-ui, sans-serif" },
      }),
    [accentColor],
  );

  useEffect(() => {
    const applyLocale = () => {
      const resolved = resolveLocale(locale);
      void i18n.changeLanguage(resolved);
      document.documentElement.lang = resolved;
    };
    applyLocale();
    if (locale !== "system") return;
    window.addEventListener("languagechange", applyLocale);
    return () => window.removeEventListener("languagechange", applyLocale);
  }, [locale]);

  useEffect(() => {
    const onStorage = (event: StorageEvent) => {
      if (event.key?.startsWith("resona-desktop-lyrics-")) {
        setDesktopLyricsState(readDesktopLyricsPreferences());
      }
      if (event.key?.startsWith("resona-compression-")) {
        setCompressionState(readCompressionPreferences());
      }
    };
    window.addEventListener("storage", onStorage);
    return () => window.removeEventListener("storage", onStorage);
  }, []);

  const value = useMemo<PreferencesContextValue>(
    () => ({
      accentColor,
      setAccentColor: (next) => {
        writePreference("resona-accent", next);
        setAccentColorState(next);
      },
      locale,
      setLocale: (next) => {
        writePreference("resona-locale", next);
        setLocaleState(next);
      },
      desktopLyrics,
      setDesktopLyrics: (next) => {
        const value = normalizeDesktopLyricsPreferences({ ...desktopLyrics, ...next });
        writePreference("resona-desktop-lyrics-font-size", String(value.fontSize));
        writePreference("resona-desktop-lyrics-enabled", String(value.enabled));
        writePreference("resona-desktop-lyrics-color", value.color);
        writePreference("resona-desktop-lyrics-text-opacity", String(value.textOpacity));
        writePreference("resona-desktop-lyrics-background-opacity", String(value.backgroundOpacity));
        setDesktopLyricsState(value);
      },
      compression,
      setCompression: (next) => {
        const value = { ...compression, ...next };
        writePreference("resona-compression-preset", value.preset);
        writePreference("resona-compression-delete-source", String(value.deleteSource));
        setCompressionState(value);
      },
    }),
    [accentColor, compression, desktopLyrics, locale],
  );

  return (
    <MantineProvider
      colorSchemeManager={colorSchemeManager}
      defaultColorScheme="auto"
      theme={theme}
    >
      <PreferencesContext.Provider value={value}>{children}</PreferencesContext.Provider>
    </MantineProvider>
  );
}

export function usePreferences() {
  const value = useContext(PreferencesContext);
  if (!value) throw new Error("usePreferences must be used inside AppProvider");
  return value;
}

function readPreference<T extends string>(key: string, allowed: readonly T[], fallback: T): T {
  try {
    const value = localStorage.getItem(key);
    return value && allowed.includes(value as T) ? (value as T) : fallback;
  } catch (error) {
    console.warn(`Unable to read UI preference ${key}`, error);
    return fallback;
  }
}

function writePreference(key: string, value: string) {
  try {
    localStorage.setItem(key, value);
  } catch (error) {
    console.warn(`Unable to persist UI preference ${key}`, error);
  }
}

function readDesktopLyricsPreferences(): DesktopLyricsPreferences {
  return normalizeDesktopLyricsPreferences({
    enabled: readBooleanPreference("resona-desktop-lyrics-enabled", defaultDesktopLyricsPreferences.enabled),
    fontSize: readNumberPreference(
      "resona-desktop-lyrics-font-size",
      defaultDesktopLyricsPreferences.fontSize,
    ),
    color: readColorPreference(
      "resona-desktop-lyrics-color",
      defaultDesktopLyricsPreferences.color,
    ),
    textOpacity: readNumberPreference(
      "resona-desktop-lyrics-text-opacity",
      defaultDesktopLyricsPreferences.textOpacity,
    ),
    backgroundOpacity: readNumberPreference(
      "resona-desktop-lyrics-background-opacity",
      defaultDesktopLyricsPreferences.backgroundOpacity,
    ),
  });
}

function readCompressionPreferences(): CompressionPreferences {
  return {
    preset: readPreference(
      "resona-compression-preset",
      ["fast", "balanced", "smallest"] as const,
      defaultCompressionPreferences.preset,
    ),
    deleteSource: readBooleanPreference(
      "resona-compression-delete-source",
      defaultCompressionPreferences.deleteSource,
    ),
  };
}

function normalizeDesktopLyricsPreferences(value: DesktopLyricsPreferences): DesktopLyricsPreferences {
  return {
    enabled: Boolean(value.enabled),
    fontSize: Math.round(Math.min(64, Math.max(16, value.fontSize))),
    color: /^#[0-9a-f]{6}$/i.test(value.color) ? value.color : defaultDesktopLyricsPreferences.color,
    textOpacity: Math.round(Math.min(100, Math.max(10, value.textOpacity))),
    backgroundOpacity: Math.round(Math.min(100, Math.max(0, value.backgroundOpacity))),
  };
}

function readBooleanPreference(key: string, fallback: boolean) {
  try {
    const value = localStorage.getItem(key);
    return value === null ? fallback : value === "true";
  } catch (error) {
    console.warn(`Unable to read UI preference ${key}`, error);
    return fallback;
  }
}

function readNumberPreference(key: string, fallback: number) {
  try {
    const value = Number(localStorage.getItem(key));
    return Number.isFinite(value) ? value : fallback;
  } catch (error) {
    console.warn(`Unable to read UI preference ${key}`, error);
    return fallback;
  }
}

function readColorPreference(key: string, fallback: string) {
  try {
    return localStorage.getItem(key) ?? fallback;
  } catch (error) {
    console.warn(`Unable to read UI preference ${key}`, error);
    return fallback;
  }
}
