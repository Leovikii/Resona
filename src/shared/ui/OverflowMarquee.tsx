import { useEffect, useRef, useState } from "react";

export function OverflowMarquee({ auto = false, className, text }: {
  auto?: boolean;
  className?: string;
  text: string;
}) {
  const viewportRef = useRef<HTMLDivElement | null>(null);
  const textRef = useRef<HTMLSpanElement | null>(null);
  const [measurement, setMeasurement] = useState({ duration: 8, overflow: false });

  useEffect(() => {
    const viewport = viewportRef.current;
    const textElement = textRef.current;
    if (!viewport || !textElement) return;

    const measure = () => {
      const overflow = textElement.scrollWidth > viewport.clientWidth + 1;
      const duration = Math.max(6, Math.min(18, textElement.scrollWidth / 28));
      setMeasurement((current) => current.overflow === overflow && current.duration === duration
        ? current
        : { duration, overflow });
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(viewport);
    observer.observe(textElement);
    return () => observer.disconnect();
  }, [text]);

  return (
    <div
      className={["overflow-marquee", className].filter(Boolean).join(" ")}
      data-auto={auto || undefined}
      data-overflow={measurement.overflow || undefined}
      ref={viewportRef}
      style={{ "--marquee-duration": `${measurement.duration}s` } as React.CSSProperties}
      title={measurement.overflow ? text : undefined}
    >
      <span className="overflow-marquee-static" ref={textRef}>{text}</span>
      {measurement.overflow && (
        <span aria-hidden="true" className="overflow-marquee-moving">
          <span>{text}</span>
          <span>{text}</span>
        </span>
      )}
    </div>
  );
}
