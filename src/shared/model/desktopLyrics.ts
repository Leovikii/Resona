export interface DesktopLyricsWindowSnapshot {
  supported: boolean;
  visible: boolean;
  locked: boolean;
}

export interface DesktopLyricsWindowFailure {
  code: string;
  message: string;
}

export const initialDesktopLyricsWindowSnapshot: DesktopLyricsWindowSnapshot = {
  supported: true,
  visible: false,
  locked: false,
};
