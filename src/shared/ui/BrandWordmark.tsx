import { memo } from "react";

import wordmarkAsset from "../../../assets/resona-resonance-wordmark.svg";

interface BrandWordmarkProps {
  className?: string;
}

export const BrandWordmark = memo(function BrandWordmark({ className }: BrandWordmarkProps) {
  return (
    <img
      alt="Resona"
      className={["brand-wordmark", className].filter(Boolean).join(" ")}
      draggable={false}
      src={wordmarkAsset}
    />
  );
});
