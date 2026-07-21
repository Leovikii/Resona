export type MainWindowLayoutMode = "wide" | "compact";

export interface MainWindowSnapshot {
  layoutMode: MainWindowLayoutMode;
}

export interface MainWindowFailure {
  code: string;
  message: string;
}
