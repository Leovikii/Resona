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

export function mergePlaybackSnapshot(
  previous: PlaybackSnapshot,
  next: PlaybackSnapshot,
): PlaybackSnapshot {
  const queue = sameQueue(previous.queue, next.queue) ? previous.queue : next.queue;
  const output = sameOutput(previous.output, next.output) ? previous.output : next.output;
  const error = sameFailure(previous.error, next.error) ? previous.error : next.error;
  if (
    previous.status === next.status
    && previous.path === next.path
    && previous.positionMs === next.positionMs
    && previous.durationMs === next.durationMs
    && previous.volume === next.volume
    && previous.seekable === next.seekable
    && previous.currentItemId === next.currentItemId
    && previous.playbackMode === next.playbackMode
    && previous.queue === queue
    && previous.output === output
    && previous.error === error
  ) {
    return previous;
  }
  return { ...next, queue, output, error };
}

function sameQueue(left: PlaybackSnapshot["queue"], right: PlaybackSnapshot["queue"]) {
  return left.length === right.length && left.every((item, index) => {
    const candidate = right[index];
    return item.id === candidate.id
      && item.path === candidate.path
      && item.displayName === candidate.displayName
      && item.durationMs === candidate.durationMs
      && item.status === candidate.status
      && item.cue?.cuePath === candidate.cue?.cuePath
      && item.cue?.trackNumber === candidate.cue?.trackNumber
      && item.cue?.title === candidate.cue?.title
      && item.cue?.performer === candidate.cue?.performer
      && item.cue?.album === candidate.cue?.album
      && item.cue?.startMs === candidate.cue?.startMs
      && item.cue?.endMs === candidate.cue?.endMs
      && sameFailure(item.error, candidate.error);
  });
}

function sameFailure(left: PlaybackFailure | null, right: PlaybackFailure | null) {
  return left === right || (left?.code === right?.code && left?.message === right?.message);
}

function sameOutput(left: PlaybackSnapshot["output"], right: PlaybackSnapshot["output"]) {
  return left.status === right.status
    && left.followSystemDefault === right.followSystemDefault
    && left.selectedDeviceId === right.selectedDeviceId
    && left.activeDeviceId === right.activeDeviceId
    && left.activeDeviceName === right.activeDeviceName
    && left.activeSampleRate === right.activeSampleRate
    && left.activeChannelCount === right.activeChannelCount
    && left.activeSampleFormat === right.activeSampleFormat
    && sameFailure(left.error, right.error)
    && left.devices.length === right.devices.length
    && left.devices.every((device, index) => {
      const candidate = right.devices[index];
      return device.id === candidate.id
        && device.name === candidate.name
        && device.isDefault === candidate.isDefault
        && device.interfaceType === candidate.interfaceType;
    });
}
