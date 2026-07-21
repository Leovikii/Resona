import { Button, Menu } from "@mantine/core";
import { ChevronDown, FileAudio, FolderPlus, Plus } from "lucide-react";

export function AddMediaMenu({
  buttonLabel,
  disabled,
  fileLabel,
  folderLabel,
  onAddFiles,
  onAddFolders,
}: {
  buttonLabel: string;
  disabled: boolean;
  fileLabel: string;
  folderLabel: string;
  onAddFiles: () => void;
  onAddFolders: () => void;
}) {
  return (
    <Menu position="bottom-end" shadow="md" width={180}>
      <Menu.Target>
        <Button
          className="add-media-button"
          disabled={disabled}
          leftSection={<Plus size={15} />}
          rightSection={<ChevronDown size={14} />}
          size="xs"
          variant="default"
        >
          {buttonLabel}
        </Button>
      </Menu.Target>
      <Menu.Dropdown>
        <Menu.Item leftSection={<FileAudio size={15} />} onClick={onAddFiles}>
          {fileLabel}
        </Menu.Item>
        <Menu.Item leftSection={<FolderPlus size={15} />} onClick={onAddFolders}>
          {folderLabel}
        </Menu.Item>
      </Menu.Dropdown>
    </Menu>
  );
}
