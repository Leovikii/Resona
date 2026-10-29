import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { CSSProperties, DragEvent, ReactNode } from "react";
import { onFileDragDropEvent } from "../shared/bridge/fileDrop";
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
  CircleCheck,
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

import { usePlaybackShortcuts } from "../features/playback/usePlaybackShortcuts";
import { TransientNotice, type TransientNoticeMessage } from "../shared/ui/TransientNotice";
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
import { WindowTitleBar } from "../shared/ui/WindowTitleBar";
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
  const [playbackNotice, setPlaybackNotice] = useState<TransientNoticeMessage | null>(null);
  const dismissPlaybackNotice = useCallback(() => setPlaybackNotice(null), []);
  const shortcuts = usePlaybackShortcuts({ blocked: confirmOpen });
  useEffect(() => {
    const feedback = shortcuts.feedback;
    if (!feedback) return;
    const volume = feedback.action.startsWith("volume_");
    const snapshot = feedback.snapshot;
    setPlaybackNotice({ id: feedback.id, tone: "info",
      message: volume ? t("playback.volumeFeedback", { volume: Math.round(snapshot.volume * 100) })
        : t("playback.controlFeedback", { status: t(`playback.${snapshot.status}`), position: Math.floor(snapshot.positionMs / 1000) }),
      progress: volume ? snapshot.volume * 100 : undefined });
  }, [shortcuts.feedback, t]);
  useEffect(() => {
    if (shortcuts.error) setPlaybackNotice({ id: Date.now(), tone: "error", message: t(`errors.${shortcuts.error.code}`, { defaultValue: shortcuts.error.message }) });
  }, [shortcuts.error, t]);
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
  const locked = scanBusy || taskBusy || compression.pending;
  const finished = compression.snapshot.status !== "idle" && !taskBusy;
  const readyPaths = useMemo(() => collectReadyPaths(compression.scan.roots), [compression.scan.roots]);
  const readyBytes = useMemo(() => collectReadyBytes(compression.scan.roots), [compression.scan.roots]);
  const taskItems = useMemo(
    () => new Map(compression.snapshot.items.map((item) => [item.source, item])),
    [compression.snapshot.items],
  );
  const canStart = !locked && !finished && compression.scan.status === "ready" && readyPaths.length > 0;
  const resultTone = compression.snapshot.status === "cancelled" ? "cancelled"
    : compression.snapshot.status === "completed" ? "success"
    : overall.succeeded > 0 ? "warning" : "failure";
  const resultTitle = t(
    compression.snapshot.status === "cancelled" ? "compression.resultCancelled"
    : compression.snapshot.status === "completed" ? "compression.resultSuccess"
    : overall.failed === 0 && overall.warnings > 0 ? "compression.resultWarning"
    : overall.succeeded > 0 ? "compression.resultPartial" : "compression.resultFailure",
  );
  const savedBytes = overall.sourceBytes - overall.outputBytes;
  const summary = compression.snapshot.total === 0
    ? t("compression.summaryReady", { count: readyPaths.length, size: formatBytes(readyBytes) })
    : formatBytes(compression.snapshot.items.reduce((total, item) => total + item.sourceBytes, 0));

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
    void onFileDragDropEvent((event) => {
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

  const reportSelectionError = (error: unknown) => {
    const code = typeof error === "object" && error !== null && "code" in error && typeof error.code === "string"
      ? error.code : "dialog_open_failed";
    setPlaybackNotice({ id: Date.now(), tone: "error", message: t(`errors.${code}`, { defaultValue: t("errors.dialog_open_failed") }) });
  };
  const addFiles = async () => {
    try {
      const paths = await selectWavFiles();
      if (paths.length > 0) void compression.addInputs(paths);
    } catch (error) { reportSelectionError(error); }
  };
  const addFolders = async () => {
    try {
      const paths = await selectCompressionFolders();
      if (paths.length > 0) void compression.addInputs(paths);
    } catch (error) { reportSelectionError(error); }
  };
  const start = () => {
    if (!canStart) return;
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
      <TransientNotice autoClose={1800} closeLabel={t("common.close")} notice={playbackNotice} onDismiss={dismissPlaybackNotice} />
      <WindowTitleBar title={`${t("app.name")} · ${t("tools.compression")}`} />

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
          disabled={taskBusy || compression.pending}
          onChange={(value) => setCompression({ preset: value as CompressionPreset })}
          size="xs"
          value={preset}
        />
        <Group className="compression-run-actions" gap="sm" wrap="nowrap">
          <Switch
            checked={deleteSource}
            disabled={taskBusy || compression.pending}
            label={t("compression.deleteSource")}
            onChange={(event) => setCompression({ deleteSource: event.currentTarget.checked })}
            size="xs"
          />
          {taskBusy ? (
            <Button color="red" onClick={() => void compression.cancel()} size="xs" variant="light">
              {t("common.cancel")}
            </Button>
          ) : (
            <Button
              className="compression-start"
              data-result={finished ? resultTone : undefined}
              aria-describedby={finished ? "compression-next-batch" : undefined}
              disabled={!canStart}
              leftSection={finished && resultTone === "success" ? <Check size={15} /> : <Wrench size={15} />}
              onClick={start} size="xs"
            >
              {finished ? t(compression.snapshot.status === "completed" ? "compression.completed"
                : compression.snapshot.status === "cancelled" ? "compression.cancelled" : "compression.ended")
                : t("compression.start", { count: readyPaths.length })}
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
                <Text c="dimmed" key={`${warning.code}-${warning.path}`} lineClamp={2} size="xs" title={`${t(`compression.scanWarnings.${warning.code}`, { defaultValue: warning.message })}: ${warning.path}`}>
                  {t(`compression.scanWarnings.${warning.code}`, { defaultValue: warning.message })} · {warning.path}
                </Text>
              ))}
            </div>
          )}
          {compression.error && <div className="compression-error" role="alert">{compression.error}</div>}
        </section>
      )}

      <div className="compression-workspace">
        <section className="compression-tree-panel" data-finished={finished || undefined} aria-label={t(finished ? "compression.results" : "compression.inputs")}>
          <div className="compression-panel-heading" data-status={compression.snapshot.status} data-result={finished ? resultTone : undefined}>
            {compression.snapshot.total > 0 && <span
              className="compression-tree-progress"
              style={{ width: `${overall.percent}%` }}
              role="progressbar"
              aria-label={t("compression.overallProgress")}
              aria-valuemin={0}
              aria-valuemax={100}
              aria-valuenow={overall.percent}
            />}
            <VisuallyHidden role="status">{finished ? resultTitle : compression.snapshot.total > 0 ? t(`compression.batchAnnouncement.${compression.snapshot.status}`) : ""}</VisuallyHidden>
            {finished ? (
              <div className="compression-result">
                <div className="compression-result-main">
                  <span className="compression-result-icon" aria-hidden="true">
                    {resultTone === "success" ? <CircleCheck size={29} /> : resultTone === "failure" ? <CircleX size={29} />
                      : resultTone === "warning" ? <CircleAlert size={29} /> : <X size={29} />}
                  </span>
                  <div>
                    <Title order={2}>{resultTitle}</Title>
                    <Text c="dimmed" size="sm">{t("compression.resultCounts", overall)}</Text>
                  </div>
                </div>
                <div className="compression-result-savings">
                  {overall.succeeded > 0 ? <>
                    <Text className="compression-result-saved" fw={750}>{t(savedBytes >= 0 ? "compression.resultSaved" : "compression.resultGrew", { size: formatBytes(Math.abs(savedBytes)) })}</Text>
                    <Text c="dimmed" size="xs">{t(savedBytes >= 0 ? "compression.resultReduction" : "compression.resultIncrease", {
                      percent: overall.sourceBytes > 0 ? Math.round(Math.abs(savedBytes) / overall.sourceBytes * 100) : 0,
                      source: formatBytes(overall.sourceBytes), output: formatBytes(overall.outputBytes),
                    })}</Text>
                  </> : <Text c="dimmed" size="sm">{t("compression.resultNoOutput")}</Text>}
                </div>
                <div className="compression-result-details">
                  <Text size="xs">{overall.succeeded > 0 ? t("compression.resultGenerated", { count: overall.succeeded }) : t("compression.resultCheckErrors")}
                    {overall.deleted > 0 ? ` · ${t("compression.resultDeleted", { count: overall.deleted })}` : ""}
                    {overall.succeeded - overall.deleted > 0 ? ` · ${t("compression.resultRetained", { count: overall.succeeded - overall.deleted })}` : ""}
                    {overall.warnings > 0 ? ` · ${t("compression.resultWarnings", { count: overall.warnings })}` : ""}
                  </Text>
                  <Text c="dimmed" size="xs" id="compression-next-batch">{t("compression.nextBatch")}</Text>
                </div>
              </div>
            ) : <>
              <Text fw={650} size="sm">{t("compression.inputs")}</Text>
              <div className="compression-summary">
                {compression.snapshot.total > 0 && <Text fw={600} size="xs">{t(`compression.batchStatus.${compression.snapshot.status}`, {
                  percent: overall.percent, ended: compression.snapshot.completed, total: compression.snapshot.total,
                  running: overall.running, succeeded: overall.succeeded, failed: overall.failed, cancelled: overall.cancelled,
                })}</Text>}
                <Text c="dimmed" lineClamp={1} size="xs" title={summary}>{summary}</Text>
              </div>
            </>}
          </div>
          {finished && <Text className="compression-results-label" fw={650} size="sm">{t("compression.results")}</Text>}
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
            <Button color="red" disabled={!canStart} onClick={() => {
              setConfirmOpen(false);
              if (!canStart) return;
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

function formatBytes(bytes: number) {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const unit = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  const value = bytes / 1024 ** unit;
  return `${new Intl.NumberFormat(undefined, { maximumFractionDigits: unit === 0 ? 0 : 1 }).format(value)} ${units[unit]}`;
}
