export interface DesktopLyricsWindowSnapshot {
  supported: boolean;
  layerShell?: boolean;
  visible: boolean;
  locked: boolean;
}

export interface DesktopLyricsWindowFailure {
  code: string;
  message: string;
}

export const initialDesktopLyricsWindowSnapshot: DesktopLyricsWindowSnapshot = {
  supported: false,
  visible: false,
  locked: false,
};
