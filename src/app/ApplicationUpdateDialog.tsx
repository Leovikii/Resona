import { useCallback } from "react";
import {
  Badge,
  Button,
  Group,
  Modal,
  Progress,
  ScrollArea,
  Stack,
  Text,
} from "@mantine/core";
import { Download } from "lucide-react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { useTranslation } from "react-i18next";

import { useApplicationUpdate } from "../features/update/useApplicationUpdate";
import { invokeTauri, isTauriRuntime } from "../shared/bridge/tauri";
import { formatBytes } from "../shared/utils/format";

export default function ApplicationUpdateDialog({ applicationUpdate, onClose, opened }: {
  applicationUpdate: ReturnType<typeof useApplicationUpdate>;
  onClose: () => void;
  opened: boolean;
}) {
  const { t } = useTranslation();
  const release = applicationUpdate.available;
  const active = applicationUpdate.status === "installing"
    || applicationUpdate.status === "cancelling";
  const openExternal = useCallback(async (url: string) => {
    if (!/^https?:\/\//i.test(url)) return;
    if (isTauriRuntime()) {
      await invokeTauri("open_external_url", { url });
    } else {
      window.open(url, "_blank", "noopener,noreferrer");
    }
  }, []);
  const releaseMeta = [
    release?.publishedAt ? formatReleaseDate(release.publishedAt) : null,
    release?.installerSize !== null && release?.installerSize !== undefined
      ? formatBytes(release.installerSize)
      : null,
  ].filter(Boolean).join(" · ");
  const progressValue = applicationUpdate.progress.totalBytes
    ? Math.min(
        100,
        (applicationUpdate.progress.downloadedBytes / applicationUpdate.progress.totalBytes) * 100,
      )
    : 100;

  return (
    <Modal
      centered
      closeOnClickOutside={!active}
      onClose={onClose}
      opened={opened && release !== null}
      size="lg"
      title={release ? t("settings.updateAvailable", { version: release.version }) : ""}
    >
      {release && (
        <Stack gap="md">
          <div className="application-update-heading">
            <Group gap="xs">
              <Text fw={700}>{release.title}</Text>
              {release.prerelease && (
                <Badge color="orange" size="xs" variant="light">{t("settings.prerelease")}</Badge>
              )}
            </Group>
            {releaseMeta && <Text c="dimmed" size="xs">{releaseMeta}</Text>}
          </div>
          <ScrollArea className="application-update-notes" scrollHideDelay={700} type="scroll">
            <div className="markdown-body">
              <ReactMarkdown
                components={{
                  a: ({ children, href }) => (
                    <a
                      href={href}
                      onClick={(event) => {
                        event.preventDefault();
                        if (href) void openExternal(href);
                      }}
                    >
                      {children}
                    </a>
                  ),
                }}
                remarkPlugins={[remarkGfm]}
                skipHtml
              >
                {release.notes || t("settings.noReleaseNotes")}
              </ReactMarkdown>
            </div>
          </ScrollArea>
          {!applicationUpdate.snapshot.updaterConfigured && (
            <Text c="orange" size="xs">{t("settings.updaterNotConfigured")}</Text>
          )}
          {applicationUpdate.error && (
            <Text c="red" role="alert" size="xs">
              {t(`settings.updateErrors.${applicationUpdate.error.code}`, {
                defaultValue: applicationUpdate.error.message,
              })}
            </Text>
          )}
          {active && (
            <Stack gap={6}>
              <Progress animated value={progressValue} />
              <Text c="dimmed" size="xs">
                {applicationUpdate.progress.totalBytes
                  ? t("settings.updateProgress", {
                      downloaded: formatBytes(applicationUpdate.progress.downloadedBytes),
                      total: formatBytes(applicationUpdate.progress.totalBytes),
                    })
                  : t("settings.preparingUpdate")}
              </Text>
            </Stack>
          )}
          <Group justify="space-between">
            <Button
              onClick={() => void openExternal(release.releaseUrl)}
              size="xs"
              variant="subtle"
            >
              {t("settings.releasePage")}
            </Button>
            <Group gap="xs">
              {active ? (
                <Button
                  disabled={applicationUpdate.status === "cancelling"}
                  onClick={() => void applicationUpdate.cancel()}
                  size="xs"
                  variant="default"
                >
                  {applicationUpdate.status === "cancelling"
                    ? t("common.cancelling")
                    : t("common.cancel")}
                </Button>
              ) : (
                <Button onClick={onClose} size="xs" variant="default">{t("common.later")}</Button>
              )}
              <Button
                disabled={active || !applicationUpdate.snapshot.updaterConfigured}
                leftSection={<Download size={15} />}
                onClick={() => void applicationUpdate.install()}
                size="xs"
              >
                {t("settings.downloadAndInstall")}
              </Button>
            </Group>
          </Group>
        </Stack>
      )}
    </Modal>
  );
}

function formatReleaseDate(value: string) {
  const date = new Date(value);
  return Number.isNaN(date.getTime())
    ? ""
    : new Intl.DateTimeFormat(undefined, { dateStyle: "medium" }).format(date);
}
