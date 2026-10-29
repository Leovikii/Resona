import { useCallback, useEffect, useState } from "react";
import { Notification, Progress, Transition } from "@mantine/core";

export type TransientNoticeTone = "error" | "info" | "success" | "warning";

export interface TransientNoticeMessage {
  id: number;
  message: string;
  tone: TransientNoticeTone;
  progress?: number;
}

const toneAppearance = {
  error: { color: "red", role: "alert" },
  info: { color: "blue", role: "status" },
  success: { color: "green", role: "status" },
  warning: { color: "orange", role: "status" },
} as const;

export function TransientNotice({
  autoClose = 4_000,
  closeLabel,
  notice,
  onDismiss,
}: {
  autoClose?: number;
  closeLabel: string;
  notice: TransientNoticeMessage | null;
  onDismiss: () => void;
}) {
  const [visible, setVisible] = useState(Boolean(notice));

  const dismiss = useCallback(() => {
    setVisible(false);
    onDismiss();
  }, [onDismiss]);

  useEffect(() => {
    if (!notice) {
      setVisible(false);
      return;
    }
    setVisible(true);
    if (notice.tone === "error" || notice.tone === "warning") return;
    const timer = window.setTimeout(dismiss, autoClose);
    return () => window.clearTimeout(timer);
  }, [autoClose, dismiss, notice?.id]);

  const appearance = toneAppearance[notice?.tone ?? "info"];
  return (
    <Transition duration={160} mounted={Boolean(notice) && visible} transition="slide-left">
      {(transitionStyles) => (
        <Notification
          className="app-notice"
          closeButtonProps={{ "aria-label": closeLabel }}
          color={appearance.color}
          onClose={dismiss}
          role={appearance.role}
          style={transitionStyles}
          radius="md"
          withBorder
          withCloseButton
        >
          {notice?.message}
          {notice?.progress !== undefined && <Progress aria-label={notice.message} mt={6} size="xs" value={notice.progress} />}
        </Notification>
      )}
    </Transition>
  );
}
