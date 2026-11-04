import type { TFunction } from "i18next";
import type { PlaybackFailure } from "../model/playback";

export function localizeFailure(
  failure: PlaybackFailure | null,
  t: TFunction,
) {
  if (!failure) return "";
  const localized = t(`errors.${failure.code}`, { defaultValue: failure.message });
  return failure.code === "task_failed" && failure.message && failure.message !== localized
    ? `${localized}: ${failure.message}`
    : localized;
}

