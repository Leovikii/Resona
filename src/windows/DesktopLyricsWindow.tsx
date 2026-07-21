import { useEffect, useMemo } from "react";
import type { CSSProperties, PointerEvent, ReactNode } from "react";
import { useTranslation } from "react-i18next";

import { useDesktopLyricsNowPlaying } from "../features/lyrics/useDesktopLyricsNowPlaying";
import { useDesktopLyricsWindow } from "../features/lyrics/useDesktopLyricsWindow";
import { usePreferences } from "../app/preferences";
import { invokeTauri, isTauriRuntime } from "../shared/bridge/tauri";
import { fileNameFromPath } from "../shared/utils/format";
import { Lock, Pause, Play, Settings, SkipBack, SkipForward, Unlock, X } from "lucide-react";

export default function DesktopLyricsWindow() {
  const { t } = useTranslation();
  const { desktopLyrics: preferences } = usePreferences();
  const { error, initialized, lyrics, playback, runPlayback } = useDesktopLyricsNowPlaying();
  const desktopLyrics = useDesktopLyricsWindow();

  useEffect(() => {
    document.documentElement.dataset.window = "desktop-lyrics";
    document.title = t("desktopLyrics.title");
    return () => {
      delete document.documentElement.dataset.window;
    };
  }, [t]);

  useEffect(() => {
    if (!initialized || !isTauriRuntime()) return;
    let cancelled = false;
    requestAnimationFrame(() => requestAnimationFrame(() => {
      if (cancelled) return;
      void invokeTauri("desktop_lyrics_window_ready").catch((readyError) => {
        console.error("Unable to show the prepared desktop lyrics window", readyError);
      });
    }));
    return () => {
      cancelled = true;
    };
  }, [initialized]);

  const display = useMemo(() => {
    if (!initialized) return { current: "", next: "" };
    if (error) return { current: t("desktopLyrics.unavailable"), next: "" };
    if (lyrics.audioPath !== playback.path || (lyrics.status === "idle" && playback.path)) {
      return { current: t("desktopLyrics.loading"), next: "" };
    }
    if (lyrics.status === "failed") {
      return { current: t("lyrics.failed"), next: "" };
    }
    if (lyrics.status === "empty") {
      return { current: t("lyrics.empty"), next: "" };
    }
    const lines = lyrics.document?.lines ?? [];
    if (lyrics.status !== "ready" || lines.length === 0) {
      return { current: t("playback.noLyrics"), next: fileNameFromPath(playback.path) };
    }
    const index = lyrics.activeLineIndex;
    if (index === null) {
      return { current: "\u00a0", next: "\u00a0" };
    }
    return {
      current: lines[index]?.text || "\u00a0",
      next: lines[index + 1]?.text ?? "",
    };
  }, [error, lyrics, playback.path, t]);

  const startDragging = (event: PointerEvent<HTMLElement>) => {
    if (event.button !== 0 || !isTauriRuntime()) return;
    event.preventDefault();
    void invokeTauri("start_desktop_lyrics_drag");
  };

  const currentId = playback.currentItemId ?? playback.queue[0]?.id;
  const hasTrack = currentId !== undefined && playback.path !== null;
  const canSkip = playback.status === "playing" || playback.status === "paused";
  const playLabel = playback.status === "playing"
    ? t("playback.pause")
    : playback.status === "paused"
      ? t("playback.resume")
      : t("playback.play");
  const runSettings = () => {
    if (isTauriRuntime()) void invokeTauri("open_main_settings");
  };
  const stopControlDrag = (event: PointerEvent<HTMLDivElement>) => event.stopPropagation();
  const style = {
    "--desktop-lyrics-font-size": `${preferences.fontSize}px`,
    "--desktop-lyrics-color": preferences.color,
    "--desktop-lyrics-text-opacity": preferences.textOpacity / 100,
    "--desktop-lyrics-background-opacity": preferences.backgroundOpacity / 100,
  } as CSSProperties;

  return (
    <main
      aria-label={t("desktopLyrics.region")}
      className="desktop-lyrics-window"
      data-locked={desktopLyrics.snapshot.locked || undefined}
      data-ready={initialized || undefined}
      data-testid="desktop-lyrics-window"
      onPointerDown={startDragging}
      style={style}
    >
      <div aria-hidden="true" className="desktop-lyrics-background" />
      <div className="desktop-lyrics-toolbar" onPointerDown={stopControlDrag}>
        <div className="desktop-lyrics-toolbar-title" title={fileNameFromPath(playback.path)}>
          {fileNameFromPath(playback.path) || t("playback.noTrack")}
        </div>
        <div className="desktop-lyrics-toolbar-group desktop-lyrics-toolbar-center">
          <DesktopLyricsButton
            ariaLabel={t("playback.previous")}
            disabled={!canSkip || desktopLyrics.busy}
            onClick={() => void runPlayback("previous_playback")}
            title={t("playback.previous")}
          >
            <SkipBack size={15} />
          </DesktopLyricsButton>
          <DesktopLyricsButton
            ariaLabel={playLabel}
            disabled={!hasTrack || desktopLyrics.busy}
            onClick={() => {
              if (playback.status === "playing") void runPlayback("pause_playback");
              else if (playback.status === "paused") void runPlayback("resume_playback");
              else if (currentId !== undefined) void runPlayback("play_queue_item", { id: currentId });
            }}
            title={playLabel}
          >
            {playback.status === "playing" ? <Pause size={15} /> : <Play size={15} />}
          </DesktopLyricsButton>
          <DesktopLyricsButton
            ariaLabel={t("playback.next")}
            disabled={!canSkip || desktopLyrics.busy}
            onClick={() => void runPlayback("next_playback")}
            title={t("playback.next")}
          >
            <SkipForward size={15} />
          </DesktopLyricsButton>
        </div>
        <div className="desktop-lyrics-toolbar-group desktop-lyrics-toolbar-right">
          <DesktopLyricsButton ariaLabel={t("desktopLyrics.settings")} onClick={runSettings} title={t("desktopLyrics.settings")}>
            <Settings size={15} />
          </DesktopLyricsButton>
          <DesktopLyricsButton
            ariaLabel={desktopLyrics.snapshot.locked ? t("desktopLyrics.unlock") : t("desktopLyrics.lock")}
            disabled={desktopLyrics.busy || !desktopLyrics.snapshot.visible}
            onClick={() => void desktopLyrics.run(
              desktopLyrics.snapshot.locked
                ? "unlock_desktop_lyrics_window"
                : "lock_desktop_lyrics_window",
            )}
            title={desktopLyrics.snapshot.locked ? t("desktopLyrics.unlock") : t("desktopLyrics.lock")}
          >
            {desktopLyrics.snapshot.locked ? <Unlock size={15} /> : <Lock size={15} />}
          </DesktopLyricsButton>
          <DesktopLyricsButton
            ariaLabel={t("desktopLyrics.close")}
            onClick={() => void desktopLyrics.run("hide_desktop_lyrics_window")}
            title={t("desktopLyrics.close")}
          >
            <X size={15} />
          </DesktopLyricsButton>
        </div>
      </div>
      <div className="desktop-lyrics-copy" key={`${lyrics.revision}-${lyrics.activeLineIndex}`}>
        <div aria-live="polite" className="desktop-lyrics-current">{display.current}</div>
        <div className="desktop-lyrics-next">{display.next || "\u00a0"}</div>
      </div>
    </main>
  );
}

function DesktopLyricsButton({
  ariaLabel,
  children,
  disabled,
  onClick,
  title,
}: {
  ariaLabel: string;
  children: ReactNode;
  disabled?: boolean;
  onClick: () => void;
  title: string;
}) {
  return (
    <button
      aria-label={ariaLabel}
      className="desktop-lyrics-button"
      disabled={disabled}
      onClick={onClick}
      title={title}
      type="button"
    >
      {children}
    </button>
  );
}
