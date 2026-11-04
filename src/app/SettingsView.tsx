import { useCallback, useEffect, useState } from "react";
import type { ReactNode } from "react";
import { ActionIcon, Button, ColorInput, Group, NumberInput, ScrollArea, SegmentedControl, Select, Slider, Switch, Text, Title, useMantineColorScheme } from "@mantine/core";
import { Download, RefreshCw } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { useApplicationLifetime } from "../features/window/useApplicationLifetime";
import type { useApplicationUpdate } from "../features/update/useApplicationUpdate";
import type { useDesktopLyricsWindow } from "../features/lyrics/useDesktopLyricsWindow";
import type { PlaybackSnapshot } from "../shared/model/playback";
import { invokeTauri, isTauriRuntime } from "../shared/bridge/tauri";
import { Hint } from "../shared/ui/Hint";
import { localizeFailure } from "../shared/i18n/playbackFailure";
import { accentColors, type AccentColor, usePreferences } from "./preferences";

export function SettingsView({ applicationLifetime, applicationUpdate, busy, desktopLyrics, layoutBusy, layoutMode, onOpenUpdate, onRefresh, onSetLayoutMode, onSelectOutput, output }: {
  applicationLifetime: ReturnType<typeof useApplicationLifetime>;
  applicationUpdate: ReturnType<typeof useApplicationUpdate>;
  busy: boolean;
  desktopLyrics: ReturnType<typeof useDesktopLyricsWindow>;
  layoutBusy: boolean;
  layoutMode: "wide" | "compact";
  onOpenUpdate: () => void;
  onRefresh: () => void;
  onSetLayoutMode: (mode: "wide" | "compact") => Promise<boolean>;
  onSelectOutput: (id: string | null) => void;
  output: PlaybackSnapshot["output"];
}) {
  const { t } = useTranslation();
  const { colorScheme, setColorScheme } = useMantineColorScheme({ keepTransitions: true });
  const {
    accentColor,
    desktopLyrics: lyricsPreferences,
    locale,
    setAccentColor,
    setDesktopLyrics,
    setLocale,
  } = usePreferences();
  const [backgroundOpacity, setBackgroundOpacity] = useState(lyricsPreferences.backgroundOpacity);
  const [textOpacity, setTextOpacity] = useState(lyricsPreferences.textOpacity);
  const [fontDraft, setFontDraft] = useState(String(lyricsPreferences.fontSize));
  const [aboutError, setAboutError] = useState<string | null>(null);
  useEffect(() => setBackgroundOpacity(lyricsPreferences.backgroundOpacity), [lyricsPreferences.backgroundOpacity]);
  useEffect(() => setTextOpacity(lyricsPreferences.textOpacity), [lyricsPreferences.textOpacity]);
  useEffect(() => setFontDraft(String(lyricsPreferences.fontSize)), [lyricsPreferences.fontSize]);
  const openProjectPage = useCallback(async (page: "repository" | "releases") => {
    try {
      if (isTauriRuntime()) {
        await invokeTauri("open_project_page", { page });
      } else {
        window.open(
          page === "repository"
            ? "https://github.com/Leovikii/Resona"
            : "https://github.com/Leovikii/Resona/releases/latest",
          "_blank",
          "noopener,noreferrer",
        );
      }
      setAboutError(null);
    } catch (error) {
      setAboutError(String(error));
    }
  }, []);
  const commitFontSize = useCallback(async (raw: string) => {
    const parsed = Number(raw);
    if (!Number.isFinite(parsed)) {
      setFontDraft(String(lyricsPreferences.fontSize));
      return;
    }
    const value = Math.round(Math.min(64, Math.max(16, parsed)));
    setFontDraft(String(value));
    if (desktopLyrics.snapshot.visible) {
      const fitted = await desktopLyrics.run("fit_desktop_lyrics_window", { fontSize: value });
      if (!fitted) return;
    }
    setDesktopLyrics({ fontSize: value });
  }, [desktopLyrics, lyricsPreferences.fontSize, setDesktopLyrics]);
  const updateButtonLabel = !applicationUpdate.snapshot.supported
    ? t("settings.versionManual", { version: applicationUpdate.snapshot.currentVersion })
    : applicationUpdate.available
    ? `v${applicationUpdate.snapshot.currentVersion} -> v${applicationUpdate.available.version}`
    : applicationUpdate.status === "checking"
      ? t("settings.versionChecking", { version: applicationUpdate.snapshot.currentVersion })
      : applicationUpdate.checked
        ? t("settings.versionCurrent", { version: applicationUpdate.snapshot.currentVersion })
        : t("settings.versionCheck", { version: applicationUpdate.snapshot.currentVersion });
  const handleUpdateAction = useCallback(async () => {
    if (applicationUpdate.available) {
      onOpenUpdate();
      return;
    }
    const release = await applicationUpdate.check();
    if (release) onOpenUpdate();
  }, [applicationUpdate.available, applicationUpdate.check, onOpenUpdate]);
  return (
    <ScrollArea className="settings-scroll" scrollHideDelay={700} type="scroll">
    <div className="page-content settings-page">
      <Title className="page-heading" order={2}>{t("settings.title")}</Title>
      <section className="settings-section settings-about-section">
        <Text className="settings-section-title" fw={650}>{t("settings.about")}</Text>
        <SettingRow label={t("settings.version")}>
          <Button
            leftSection={applicationUpdate.available ? <Download size={15} /> : <RefreshCw size={15} />}
            loading={applicationUpdate.status === "checking"}
            onClick={() => void handleUpdateAction()}
            size="xs"
            variant={applicationUpdate.available ? "filled" : "light"}
          >
            {updateButtonLabel}
          </Button>
        </SettingRow>
        {applicationUpdate.snapshot.currentBuild && <SettingRow label={t("settings.build")}>
          <Text size="xs" style={{ overflowWrap: "anywhere" }}>{applicationUpdate.snapshot.currentBuild}</Text>
        </SettingRow>}
        {applicationUpdate.snapshot.supported && <SettingRow label={t("settings.prereleaseUpdates")}>
          <Switch
            aria-label={t("settings.prereleaseUpdates")}
            checked={applicationUpdate.snapshot.receivePrereleaseUpdates}
            disabled={applicationUpdate.status !== "idle"}
            onChange={(event) => {
              void applicationUpdate.setReceivePrereleaseUpdates(event.currentTarget.checked);
            }}
            size="sm"
          />
        </SettingRow>}
        <div className="about-footer-row">
          <Text c="dimmed" size="xs">{t("settings.copyright")}</Text>
          <Hint label={t("settings.repository")}>
            <ActionIcon
              aria-label={t("settings.repository")}
              onClick={() => void openProjectPage("repository")}
              size="sm"
              variant="subtle"
            >
              <GitHubMark />
            </ActionIcon>
          </Hint>
        </div>
        {aboutError && <Text c="red" role="alert" size="xs">{aboutError}</Text>}
        {applicationUpdate.error && (
          <Text c="red" role="alert" size="xs">
            {t(`settings.updateErrors.${applicationUpdate.error.code}`, {
              defaultValue: applicationUpdate.error.message,
            })}
          </Text>
        )}
      </section>
      <section className="settings-section">
        <Text className="settings-section-title" fw={650}>{t("settings.appearance")}</Text>
        <SettingRow label={t("settings.colorScheme")}>
          <SegmentedControl
            data={[
              { label: t("settings.system"), value: "auto" },
              { label: t("settings.light"), value: "light" },
              { label: t("settings.dark"), value: "dark" },
            ]}
            onChange={(value) => setColorScheme(value as "auto" | "light" | "dark")}
            size="xs"
            value={colorScheme}
          />
        </SettingRow>
        <SettingRow label={t("settings.accent")}>
          <Group gap="xs">
            {accentColors.map((color) => (
              <button
                aria-label={t(`settings.accentColors.${color}`)}
                className="color-swatch"
                data-active={accentColor === color || undefined}
                key={color}
                onClick={() => setAccentColor(color as AccentColor)}
                style={{ backgroundColor: `var(--mantine-color-${color}-6)` }}
                type="button"
              />
            ))}
          </Group>
        </SettingRow>
        <SettingRow label={t("settings.language")}>
          <SegmentedControl
            data={[
              { label: t("settings.languageSystem"), value: "system" },
              { label: t("settings.chinese"), value: "zh-CN" },
              { label: t("settings.english"), value: "en" },
            ]}
            onChange={(value) => setLocale(value as "system" | "zh-CN" | "en")}
            size="xs"
            value={locale}
          />
        </SettingRow>
        <SettingRow label={t("settings.windowLayout")}>
          <SegmentedControl
            data={[
              { label: t("settings.wideLayout"), value: "wide" },
              { label: t("settings.compactLayout"), value: "compact" },
            ]}
            disabled={layoutBusy}
            onChange={(value) => void onSetLayoutMode(value as "wide" | "compact")}
            size="xs"
            value={layoutMode}
          />
        </SettingRow>
        <SettingRow label={t("settings.closeBehavior")}>
          <Select
            allowDeselect={false}
            data={[
              { label: t("settings.closeBehaviors.ask"), value: "ask" },
              { label: t("settings.closeBehaviors.hide_to_tray"), value: "hide_to_tray" },
              { label: t("settings.closeBehaviors.exit"), value: "exit" },
            ]}
            onChange={(value) => {
              if (value) {
                void applicationLifetime.setCloseBehavior(
                  value as "ask" | "hide_to_tray" | "exit",
                );
              }
            }}
            size="xs"
            value={applicationLifetime.snapshot.closeBehavior}
          />
        </SettingRow>
        {applicationLifetime.error && (
          <Text c="red" role="alert" size="xs">{applicationLifetime.error}</Text>
        )}
      </section>

      <section className="settings-section">
        <Text className="settings-section-title" fw={650}>{t("desktopLyrics.title")}</Text>
        {!desktopLyrics.snapshot.supported && <Text c="dimmed" size="sm">{t("desktopLyrics.unavailable")}</Text>}
        {desktopLyrics.snapshot.supported && <>
        <SettingRow className="font-size-setting" label={t("desktopLyrics.fontSize")}>
          <NumberInput
            allowDecimal={false}
            allowNegative={false}
            aria-label={t("desktopLyrics.fontSize")}
            clampBehavior="blur"
            className="desktop-lyrics-font-input"
            hideControls
            inputMode="numeric"
            max={64}
            min={16}
            onBlur={() => void commitFontSize(fontDraft)}
            onChange={(value) => setFontDraft(String(value))}
            onKeyDown={(event) => {
              if (event.key === "Enter") {
                event.preventDefault();
                void commitFontSize(fontDraft);
              } else if (event.key === "Escape") {
                setFontDraft(String(lyricsPreferences.fontSize));
                event.currentTarget.blur();
              }
            }}
            onWheel={(event) => event.currentTarget.blur()}
            suffix=" px"
            value={fontDraft}
          />
        </SettingRow>
        <SettingRow className="color-setting" label={t("desktopLyrics.color")}>
          <ColorInput
            aria-label={t("desktopLyrics.color")}
            format="hex"
            onChange={(value) => setDesktopLyrics({ color: value })}
            value={lyricsPreferences.color}
            w={130}
          />
        </SettingRow>
        <SettingRow label={t("desktopLyrics.textOpacity")}>
          <Slider
            aria-label={t("desktopLyrics.textOpacity")}
            className="lyrics-opacity-setting"
            label={(value) => `${value}%`}
            max={100}
            min={10}
            onChange={(value) => {
              setTextOpacity(value);
              setDesktopLyrics({ textOpacity: value });
            }}
            step={10}
            value={textOpacity}
          />
        </SettingRow>
        <SettingRow label={t("desktopLyrics.backgroundOpacity")}>
          <Slider
            aria-label={t("desktopLyrics.backgroundOpacity")}
            className="lyrics-opacity-setting"
            label={(value) => `${value}%`}
            max={90}
            min={0}
            onChange={(value) => {
              setBackgroundOpacity(value);
              setDesktopLyrics({ backgroundOpacity: value });
            }}
            step={10}
            value={backgroundOpacity}
          />
        </SettingRow>
        </>}
        {desktopLyrics.error && (
          <Text c="red" size="xs">
            {t(`desktopLyrics.errors.${desktopLyrics.error.code}`, {
              defaultValue: t("desktopLyrics.unavailable"),
            })}
          </Text>
        )}
      </section>

      <section className="settings-section">
        <Text className="settings-section-title" fw={650}>{t("settings.output")}</Text>
        <SettingRow label={t("settings.device")}>
          <Group className="output-setting" gap="xs" wrap="nowrap">
            <Select
              allowDeselect={false}
              data={[
                { value: "__system_default__", label: t("settings.followDefault") },
                ...output.devices.map((device) => ({
                  value: device.id,
                  label: `${device.name}${device.isDefault ? ` (${t("settings.system")})` : ""}`,
                })),
              ]}
              disabled={busy}
              onChange={(value) => onSelectOutput(value === "__system_default__" ? null : value)}
              value={output.followSystemDefault ? "__system_default__" : output.selectedDeviceId}
            />
            <ActionIcon aria-label={t("common.refresh")} disabled={busy} onClick={onRefresh} variant="default">
              <RefreshCw size={16} />
            </ActionIcon>
          </Group>
        </SettingRow>
        <Text c={output.status === "unavailable" ? "red" : "dimmed"} size="xs">
          {output.status === "unavailable"
            ? localizeFailure(output.error, t)
            : output.activeDeviceName
              ? t("settings.activeOutput", { name: output.activeDeviceName })
              : t("settings.outputClosed")}
        </Text>
      </section>
      <section className="settings-section">
        <Text c="dimmed" size="sm">{t("playback.shortcuts")}</Text>
      </section>
    </div>
    </ScrollArea>
  );
}

