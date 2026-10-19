import { useEffect, useMemo, useRef, useState } from "react";
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
  VisuallyHidden,
} from "@mantine/core";
import {
  ChevronDown,
  ChevronRight,
  Check,
  CircleAlert,
  CircleX,
  FileAudio,
  Folder,
  FolderOpen,
  Trash2,
  Wrench,
  X,
} from "lucide-react";
import { useTranslation } from "react-i18next";

import { compressionProgress } from "../features/compression/compressionProgress";
import { useAudioCompression } from "../features/compression/useAudioCompression";
import { selectCompressionFolders, selectWavFiles } from "../shared/bridge/audioDialog";
import { isTauriRuntime } from "../shared/bridge/tauri";
import { initializeCurrentWindowMaterial } from "../shared/bridge/windowAppearance";
import type {
  CompressionItem,
  CompressionPreset,
  CompressionScanNode,
} from "../shared/model/compression";
import { AddMediaMenu } from "../shared/ui/AddMediaMenu";
import {
  indexCompressionAncestors,
  reconcileCompressionActivityExpansion,
  reconcileCompressionRootExpansion,
  toggleCompressionDirectory,
  type CompressionActivityExpansion,
} from "./compressionRootExpansion";
import { usePreferences } from "./preferences";

export default function AudioCompressionApp() {
  const { t } = useTranslation();
  const compression = useAudioCompression();
  const { compression: preferences, setCompression } = usePreferences();
  const { deleteSource, preset } = preferences;
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [dragActive, setDragActive] = useState(false);
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const expandedRootsRef = useRef({ scanId: 0, seenRoots: new Set<string>() });
  const activityRef = useRef<CompressionActivityExpansion>({ taskId: 0, seenItems: new Set(), collapsed: new Set() });
  const ancestors = useMemo(() => indexCompressionAncestors(compression.scan.roots), [compression.scan.roots]);
  const runningSources = useMemo(() => compression.snapshot.status === "running"
    ? compression.snapshot.items.filter((item) => item.status === "running").map((item) => item.source)
    : [], [compression.snapshot]);
  const runningByDirectory = useMemo(() => {
    const counts = new Map<string, number>();
    for (const source of runningSources) {
      for (const parent of ancestors.get(source) ?? []) counts.set(parent, (counts.get(parent) ?? 0) + 1);
    }
    return counts;
  }, [ancestors, runningSources]);
  const overall = compressionProgress(compression.snapshot);
  const scanBusy = compression.scan.status === "scanning" || compression.scan.status === "cancelling";
  const taskBusy = compression.snapshot.status === "running" || compression.snapshot.status === "cancelling";
  const locked = scanBusy || taskBusy;
  const readyPaths = useMemo(() => collectReadyPaths(compression.scan.roots), [compression.scan.roots]);
  const readyBytes = useMemo(() => collectReadyBytes(compression.scan.roots), [compression.scan.roots]);
  const taskItems = useMemo(
    () => new Map(compression.snapshot.items.map((item) => [item.source, item])),
    [compression.snapshot.items],
  );
  const summary = compressionSummary(compression.snapshot, readyPaths.length, readyBytes, t);

  useEffect(() => {
    document.documentElement.dataset.window = "audio-compression";
    document.title = `${t("app.name")} - ${t("tools.compression")}`;
    if (isTauriRuntime()) {
      void getCurrentWindow().setTitle(document.title);
      void initializeCurrentWindowMaterial()
        .then(() => getCurrentWindow().show())
        .catch((error) => {
          console.error("Unable to show the audio compression window", error);
        });
    }
    return () => {
      delete document.documentElement.dataset.window;
    };
  }, [t]);

  useEffect(() => {
    const result = reconcileCompressionRootExpansion(
      expandedRootsRef.current.scanId,
      expandedRootsRef.current.seenRoots,
      compression.scan.scanId,
      compression.scan.roots.map((root) => root.path),
    );
    if (result.reset) activityRef.current = { taskId: 0, seenItems: new Set(), collapsed: new Set() };
    expandedRootsRef.current = {
      scanId: result.scanId,
      seenRoots: result.seenRoots,
    };
    if (!result.reset && result.pathsToExpand.length === 0) return;
    setExpanded((current) => {
      const next = result.reset ? new Set<string>() : new Set(current);
      result.pathsToExpand.forEach((path) => next.add(path));
      return next;
    });
  }, [compression.scan.roots, compression.scan.scanId]);

  useEffect(() => {
    const result = reconcileCompressionActivityExpansion(activityRef.current, compression.snapshot.taskId, runningSources, ancestors);
    activityRef.current = result;
    if (result.pathsToExpand.size === 0) return;
    setExpanded((current) => {
      if ([...result.pathsToExpand].every((path) => current.has(path))) return current;
      return new Set([...current, ...result.pathsToExpand]);
    });
  }, [ancestors, compression.snapshot.taskId, runningSources, expanded]);

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
          <ThemeIcon size={32} variant="light"><Wrench size={17} /></ThemeIcon>
          <div className="compression-window-title">
            <Title order={1}>{t("tools.compression")}</Title>
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
          onChange={(value) => setCompression({ preset: value as CompressionPreset })}
          size="xs"
          value={preset}
        />
        <Group className="compression-run-actions" gap="sm" wrap="nowrap">
          <Switch
            checked={deleteSource}
            disabled={taskBusy}
            label={t("compression.deleteSource")}
            onChange={(event) => setCompression({ deleteSource: event.currentTarget.checked })}
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

      {(scanBusy || compression.scan.warnings.length > 0 || compression.error) && (
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
          <div className="compression-panel-heading" data-status={compression.snapshot.status}>
            {compression.snapshot.total > 0 && <span
              className="compression-tree-progress"
              style={{ width: `${overall.percent}%` }}
              role="progressbar"
              aria-label={t("compression.overallProgress")}
              aria-valuemin={0}
              aria-valuemax={100}
              aria-valuenow={overall.percent}
            />}
            <Text fw={650} size="sm">{t("compression.inputs")}</Text>
            <VisuallyHidden role="status">{compression.snapshot.total > 0 ? t(`compression.batchAnnouncement.${compression.snapshot.status}`) : ""}</VisuallyHidden>
            <div className="compression-summary">
              {compression.snapshot.total > 0 && <Text fw={600} size="xs">{t(`compression.batchStatus.${compression.snapshot.status}`, {
                percent: overall.percent, ended: compression.snapshot.completed, total: compression.snapshot.total,
                running: overall.running, succeeded: overall.succeeded, failed: overall.failed, cancelled: overall.cancelled,
              })}</Text>}
              <Text c="dimmed" lineClamp={1} size="xs" title={summary}>{summary}</Text>
            </div>
          </div>
          <ScrollArea className="compression-tree-scroll" scrollHideDelay={700} type="scroll">
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
                    taskItems={taskItems}
                    runningByDirectory={runningByDirectory}
                    onRemove={(path) => void compression.removeInputs([path])}
                    onToggle={(path) => {
                      toggleCompressionDirectory(activityRef.current, path, expanded.has(path), ancestors);
                      setExpanded((current) => {
                        const next = new Set(current);
                        if (next.has(path)) next.delete(path);
                        else next.add(path);
                        return next;
                      });
                    }}
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

function CompressionTreeNode({ disabled, expanded, level, node, onRemove, onToggle, taskItems, runningByDirectory }: {
  disabled: boolean;
  expanded: Set<string>;
  level: number;
  node: CompressionScanNode;
  taskItems: Map<string, CompressionItem>;
  runningByDirectory: Map<string, number>;
  onRemove: (path: string) => void;
  onToggle: (path: string) => void;
}) {
  const { t } = useTranslation();
  const directory = node.kind !== "file";
  const open = expanded.has(node.path);
  const [visibleCount, setVisibleCount] = useState(200);
  const visibleChildren = node.children.slice(0, visibleCount);
  const remaining = node.children.length - visibleChildren.length;
  const hiddenRunning = node.children.slice(visibleCount).reduce((count, child) => count
    + (child.kind === "file" ? Number(taskItems.get(child.path)?.status === "running") : runningByDirectory.get(child.path) ?? 0), 0);
  const item = directory ? undefined : taskItems.get(node.path);
  const progress = item ? Math.round(item.progress * 100) : 0;
  return (
    <div role="treeitem" aria-expanded={directory ? open : undefined}>
      <div
        className="compression-tree-row"
        data-status={item?.status}
        data-unavailable={!node.ready && !directory || undefined}
        style={{ "--tree-indent": `${level * 18}px` } as CSSProperties}
        title={item?.message || undefined}
      >
        {item && <span className="compression-tree-progress" style={{ width: `${progress}%` }} />}
        {directory ? (
          <UnstyledButton aria-label={open ? t("common.collapse") : t("common.expand")} className="tree-toggle" onClick={() => onToggle(node.path)}>
            {open ? <ChevronDown size={15} /> : <ChevronRight size={15} />}
          </UnstyledButton>
        ) : <span className="tree-toggle-spacer" />}
        {directory ? <Folder size={17} /> : <FileAudio size={17} />}
        <Text className="compression-tree-name" lineClamp={1} size="sm" title={node.path}>{node.name}</Text>
        <span className="compression-tree-meta">
          {directory && !open && (runningByDirectory.get(node.path) ?? 0) > 0 && <Text size="xs">{t("compression.activeCount", { count: runningByDirectory.get(node.path) })}</Text>}
          {node.issueCode && <Badge color="yellow" size="xs" variant="light">{t(`compression.issues.${node.issueCode}`)}</Badge>}
          {item?.status === "running" && <Text c="dimmed" size="xs">{progress}%</Text>}
          {item?.status === "completed" && (
            <Tooltip label={item.sourceDeleted ? t("compression.sourceDeleted") : t("compression.completed")}>
              <span className="compression-item-state compression-item-state-completed"><Check size={14} />{formatBytes(item.outputBytes)}</span>
            </Tooltip>
          )}
          {item?.status === "failed" && (
            <Tooltip label={item.message || t("compression.failed")}><CircleX className="compression-item-state-failed" size={15} /></Tooltip>
          )}
          {item?.status === "cancelled" && <Text c="dimmed" size="xs">{t("compression.cancelled")}</Text>}
          {!item && node.kind === "file" && <Text c="dimmed" size="xs">{formatBytes(node.sourceBytes)}</Text>}
        </span>
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
              taskItems={taskItems}
              runningByDirectory={runningByDirectory}
            />
          ))}
          {hiddenRunning > 0 && <Text className="compression-tree-more" c="dimmed" size="xs">{t("compression.hiddenRunning", { count: hiddenRunning })}</Text>}
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

function collectReadyBytes(nodes: CompressionScanNode[]): number {
  return nodes.reduce(
    (total, node) => total + (node.kind === "file" && node.ready ? node.sourceBytes : 0) + collectReadyBytes(node.children),
    0,
  );
}

function compressionSummary(
  snapshot: ReturnType<typeof useAudioCompression>["snapshot"],
  readyCount: number,
  readyBytes: number,
  t: ReturnType<typeof useTranslation>["t"],
) {
  if (snapshot.total === 0) {
    return t("compression.summaryReady", { count: readyCount, size: formatBytes(readyBytes) });
  }
  const { sourceBytes, outputBytes, failed, cancelled } = compressionProgress(snapshot);
  if (snapshot.status === "running" || snapshot.status === "cancelling") {
    const allSourceBytes = snapshot.items.reduce((total, item) => total + item.sourceBytes, 0);
    return t("compression.summaryRunning", {
      completed: snapshot.completed,
      size: formatBytes(allSourceBytes),
      total: snapshot.total,
    });
  }
  const savedBytes = Math.max(0, sourceBytes - outputBytes);
  const percent = sourceBytes > 0 ? Math.round((savedBytes / sourceBytes) * 100) : 0;
  return t("compression.summaryComplete", {
    cancelled: cancelled > 0 ? ` · ${t("compression.cancelledCount", { count: cancelled })}` : "",
    failed: failed > 0 ? ` · ${t("compression.failedCount", { count: failed })}` : "",
    output: formatBytes(outputBytes),
    percent,
    saved: formatBytes(savedBytes),
    source: formatBytes(sourceBytes),
  });
}

function formatBytes(bytes: number) {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const unit = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  const value = bytes / 1024 ** unit;
  return `${new Intl.NumberFormat(undefined, { maximumFractionDigits: unit === 0 ? 0 : 1 }).format(value)} ${units[unit]}`;
}
