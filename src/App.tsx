import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import {
  ActionIcon,
  Button,
  Group,
  Loader,
  Stack,
  Text,
  Title,
  Tooltip,
} from "@mantine/core";
import { FolderOpen, Music2, Pause, Play, Square } from "lucide-react";

type PlaybackStatus = "idle" | "playing" | "paused" | "stopped" | "failed";

interface PlaybackSnapshot {
  status: PlaybackStatus;
  path: string | null;
  error: string | null;
}

const initialSnapshot: PlaybackSnapshot = {
  status: "idle",
  path: null,
  error: null,
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
      filters: [{ name: "WAV / FLAC", extensions: ["wav", "flac"] }],
    });

    if (typeof selected === "string") {
      setSelectedPath(selected);
      setSnapshot((current) => ({ ...current, error: null }));
    }
  };

  const runCommand = async (
    command: "play_file" | "pause_playback" | "resume_playback" | "stop_playback",
    args?: Record<string, unknown>,
  ) => {
    setPending(true);
    try {
      const next = await invoke<PlaybackSnapshot>(command, args);
      setSnapshot(next);
    } catch (error) {
      setSnapshot((current) => ({
        ...current,
        status: "failed",
        error: String(error),
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

  return (
    <main className="app-shell">
      <header className="app-header">
        <Group gap="sm">
          <Music2 aria-hidden="true" size={24} strokeWidth={1.8} />
          <div>
            <Title order={1}>Resona</Title>
            <Text c="dimmed" size="xs">0.0.1</Text>
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
            {selectedPath ?? "WAV / FLAC"}
          </Text>
        </div>

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
            {snapshot.error}
          </Text>
        )}
      </section>
    </main>
  );
}

export default App;
