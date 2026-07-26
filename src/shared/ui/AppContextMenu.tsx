import type { ReactElement } from "react";
import { Fragment, useCallback, useEffect, useRef, useState } from "react";
import { Menu } from "@mantine/core";
import type { LucideIcon } from "lucide-react";

interface ActiveContextMenu {
  close: () => void;
  id: symbol;
}

let activeContextMenu: ActiveContextMenu | null = null;

export function closeActiveContextMenu() {
  const current = activeContextMenu;
  activeContextMenu = null;
  current?.close();
}

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
  const [opened, setOpened] = useState(false);
  const menuId = useRef(Symbol("app-context-menu"));
  const closeMenu = useCallback(() => setOpened(false), []);

  useEffect(() => () => {
    if (activeContextMenu?.id === menuId.current) {
      activeContextMenu = null;
    }
  }, []);

  useEffect(() => {
    if (!opened) return;
    window.addEventListener("blur", closeActiveContextMenu);
    return () => window.removeEventListener("blur", closeActiveContextMenu);
  }, [opened]);

  const handleOpenedChange = useCallback((nextOpened: boolean) => {
    if (nextOpened) {
      if (activeContextMenu?.id !== menuId.current) {
        closeActiveContextMenu();
      }
      activeContextMenu = { close: closeMenu, id: menuId.current };
    } else if (activeContextMenu?.id === menuId.current) {
      activeContextMenu = null;
    }
    setOpened(nextOpened);
  }, [closeMenu]);

  return (
    <Menu onChange={handleOpenedChange} opened={opened} shadow="md" width={190} withinPortal>
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
