import { useState } from "react";
import { Button, Group, Modal, Stack, Text } from "@mantine/core";
import { useTranslation } from "react-i18next";
import { invokeTauri, isTauriRuntime } from "../shared/bridge/tauri";

export default function PackageManagerUpdateDialog({ opened, onClose, prerelease }: {
  opened: boolean;
  onClose: () => void;
  prerelease: boolean;
}) {
  const { t } = useTranslation();
  const [copied, setCopied] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const copy = async (command: string) => {
    try {
      await navigator.clipboard.writeText(command);
      setCopied(command);
      setError(null);
    } catch (cause) { setError(String(cause)); }
  };
  const open = async (url: string) => {
    try {
      if (isTauriRuntime()) await invokeTauri("open_external_url", { url });
      else window.open(url, "_blank", "noopener,noreferrer");
      setError(null);
    } catch (cause) { setError(String(cause)); }
  };
  return <Modal centered closeButtonProps={{ "aria-label": t("common.close") }} opened={opened} onClose={onClose} title={t("settings.packageUpdateHelp")} size="md">
    <Stack gap="md">
      <Text size="sm">{t("settings.packageUpdateBody")}</Text>
      <Group gap="xs">
        {["yay -Syu", "paru -Syu"].map((command) => <Button key={command} size="xs" variant="light" onClick={() => void copy(command)}>
          {t(copied === command ? "settings.copiedCommand" : "settings.copyCommand", { command })}
        </Button>)}
      </Group>
      <Text size="sm">{t("settings.packageUpdateManual")}</Text>
      <Text c="dimmed" size="xs">{t("settings.packageUpdateData")}</Text>
      <Group gap="xs">
        <Button size="xs" variant="default" onClick={() => void open(`https://aur.archlinux.org/packages/${prerelease ? "resona-preview-bin" : "resona-bin"}`)}>{t("settings.aurPage")}</Button>
        <Button size="xs" variant="default" onClick={() => void open("https://github.com/Leovikii/Resona/releases")}>{t("settings.releasePage")}</Button>
      </Group>
      {error && <Text c="red" size="xs" role="alert">{error}</Text>}
    </Stack>
  </Modal>;
}
