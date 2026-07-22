import { useLayoutEffect, useRef, useState } from "react";
import type { RefObject } from "react";

export type DesktopLyricsLayoutMode = "single" | "wrapped" | "scrolling";

interface DesktopLyricsLayout {
  containerRef: RefObject<HTMLDivElement | null>;
  marqueeDurationSeconds: number;
  marqueeDistance: number;
  mode: DesktopLyricsLayoutMode;
  nowrapMeasureRef: RefObject<HTMLSpanElement | null>;
  wrappedMeasureRef: RefObject<HTMLDivElement | null>;
}

const initialLayout = {
  marqueeDistance: 0,
  mode: "single" as DesktopLyricsLayoutMode,
};

export function useDesktopLyricsLayout(text: string, fontSize: number): DesktopLyricsLayout {
  const containerRef = useRef<HTMLDivElement>(null);
  const nowrapMeasureRef = useRef<HTMLSpanElement>(null);
  const wrappedMeasureRef = useRef<HTMLDivElement>(null);
  const [layout, setLayout] = useState(initialLayout);

  useLayoutEffect(() => {
    const container = containerRef.current;
    const nowrapMeasure = nowrapMeasureRef.current;
    const wrappedMeasure = wrappedMeasureRef.current;
    if (!container || !nowrapMeasure || !wrappedMeasure) return;

    const measure = () => {
      const lineHeight = Number.parseFloat(getComputedStyle(wrappedMeasure).lineHeight);
      if (!Number.isFinite(lineHeight) || lineHeight <= 0) return;
      const wrappedHeight = wrappedMeasure.getBoundingClientRect().height;
      const lineCount = Math.max(1, Math.ceil((wrappedHeight - 1) / lineHeight));
      const marqueeDistance = Math.max(0, nowrapMeasure.scrollWidth - container.clientWidth);
      const mode: DesktopLyricsLayoutMode = lineCount <= 1
        ? "single"
        : lineCount <= 2 || marqueeDistance === 0
          ? "wrapped"
          : "scrolling";
      const next = {
        marqueeDistance: mode === "scrolling" ? marqueeDistance : 0,
        mode,
      };
      setLayout((current) => current.mode === next.mode
        && current.marqueeDistance === next.marqueeDistance
        ? current
        : next);
    };

    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(container);
    return () => observer.disconnect();
  }, [fontSize, text]);

  return {
    containerRef,
    marqueeDurationSeconds: Math.min(30, Math.max(8, layout.marqueeDistance / 36)),
    marqueeDistance: layout.marqueeDistance,
    mode: layout.mode,
    nowrapMeasureRef,
    wrappedMeasureRef,
  };
}
