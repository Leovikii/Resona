import type { ReactElement } from "react";
import { Fragment } from "react";
import { Menu } from "@mantine/core";
import type { LucideIcon } from "lucide-react";

export interface AppContextMenuItem {
  destructive?: boolean;
  disabled?: boolean;
  dividerBefore?: boolean;
  icon: LucideIcon;
  id: string;
  label: string;
  onSelect: () => void;
}

interface AppContextMenuProps {
  children: ReactElement;
  items: AppContextMenuItem[];
}

export function AppContextMenu({ children, items }: AppContextMenuProps) {
  return (
    <Menu shadow="md" width={190} withinPortal>
      <Menu.ContextMenu>{children}</Menu.ContextMenu>
      <Menu.Dropdown>
        {items.map((item) => {
          const Icon = item.icon;
          return (
            <Fragment key={item.id}>
              {item.dividerBefore && <Menu.Divider />}
              <Menu.Item
                color={item.destructive ? "red" : undefined}
                disabled={item.disabled}
                leftSection={<Icon size={15} strokeWidth={1.8} />}
                onClick={item.onSelect}
              >
                {item.label}
              </Menu.Item>
            </Fragment>
          );
        })}
      </Menu.Dropdown>
    </Menu>
  );
}
