import { useEffect, useState } from "react";
import { ActionIcon, Tooltip } from "@mantine/core";
import { Copy, Minus, Settings, Square, Wrench, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import icon from "../../../assets/resona-icon.svg";
import { observeWindowState, performWindowAction, resizeWindow, type WindowAction } from "../bridge/windowControls";

const titleBarIconProps = { size: 16, strokeWidth: 1.5, absoluteStrokeWidth: true };

export function WindowTitleBar({ title = "Resona", compact = false, selection, onNavigate }: {
  title?: string;
  compact?: boolean;
  selection?: string;
  onNavigate?: (kind: "tools" | "settings") => void;
}) {
  const { t } = useTranslation();
  const [state, setState] = useState({ focused: true, maximized: false, nativeMaximizeHovered: false });
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let disposed = false;
    let stop: (() => void) | undefined;
    void observeWindowState(setState).then((unlisten) => {
      if (disposed) unlisten(); else stop = unlisten;
    }).catch((reason) => { console.error("Window state unavailable", reason); setError(String(reason)); });
    return () => { disposed = true; stop?.(); };
  }, []);
  const act = (action: WindowAction) => {
    void performWindowAction(action).catch((reason) => {
      console.error("Window action failed", reason);
      setError(String(reason));
    });
  };
  return (
    <header className="window-titlebar" data-compact={compact || undefined} data-focused={state.focused}>
      <div className="window-titlebar-drag" data-tauri-drag-region onContextMenu={(event) => { event.preventDefault(); act("menu"); }}>
        <img alt="" draggable={false} src={icon} />
        <span>{title}</span>
      </div>
      {compact && onNavigate && <nav className="window-titlebar-navigation" aria-label={t("app.name")}>
        {(["tools", "settings"] as const).map((kind) => <Tooltip key={kind} label={t(`nav.${kind}`)} events={{ hover: true, focus: true, touch: false }}>
          <ActionIcon aria-current={selection === kind ? "page" : undefined} aria-label={t(`nav.${kind}`)} className="window-titlebar-button" data-active={selection === kind || undefined} onClick={() => onNavigate(kind)} radius={0} size={44} variant="subtle">
            {kind === "tools" ? <Wrench {...titleBarIconProps} /> : <Settings {...titleBarIconProps} />}
          </ActionIcon>
        </Tooltip>)}
      </nav>}
      <nav className="window-caption-buttons" aria-label={t("window.controls")}>
        {(["minimize", "maximize", "close"] as const).map((action) => {
          const label = t(`window.${action === "maximize" && state.maximized ? "restore" : action}`);
          return <Tooltip key={action} label={label} events={{ hover: true, focus: true, touch: false }}>
            <ActionIcon aria-label={label} className="window-titlebar-button" data-caption={action} data-native-hover={action === "maximize" && state.nativeMaximizeHovered || undefined} onClick={() => act(action)} radius={0} size={44} variant="subtle">
              {action === "minimize" ? <Minus {...titleBarIconProps} /> : action === "close" ? <X {...titleBarIconProps} /> : state.maximized ? <Copy {...titleBarIconProps} /> : <Square {...titleBarIconProps} />}
            </ActionIcon>
          </Tooltip>;
        })}
      </nav>
      {error && <button className="window-control-error" role="alert" onClick={() => setError(null)}>{t("window.failed")}: {error}</button>}
      {!state.maximized && (["North", "South", "East", "West", "NorthEast", "NorthWest", "SouthEast", "SouthWest"] as const).map((direction) =>
        <span key={direction} aria-hidden="true" className="window-resize-edge" data-direction={direction} onPointerDown={(event) => {
          if (event.button !== 0) return;
          event.preventDefault();
          void resizeWindow(direction).catch((reason) => { console.error("Window resize failed", reason); setError(String(reason)); });
        }} />)}
    </header>
  );
}
