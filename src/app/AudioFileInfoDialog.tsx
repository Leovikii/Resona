import { useEffect, useMemo, useState } from "react";
import { Button, Group, Loader, Modal, Stack, Text } from "@mantine/core";
import { FolderOpen } from "lucide-react";
import { useTranslation } from "react-i18next";

import { useAudioFileInfo } from "../features/metadata/useAudioFileInfo";
import { revealAudioFile } from "../shared/bridge/fileExplorer";
import { formatBytes, formatDuration } from "../shared/utils/format";

export function AudioFileInfoDialog({ onClose, path }: {
  onClose: () => void;
  path: string | null;
}) {
  const { t } = useTranslation();
  const { details, error, loading } = useAudioFileInfo(path);
  const [revealError, setRevealError] = useState<string | null>(null);
  useEffect(() => setRevealError(null), [path]);

  const rows = useMemo(() => details ? [
    [t("metadata.fileName"), details.fileName],
    [t("metadata.trackTitle"), details.title],
    [t("metadata.artist"), details.artist],
    [t("metadata.album"), details.album],
    [t("metadata.genre"), details.genre],
    [t("metadata.track"), numberedValue(details.trackNumber, details.trackTotal)],
    [t("metadata.disc"), numberedValue(details.discNumber, details.discTotal)],
    [t("metadata.date"), details.date],
    [t("metadata.codec"), details.codec],
    [t("metadata.bitrate"), details.audioBitrate ? `${details.audioBitrate} kbps` : null],
    [t("metadata.sampleRate"), sampleRate(details.sampleRate)],
    [t("metadata.bitDepth"), details.bitDepth ? `${details.bitDepth} bit` : null],
    [t("metadata.channelCount"), details.channels ? t("metadata.channels", { count: details.channels }) : null],
    [t("metadata.duration"), details.durationMs !== null ? formatDuration(details.durationMs) : null],
    [t("metadata.fileSize"), details.fileSize !== null ? formatBytes(details.fileSize) : null],
    [t("metadata.path"), details.path],
  ].filter((row): row is [string, string] => Boolean(row[1])) : [], [details, t]);

  return (
    <Modal centered onClose={onClose} opened={path !== null} size="lg" title={t("metadata.fileInfo")}>
      {loading ? (
        <div className="audio-info-loading"><Loader size="sm" /></div>
      ) : details ? (
        <Stack gap="md">
          <div className="audio-info-grid">
            {rows.map(([label, value]) => (
              <div className="audio-info-row" key={label}>
                <Text c="dimmed" size="sm">{label}</Text>
                <Text className="audio-info-value" size="sm" title={value}>{value}</Text>
              </div>
            ))}
          </div>
          {(error || revealError || details.metadataWarning) && (
            <Text c="red" role="alert" size="sm">{revealError || error || details.metadataWarning}</Text>
          )}
          <Group justify="flex-end">
            <Button onClick={onClose} variant="default">{t("common.close")}</Button>
            <Button
              leftSection={<FolderOpen size={16} />}
              onClick={() => {
                setRevealError(null);
                void revealAudioFile(details.path).catch((cause) => setRevealError(String(cause)));
              }}
            >{t("metadata.showInFolder")}</Button>
          </Group>
        </Stack>
      ) : (
        <Text c="red" role="alert" size="sm">{error}</Text>
      )}
    </Modal>
  );
}

function numberedValue(value: number | null, total: number | null) {
  if (value === null) return null;
  return total === null ? String(value) : `${value} / ${total}`;
}

function sampleRate(value: number | null) {
  if (value === null) return null;
  const kilohertz = value / 1000;
  return `${Number.isInteger(kilohertz) ? kilohertz : kilohertz.toFixed(1)} kHz`;
}
