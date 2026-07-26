import { useCallback, useEffect, useState } from "react";
import { Notification, Transition } from "@mantine/core";
import { CircleAlert, CircleCheck, CircleX, Info } from "lucide-react";

export type TransientNoticeTone = "error" | "info" | "success" | "warning";

export interface TransientNoticeMessage {
  id: number;
  message: string;
  tone: TransientNoticeTone;
}

const toneAppearance = {
  error: { color: "red", icon: CircleX, role: "alert" },
  info: { color: "blue", icon: Info, role: "status" },
  success: { color: "green", icon: CircleCheck, role: "status" },
  warning: { color: "orange", icon: CircleAlert, role: "status" },
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
    const timer = window.setTimeout(dismiss, autoClose);
    return () => window.clearTimeout(timer);
  }, [autoClose, dismiss, notice?.id]);

  const appearance = toneAppearance[notice?.tone ?? "info"];
  const Icon = appearance.icon;
  return (
    <Transition duration={160} mounted={Boolean(notice) && visible} transition="slide-left">
      {(transitionStyles) => (
        <Notification
          className="app-notice"
          closeButtonProps={{ "aria-label": closeLabel }}
          color={appearance.color}
          icon={<Icon size={16} />}
          onClose={dismiss}
          role={appearance.role}
          style={transitionStyles}
          withBorder={false}
          withCloseButton
        >
          {notice?.message}
        </Notification>
      )}
    </Transition>
  );
}
