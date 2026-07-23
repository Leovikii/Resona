export type CloseBehavior = "ask" | "hide_to_tray" | "exit";
export type CloseDecision = Exclude<CloseBehavior, "ask">;

export interface ApplicationLifetimeSnapshot {
  closeBehavior: CloseBehavior;
}

export const defaultApplicationLifetimeSnapshot: ApplicationLifetimeSnapshot = {
  closeBehavior: "ask",
};
