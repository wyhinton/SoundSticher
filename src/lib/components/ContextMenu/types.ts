export interface ContextMenuPosition {
  x: number;
  y: number;
}

export interface ContextMenuItem {
  type?: 'item' | 'separator';
  label?: string;
  icon?: string;
  shortcut?: string;
  disabled?: boolean;
  variant?: 'default' | 'danger';
  action?: () => void;
  submenu?: ContextMenuItem[];
}

export interface ContextMenuConfig {
  items: ContextMenuItem[];
  position: ContextMenuPosition;
}

export type ContextMenuProvider = (context: ContextMenuContext) => ContextMenuConfig | null;

export interface ContextMenuContext {
  target: HTMLElement;
  event: MouseEvent;
  appState: any;
  selectedItems?: any[];
  hoveredItem?: any;
}
