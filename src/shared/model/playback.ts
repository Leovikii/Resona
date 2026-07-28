export type PlaybackStatus = "idle" | "playing" | "paused" | "stopped" | "failed";
export type PlaybackMode = "sequential" | "repeat_one" | "repeat_all" | "shuffle";
export type QueueItemStatus = "pending" | "playing" | "paused" | "played" | "failed";
export type OutputStatus = "closed" | "ready" | "unavailable";

export interface PlaybackFailure {
  code: string;
  message: string;
}

export interface QueueItem {
  id: number;
  path: string;
  displayName: string;
  durationMs: number | null;
  status: QueueItemStatus;
  error: PlaybackFailure | null;
  cue?: import("./library").CueTrackSource | null;
}

export interface OutputDevice {
  id: string;
  name: string;
  isDefault: boolean;
  interfaceType: string;
}

export interface OutputSnapshot {
  status: OutputStatus;
  devices: OutputDevice[];
  followSystemDefault: boolean;
  selectedDeviceId: string | null;
  activeDeviceId: string | null;
  activeDeviceName: string | null;
  activeSampleRate: number | null;
  activeChannelCount: number | null;
  activeSampleFormat: string | null;
  error: PlaybackFailure | null;
}

export interface PlaybackSnapshot {
  status: PlaybackStatus;
  path: string | null;
  error: PlaybackFailure | null;
  positionMs: number;
  durationMs: number | null;
  volume: number;
  seekable: boolean;
  queue: QueueItem[];
  currentItemId: number | null;
  playbackMode: PlaybackMode;
  output: OutputSnapshot;
}

export const emptySnapshot: PlaybackSnapshot = {
  status: "idle",
  path: null,
  error: null,
  positionMs: 0,
  durationMs: null,
  volume: 1,
  seekable: false,
  queue: [],
  currentItemId: null,
  playbackMode: "sequential",
  output: {
    status: "closed",
    devices: [],
    followSystemDefault: true,
    selectedDeviceId: null,
    activeDeviceId: null,
    activeDeviceName: null,
    activeSampleRate: null,
    activeChannelCount: null,
    activeSampleFormat: null,
    error: null,
  },
};

export function previewSnapshot(): PlaybackSnapshot {
  return {
    ...emptySnapshot,
    status: "playing",
    path: "C:\\Music\\Resona Demo\\Midnight Signal.flac",
    positionMs: 82_000,
    durationMs: 247_000,
    seekable: true,
    volume: 0.72,
    currentItemId: 2,
    queue: [
      queueItem(1, "First Light.wav", "played", 194_000),
      queueItem(2, "Midnight Signal.flac", "playing", 247_000),
      queueItem(3, "Blue Transit.mp3", "pending", 219_000),
      queueItem(4, "Afterimage.flac", "pending", 281_000),
    ],
    output: {
      status: "ready",
      devices: [
        {
          id: "preview-default",
          name: "Speakers (Realtek Audio)",
          isDefault: true,
          interfaceType: "built_in",
        },
        {
          id: "preview-bluetooth",
          name: "Bluetooth Headphones",
          isDefault: false,
          interfaceType: "bluetooth",
        },
      ],
      followSystemDefault: true,
      selectedDeviceId: null,
      activeDeviceId: "preview-default",
      activeDeviceName: "Speakers (Realtek Audio)",
      activeSampleRate: 48_000,
      activeChannelCount: 2,
      activeSampleFormat: "f32",
      error: null,
    },
  };
}

function queueItem(
  id: number,
  displayName: string,
  status: QueueItemStatus,
  durationMs: number,
): QueueItem {
  return {
    id,
    displayName,
    path: `C:\\Music\\Resona Demo\\${displayName}`,
    durationMs,
    status,
    error: null,
  };
}
