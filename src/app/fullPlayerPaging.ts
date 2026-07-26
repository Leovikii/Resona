import type { LyricsStatus } from "../shared/model/lyrics";

export type FullPlayerPage = "artwork" | "lyrics" | "details";

export interface FullPlayerPagingState {
  compact: boolean;
  defaultResolved: boolean;
  lyricsResolved: boolean;
  manuallySelected: boolean;
  page: FullPlayerPage;
  trackKey: string | null;
}

interface FullPlayerPagingInput {
  artworkResolved: boolean;
  compact: boolean;
  hasArtwork: boolean;
  lineCount: number;
  lyricsStatus: LyricsStatus;
  trackKey: string | null;
}

export function fullPlayerPages(compact: boolean): FullPlayerPage[] {
  return compact ? ["artwork", "lyrics", "details"] : ["lyrics", "details"];
}

export function defaultFullPlayerPage(
  compact: boolean,
  lyricsStatus: LyricsStatus,
  lineCount: number,
  hasArtwork: boolean,
): FullPlayerPage {
  if (lyricsStatus === "ready" && lineCount > 0) return "lyrics";
  if (isLyricsResolved(lyricsStatus)) return compact && hasArtwork ? "artwork" : "details";
  return "lyrics";
}

export function createFullPlayerPagingState(
  input: FullPlayerPagingInput,
): FullPlayerPagingState {
  return {
    compact: input.compact,
    defaultResolved: isDefaultResolved(input),
    lyricsResolved: isLyricsResolved(input.lyricsStatus),
    manuallySelected: false,
    page: defaultFullPlayerPage(
      input.compact,
      input.lyricsStatus,
      input.lineCount,
      input.hasArtwork,
    ),
    trackKey: input.trackKey,
  };
}

export function reconcileFullPlayerPaging(
  current: FullPlayerPagingState,
  input: FullPlayerPagingInput,
): FullPlayerPagingState {
  if (current.trackKey !== input.trackKey) {
    const next = createFullPlayerPagingState(input);
    const currentPageAvailable = fullPlayerPages(input.compact).includes(current.page);
    return {
      ...next,
      page: next.defaultResolved || !currentPageAvailable ? next.page : current.page,
    };
  }

  const defaultResolved = isDefaultResolved(input);
  const lyricsResolved = isLyricsResolved(input.lyricsStatus);
  const pages = fullPlayerPages(input.compact);
  const layoutChanged = current.compact !== input.compact;
  const pageUnavailable = !pages.includes(current.page);
  const applyResolvedDefault = !current.defaultResolved && defaultResolved && !current.manuallySelected;

  return {
    ...current,
    compact: input.compact,
    defaultResolved,
    lyricsResolved: current.lyricsResolved || lyricsResolved,
    page: pageUnavailable || applyResolvedDefault
      ? defaultFullPlayerPage(
        input.compact,
        input.lyricsStatus,
        input.lineCount,
        input.hasArtwork,
      )
      : current.page,
    manuallySelected: layoutChanged && pageUnavailable ? false : current.manuallySelected,
  };
}

export function selectFullPlayerPage(
  current: FullPlayerPagingState,
  page: FullPlayerPage,
): FullPlayerPagingState {
  if (!fullPlayerPages(current.compact).includes(page)) return current;
  return { ...current, manuallySelected: true, page };
}

export function adjacentFullPlayerPage(
  pages: FullPlayerPage[],
  page: FullPlayerPage,
  direction: -1 | 1,
): FullPlayerPage | null {
  const nextIndex = pages.indexOf(page) + direction;
  return nextIndex >= 0 && nextIndex < pages.length ? pages[nextIndex] : null;
}

function isLyricsResolved(status: LyricsStatus) {
  return status === "missing" || status === "empty" || status === "ready" || status === "failed";
}

function isDefaultResolved(input: FullPlayerPagingInput) {
  if (!isLyricsResolved(input.lyricsStatus)) return false;
  if (input.lyricsStatus === "ready" && input.lineCount > 0) return true;
  return !input.compact || input.artworkResolved;
}
