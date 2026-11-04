import { Tooltip } from "@mantine/core";
import type { ReactElement } from "react";

export function Hint({ children, disabled, label }: {
  children: ReactElement;
  disabled?: boolean;
  label: string | null | undefined;
}) {
  return <Tooltip disabled={disabled || !label} label={label}>{children}</Tooltip>;
}
