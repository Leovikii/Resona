import { useEffect, useMemo, useState } from "react";
import type { CSSProperties, DragEvent, ReactNode } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  ActionIcon,
  Badge,
  Button,
  Group,
  Modal,
  Progress,
  ScrollArea,
  SegmentedControl,
  Stack,
  Switch,
  Text,
  ThemeIcon,
  Title,
  Tooltip,
  UnstyledButton,
} from "@mantine/core";
import {
  ChevronDown,
  ChevronRight,
  CircleAlert,
  FileAudio,
  Folder,
  FolderOpen,
  Trash2,
  Wrench,
  X,
} from "lucide-react";
import { useTranslation } from "react-i18next";

import { useAudioCompression } from "../features/compression/useAudioCompression";
import { selectCompressionFolders, selectWavFiles } from "../shared/bridge/audioDialog";
import { isTauriRuntime } from "../shared/bridge/tauri";
import type {
  CompressionPreset,
  CompressionScanNode,
} from "../shared/model/compression";
import { AddMediaMenu } from "../shared/ui/AddMediaMenu";

export default function AudioCompressionApp() {
  const { t } = useTranslation();
  const compression = useAudioCompression();
  const [preset, setPreset] = useState<CompressionPreset>("balanced");
  const [deleteSource, setDeleteSource] = useState(true);
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [dragActive, setDragActive] = useState(false);
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const scanBusy = compression.scan.status === "scanning" || compression.scan.status === "cancelling";
  const taskBusy = compression.snapshot.status === "running" || compression.snapshot.status === "cancelling";
  const locked = scanBusy || taskBusy;
  const readyPaths = useMemo(() => collectReadyPaths(compression.scan.roots), [compression.scan.roots]);

  useEffect(() => {
    document.documentElement.dataset.window = "audio-compression";
    document.title = `${t("app.name")} - ${t("tools.compression")}`;
    if (isTauriRuntime()) {
      void getCurrentWindow().setTitle(document.title);
      void getCurrentWindow().show().catch((error) => {
        console.error("Unable to show the audio compression window", error);
      });
    }
    return () => {
      delete document.documentElement.dataset.window;
    };
  }, [t]);

  useEffect(() => {
    setExpanded((current) => {
      const next = new Set(current);
      compression.scan.roots.forEach((root) => next.add(root.path));
      return next;
    });
  }, [compression.scan.scanId]);

  useEffect(() => {
    if (!isTauriRuntime()) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void getCurrentWebview().onDragDropEvent((event) => {
      if (event.payload.type === "leave") {
        setDragActive(false);
        return;
      }
      if (event.payload.type === "enter" || event.payload.type === "over") {
        setDragActive(!locked);
        return;
      }
      setDragActive(false);
      if (!locked && event.payload.paths.length > 0) {
        void compression.addInputs(event.payload.paths);
      }
    }).then((stop) => {
      if (disposed) stop();
      else unlisten = stop;
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [compression.addInputs, locked]);

  const addFiles = async () => {
    const paths = await selectWavFiles();
    if (paths.length > 0) void compression.addInputs(paths);
  };
  const addFolders = async () => {
    const paths = await selectCompressionFolders();
    if (paths.length > 0) void compression.addInputs(paths);
  };
  const start = () => {
    if (readyPaths.length === 0) return;
    if (deleteSource) setConfirmOpen(true);
    else void compression.start(readyPaths, preset, false, false);
  };

  const onBrowserDragOver = (event: DragEvent<HTMLElement>) => {
    event.preventDefault();
    if (!locked) setDragActive(true);
  };
  return (
    <main
      className="compression-window"
      data-drag-active={dragActive || undefined}
      onDragEnter={onBrowserDragOver}
      onDragOver={onBrowserDragOver}
      onDragLeave={(event) => {
        if (!event.currentTarget.contains(event.relatedTarget as Node | null)) setDragActive(false);
      }}
      onDrop={(event) => {
        event.preventDefault();
        setDragActive(false);
      }}
    >
      <header className="compression-window-header">
        <Group gap="sm" wrap="nowrap">
          <ThemeIcon size={36} variant="light"><Wrench size={18} /></ThemeIcon>
          <div className="compression-window-title">
            <Title order={1}>{t("tools.compression")}</Title>
            <Text c="dimmed" size="xs">{t("tools.compressionScope")}</Text>
          </div>
        </Group>
      </header>

      <section className="compression-toolbar" aria-label={t("compression.inputs")}>
        <Group className="compression-input-actions" gap="xs" wrap="nowrap">
          <AddMediaMenu
            buttonLabel={t("common.add")}
            disabled={locked}
            fileLabel={t("compression.addFiles")}
            folderLabel={t("compression.addFolder")}
            onAddFiles={() => void addFiles()}
            onAddFolders={() => void addFolders()}
          />
          {scanBusy ? (
            <Button color="red" onClick={() => void compression.cancelScan()} size="xs" variant="subtle">
              {t("compression.cancelScan")}
            </Button>
          ) : (
            <Button disabled={locked || compression.scan.inputRoots.length === 0} leftSection={<Trash2 size={15} />} onClick={() => void compression.clearInputs()} size="xs" variant="subtle">
              {t("common.clear")}
            </Button>
          )}
        </Group>
        <SegmentedControl
          aria-label={t("compression.preset")}
          className="compression-preset"
          data={[
            { label: t("compression.fast"), value: "fast" },
            { label: t("compression.balanced"), value: "balanced" },
            { label: t("compression.smallest"), value: "smallest" },
          ]}
          disabled={taskBusy}
          onChange={(value) => setPreset(value as CompressionPreset)}
          size="xs"
          value={preset}
        />
        <Group className="compression-run-actions" gap="sm" wrap="nowrap">
          <Switch
            checked={deleteSource}
            disabled={taskBusy}
            label={t("compression.deleteSource")}
            onChange={(event) => setDeleteSource(event.currentTarget.checked)}
            size="xs"
          />
          {taskBusy ? (
            <Button color="red" onClick={() => void compression.cancel()} size="xs" variant="light">
              {t("common.cancel")}
            </Button>
          ) : (
            <Button disabled={locked || readyPaths.length === 0} leftSection={<Wrench size={15} />} onClick={start} size="xs">
              {t("compression.start", { count: readyPaths.length })}
            </Button>
          )}
        </Group>
      </section>

      {(scanBusy || compression.snapshot.total > 0 || compression.scan.warnings.length > 0 || compression.error) && (
        <section className="compression-status-band" aria-live="polite">
          {scanBusy && (
            <div className="compression-scan-progress">
              <Progress
                animated
                size="xs"
                value={compression.scan.candidateFiles > 0
                  ? (compression.scan.validatedFiles / compression.scan.candidateFiles) * 100
                  : 20}
              />
              <Text c="dimmed" size="xs">
                {t("compression.scanning", {
                  candidates: compression.scan.candidateFiles,
                  scanned: compression.scan.scannedEntries,
                })}
              </Text>
            </div>
          )}
          <CompressionTaskProgress snapshot={compression.snapshot} />
          {compression.scan.warnings.length > 0 && (
            <div className="compression-warnings">
              <Group gap={6} wrap="nowrap">
                <CircleAlert size={15} />
                <Text fw={650} size="xs">{t("compression.warnings", { count: compression.scan.warnings.length })}</Text>
              </Group>
              {compression.scan.warnings.slice(0, 4).map((warning) => (
                <Text c="dimmed" key={`${warning.code}-${warning.path}`} lineClamp={1} size="xs" title={warning.message}>
                  {warning.path}
                </Text>
              ))}
            </div>
          )}
          {compression.error && <div className="compression-error" role="alert">{compression.error}</div>}
        </section>
      )}

      <div className="compression-workspace">
        <section className="compression-tree-panel" aria-label={t("compression.inputs")}>
          <div className="compression-panel-heading">
            <Text fw={650} size="sm">{t("compression.inputs")}</Text>
            <Text c="dimmed" size="xs">
              {t("compression.candidateCount", { count: compression.scan.candidateFiles })}
            </Text>
          </div>
          <ScrollArea className="compression-tree-scroll" type="auto">
            {compression.scan.roots.length === 0 ? (
              <CompressionEmpty icon={<FileAudio />} label={t("compression.emptyInputs")} />
            ) : (
              <div className="compression-tree" role="tree">
                {compression.scan.roots.map((node) => (
                  <CompressionTreeNode
                    disabled={locked}
                    expanded={expanded}
                    key={node.path}
                    level={0}
                    node={node}
                    onRemove={(path) => void compression.removeInputs([path])}
                    onToggle={(path) => setExpanded((current) => {
                      const next = new Set(current);
                      if (next.has(path)) next.delete(path);
                      else next.add(path);
                      return next;
                    })}
                  />
                ))}
              </div>
            )}
          </ScrollArea>
        </section>

      </div>

      {dragActive && (
        <div className="compression-drop-overlay" aria-hidden="true">
          <FolderOpen size={34} />
          <Text fw={650}>{t("compression.dropNow")}</Text>
        </div>
      )}

      <Modal centered onClose={() => setConfirmOpen(false)} opened={confirmOpen} title={t("compression.confirmTitle")}>
        <Stack>
          <Text size="sm">{t("compression.confirmBody", { count: readyPaths.length })}</Text>
          <Group justify="flex-end">
            <Button onClick={() => setConfirmOpen(false)} variant="default">{t("common.cancel")}</Button>
            <Button color="red" onClick={() => {
              setConfirmOpen(false);
              void compression.start(readyPaths, preset, true, true);
            }}>{t("compression.confirm")}</Button>
          </Group>
        </Stack>
      </Modal>
    </main>
  );
}

function CompressionTreeNode({ disabled, expanded, level, node, onRemove, onToggle }: {
  disabled: boolean;
  expanded: Set<string>;
  level: number;
  node: CompressionScanNode;
  onRemove: (path: string) => void;
  onToggle: (path: string) => void;
}) {
  const { t } = useTranslation();
  const directory = node.kind !== "file";
  const open = expanded.has(node.path);
  const [visibleCount, setVisibleCount] = useState(200);
  const visibleChildren = node.children.slice(0, visibleCount);
  const remaining = node.children.length - visibleChildren.length;
  return (
    <div role="treeitem" aria-expanded={directory ? open : undefined}>
      <div className="compression-tree-row" data-unavailable={!node.ready && !directory || undefined} style={{ "--tree-indent": `${level * 18}px` } as CSSProperties}>
        {directory ? (
          <UnstyledButton aria-label={open ? t("common.collapse") : t("common.expand")} className="tree-toggle" onClick={() => onToggle(node.path)}>
            {open ? <ChevronDown size={15} /> : <ChevronRight size={15} />}
          </UnstyledButton>
        ) : <span className="tree-toggle-spacer" />}
        {directory ? <Folder size={17} /> : <FileAudio size={17} />}
        <Text className="compression-tree-name" lineClamp={1} size="sm" title={node.path}>{node.name}</Text>
        {node.issueCode && <Badge color="yellow" size="xs" variant="light">{t(`compression.issues.${node.issueCode}`)}</Badge>}
        <Tooltip label={t("common.remove")}>
          <ActionIcon aria-label={t("common.remove")} disabled={disabled} onClick={() => onRemove(node.path)} size="sm" variant="subtle">
            <X size={14} />
          </ActionIcon>
        </Tooltip>
      </div>
      {directory && open && (
        <>
          {visibleChildren.map((child) => (
            <CompressionTreeNode
              disabled={disabled}
              expanded={expanded}
              key={child.path}
              level={level + 1}
              node={child}
              onRemove={onRemove}
              onToggle={onToggle}
            />
          ))}
          {remaining > 0 && (
            <Button
              className="compression-tree-more"
              onClick={() => setVisibleCount((count) => count + 200)}
              size="compact-xs"
              variant="subtle"
            >{t("compression.showMore", { count: Math.min(remaining, 200) })}</Button>
          )}
        </>
      )}
    </div>
  );
}

function CompressionTaskProgress({ snapshot }: {
  snapshot: ReturnType<typeof useAudioCompression>["snapshot"];
}) {
  const { t } = useTranslation();
  if (snapshot.total === 0) return null;
  const value = ((snapshot.completed + snapshot.currentProgress) / snapshot.total) * 100;
  return (
    <div className="compression-task-progress">
      <Progress animated={snapshot.status === "running"} value={value} />
      <Text c="dimmed" size="xs">{t("compression.progress", { completed: snapshot.completed, total: snapshot.total })}</Text>
      <ScrollArea.Autosize mah={180} type="auto">
        <div className="compression-results">
          {snapshot.items.map((item) => (
            <Text c={item.status === "failed" || item.message ? "red" : "dimmed"} key={item.source} lineClamp={1} size="xs" title={item.message || item.output}>
              {item.sourceDeleted ? t("compression.deleted", { path: item.source }) : item.message || item.output}
            </Text>
          ))}
        </div>
      </ScrollArea.Autosize>
    </div>
  );
}

function CompressionEmpty({ icon, label }: { icon: ReactNode; label: string }) {
  return (
    <div className="compression-empty">
      <ThemeIcon color="gray" size={46} variant="light">{icon}</ThemeIcon>
      <Text c="dimmed" size="sm">{label}</Text>
    </div>
  );
}

function collectReadyPaths(nodes: CompressionScanNode[]): string[] {
  return nodes.flatMap((node) => [
    ...(node.kind === "file" && node.ready ? [node.path] : []),
    ...collectReadyPaths(node.children),
  ]);
}
