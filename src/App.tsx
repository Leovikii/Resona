import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import {
  ActionIcon,
  Button,
  Group,
  Loader,
  Stack,
  Slider,
  Text,
  Title,
  Tooltip,
} from "@mantine/core";
import { FolderOpen, Music2, Pause, Play, Square } from "lucide-react";

type PlaybackStatus = "idle" | "playing" | "paused" | "stopped" | "failed";

interface PlaybackSnapshot {
  status: PlaybackStatus;
  path: string | null;
  error: PlaybackFailure | null;
  positionMs: number;
  durationMs: number | null;
  volume: number;
  seekable: boolean;
}

interface PlaybackFailure {
  code: string;
  message: string;
}

const initialSnapshot: PlaybackSnapshot = {
  status: "idle",
  path: null,
  error: null,
  positionMs: 0,
  durationMs: null,
  volume: 1,
  seekable: false,
};

const statusLabels: Record<PlaybackStatus, string> = {
  idle: "待机",
  playing: "播放中",
  paused: "已暂停",
  stopped: "已停止",
  failed: "播放失败",
};

function App() {
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [snapshot, setSnapshot] = useState<PlaybackSnapshot>(initialSnapshot);
  const [pending, setPending] = useState(false);

  const fileName = useMemo(() => {
    if (!selectedPath) return "未选择音频";
    return selectedPath.split(/[\\/]/).pop() ?? selectedPath;
  }, [selectedPath]);

  const [seeking, setSeeking] = useState(false);
  const [seekPositionMs, setSeekPositionMs] = useState(0);
  const [changingVolume, setChangingVolume] = useState(false);
  const [volumePercent, setVolumePercent] = useState(100);

  useEffect(() => {
    if (!seeking) setSeekPositionMs(snapshot.positionMs);
    if (!changingVolume) setVolumePercent(Math.round(snapshot.volume * 100));
  }, [changingVolume, seeking, snapshot.positionMs, snapshot.volume]);

  const refreshSnapshot = useCallback(async () => {
    try {
      const next = await invoke<PlaybackSnapshot>("get_playback_state");
      setSnapshot(next);
    } catch {
      // The browser preview has no Tauri runtime; desktop commands remain available in-app.
    }
  }, []);

  useEffect(() => {
    void refreshSnapshot();
    const timer = window.setInterval(() => void refreshSnapshot(), 750);
    return () => window.clearInterval(timer);
  }, [refreshSnapshot]);

  const chooseFile = async () => {
    const selected = await open({
      multiple: false,
      directory: false,
      filters: [{ name: "MP3 / WAV / FLAC", extensions: ["mp3", "wav", "flac"] }],
    });

    if (typeof selected === "string") {
      setSelectedPath(selected);
      setSnapshot((current) => ({ ...current, error: null }));
    }
  };

  const runCommand = async (
    command:
      | "play_file"
      | "pause_playback"
      | "resume_playback"
      | "stop_playback"
      | "seek_playback"
      | "set_playback_volume",
    args?: Record<string, unknown>,
  ) => {
    setPending(true);
    try {
      const next = await invoke<PlaybackSnapshot>(command, args);
      setSnapshot(next);
    } catch (error) {
      const failure = toPlaybackFailure(error);
      setSnapshot((current) => ({
        ...current,
        status: command === "play_file" ? "failed" : current.status,
        error: failure,
      }));
    } finally {
      setPending(false);
    }
  };

  const togglePause = () => {
    const command = snapshot.status === "paused" ? "resume_playback" : "pause_playback";
    void runCommand(command);
  };

  const canControlPlayback =
    snapshot.status === "playing" || snapshot.status === "paused";
  const durationMs = snapshot.durationMs ?? 0;

  return (
    <main className="app-shell">
      <header className="app-header">
        <Group gap="sm">
          <Music2 aria-hidden="true" size={24} strokeWidth={1.8} />
          <div>
            <Title order={1}>Resona</Title>
            <Text c="dimmed" size="xs">0.0.2</Text>
          </div>
        </Group>
        <Text className={`status status-${snapshot.status}`} size="sm">
          {statusLabels[snapshot.status]}
        </Text>
      </header>

      <section className="player-surface">
        <div className="file-summary">
          <Text fw={600} lineClamp={1}>{fileName}</Text>
          <Text c="dimmed" size="sm" lineClamp={1} title={selectedPath ?? undefined}>
            {selectedPath ?? "MP3 / WAV / FLAC"}
          </Text>
        </div>

        <Stack className="timeline" gap="xs">
          <Group justify="space-between" gap="xs">
            <Text c="dimmed" size="xs">{formatDuration(seeking ? seekPositionMs : snapshot.positionMs)}</Text>
            <Text c="dimmed" size="xs">{formatDuration(durationMs)}</Text>
          </Group>
          <Slider
            aria-label="播放进度"
            data-testid="seek-slider"
            disabled={!canControlPlayback || !snapshot.seekable || pending}
            label={formatDuration}
            max={Math.max(durationMs, 1)}
            min={0}
            onChange={(value) => {
              setSeeking(true);
              setSeekPositionMs(value);
            }}
            onChangeEnd={(value) => {
              setSeeking(false);
              void runCommand("seek_playback", { positionMs: Math.round(value) });
            }}
            value={Math.min(seekPositionMs, Math.max(durationMs, 1))}
          />
          <Group className="volume-control" gap="sm" wrap="nowrap">
            <Text c="dimmed" size="xs">音量</Text>
            <Slider
              aria-label="音量"
              data-testid="volume-slider"
              label={(value) => `${Math.round(value)}%`}
              max={100}
              min={0}
              onChange={(value) => {
                setChangingVolume(true);
                setVolumePercent(value);
              }}
              onChangeEnd={(value) => {
                setChangingVolume(false);
                void runCommand("set_playback_volume", { volume: value / 100 });
              }}
              value={volumePercent}
            />
            <Text c="dimmed" size="xs" w={32}>{volumePercent}%</Text>
          </Group>
        </Stack>

        <Group className="transport" gap="sm" wrap="nowrap">
          <Button
            leftSection={<FolderOpen aria-hidden="true" size={18} />}
            variant="default"
            onClick={() => void chooseFile()}
          >
            选择文件
          </Button>
          <Tooltip label="播放">
            <ActionIcon
              aria-label="播放"
              disabled={!selectedPath || pending}
              onClick={() => void runCommand("play_file", { path: selectedPath })}
              size="lg"
              variant="filled"
            >
              <Play aria-hidden="true" fill="currentColor" size={18} />
            </ActionIcon>
          </Tooltip>
          <Tooltip label={snapshot.status === "paused" ? "继续" : "暂停"}>
            <ActionIcon
              aria-label={snapshot.status === "paused" ? "继续" : "暂停"}
              disabled={!canControlPlayback || pending}
              onClick={togglePause}
              size="lg"
              variant="default"
            >
              {snapshot.status === "paused" ? (
                <Play aria-hidden="true" fill="currentColor" size={17} />
              ) : (
                <Pause aria-hidden="true" fill="currentColor" size={17} />
              )}
            </ActionIcon>
          </Tooltip>
          <Tooltip label="停止">
            <ActionIcon
              aria-label="停止"
              disabled={!canControlPlayback || pending}
              onClick={() => void runCommand("stop_playback")}
              size="lg"
              variant="default"
            >
              <Square aria-hidden="true" fill="currentColor" size={15} />
            </ActionIcon>
          </Tooltip>
          {pending && <Loader aria-label="处理中" size="sm" />}
        </Group>

        {snapshot.error && (
          <Text c="red.4" role="alert" size="sm">
            {snapshot.error.message}
          </Text>
        )}
      </section>
    </main>
  );
}

function formatDuration(milliseconds: number) {
  const totalSeconds = Math.max(0, Math.floor(milliseconds / 1000));
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;
  if (hours > 0) {
    return `${hours}:${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`;
  }
  return `${minutes}:${String(seconds).padStart(2, "0")}`;
}

function toPlaybackFailure(error: unknown): PlaybackFailure {
  if (typeof error === "object" && error !== null) {
    const candidate = error as { code?: unknown; message?: unknown };
    if (typeof candidate.code === "string" && typeof candidate.message === "string") {
      return { code: candidate.code, message: candidate.message };
    }
  }
  return { code: "task_failed", message: String(error) };
}

export default App;