function GitHubMark() {
  return (
    <svg aria-hidden="true" fill="currentColor" height="17" viewBox="0 0 16 16" width="17">
      <path d="M8 0C3.58 0 0 3.64 0 8.13c0 3.59 2.29 6.64 5.47 7.71.4.08.55-.18.55-.39 0-.2-.01-.84-.01-1.53-2.01.38-2.53-.5-2.69-.96-.09-.24-.48-.97-.82-1.17-.28-.16-.68-.56-.01-.57.63-.01 1.08.59 1.23.83.72 1.22 1.87.88 2.33.67.07-.53.28-.88.51-1.08-1.78-.21-3.64-.91-3.64-4.02 0-.89.31-1.63.82-2.2-.08-.2-.36-1.04.08-2.17 0 0 .67-.22 2.2.84A7.4 7.4 0 0 1 8 3.82a7.4 7.4 0 0 1 2 .27c1.53-1.06 2.2-.84 2.2-.84.44 1.13.16 1.97.08 2.17.51.57.82 1.3.82 2.2 0 3.12-1.87 3.81-3.65 4.02.29.25.54.74.54 1.5 0 1.08-.01 1.95-.01 2.22 0 .22.15.47.55.39A8.12 8.12 0 0 0 16 8.13C16 3.64 12.42 0 8 0Z" />
    </svg>
  );
}

function SettingRow({ children, className, label }: { children: ReactNode; className?: string; label: string }) {
  return (
    <div className={`setting-row${className ? ` ${className}` : ""}`}>
      <Text c="dimmed" size="sm">{label}</Text>
      <div className="setting-control">{children}</div>
    </div>
  );
}

