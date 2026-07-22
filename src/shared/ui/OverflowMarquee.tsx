import { useCallback, useEffect, useRef, useState } from "react";

export function OverflowMarquee({ auto = false, className, observe = true, text }: {
  auto?: boolean;
  className?: string;
  observe?: boolean;
  text: string;
}) {
  const viewportRef = useRef<HTMLDivElement | null>(null);
  const textRef = useRef<HTMLSpanElement | null>(null);
  const [measurement, setMeasurement] = useState({ duration: 8, overflow: false });
  const measure = useCallback(() => {
    const viewport = viewportRef.current;
    const textElement = textRef.current;
    if (!viewport || !textElement) return;

    const textWidth = textElement.getBoundingClientRect().width;
    const overflow = textWidth > viewport.clientWidth + 1;
    const duration = Math.max(6, Math.min(18, textWidth / 28));
    setMeasurement((current) => current.overflow === overflow && current.duration === duration
      ? current
      : { duration, overflow });
  }, []);

  useEffect(() => {
    if (!observe) {
      setMeasurement((current) => current.overflow ? { ...current, overflow: false } : current);
      return;
    }
    const viewport = viewportRef.current;
    const textElement = textRef.current;
    if (!viewport || !textElement) return;

    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(viewport);
    observer.observe(textElement);
    return () => observer.disconnect();
  }, [measure, observe, text]);

  return (
    <div
      className={["overflow-marquee", className].filter(Boolean).join(" ")}
      data-auto={auto || undefined}
      data-overflow={measurement.overflow || undefined}
      onFocus={observe ? undefined : measure}
      onPointerDown={observe ? undefined : measure}
      onPointerEnter={observe ? undefined : measure}
      ref={viewportRef}
      style={{ "--marquee-duration": `${measurement.duration}s` } as React.CSSProperties}
      title={measurement.overflow ? text : undefined}
    >
      <span className="overflow-marquee-static">
        <span className="overflow-marquee-measure" ref={textRef}>{text}</span>
      </span>
      {measurement.overflow && (
        <span aria-hidden="true" className="overflow-marquee-moving">
          <span>{text}</span>
          <span>{text}</span>
        </span>
      )}
    </div>
  );
}
