import { debugState } from '../state/debug.svelte';
import { get } from 'svelte/store';
import type {
  ContextMenuConfig,
  ContextMenuProvider,
  ContextMenuContext,
} from '../components/ContextMenu/types';

/**
 * Context menu manager that handles whether to use custom or browser context menus
 */
export class ContextMenuManager {
  private static instance: ContextMenuManager;
  private contextMenuVisible = $state(false);
  private contextMenuConfig = $state<ContextMenuConfig | null>(null);

  public static getInstance(): ContextMenuManager {
    if (!ContextMenuManager.instance) {
      ContextMenuManager.instance = new ContextMenuManager();
    }
    return ContextMenuManager.instance;
  }

  /**
   * Show a context menu if custom context menus are enabled
   */
  public showContextMenu(
    event: MouseEvent,
    provider: ContextMenuProvider,
    context: Omit<ContextMenuContext, 'event'>
  ): boolean {
    const debugSettings = get(debugState);

    if (!debugSettings.useCustomContextMenu) {
      // Return false to let browser handle the context menu
      return false;
    }

    event.preventDefault();
    event.stopPropagation();

    const fullContext: ContextMenuContext = {
      ...context,
      event,
    };

    const config = provider(fullContext);

    if (config) {
      this.contextMenuConfig = config;
      this.contextMenuVisible = true;
      return true;
    }

    return false;
  }

  /**
   * Hide the custom context menu
   */
  public hideContextMenu(): void {
    this.contextMenuVisible = false;
    this.contextMenuConfig = null;
  }

  /**
   * Get current context menu state
   */
  public get isVisible(): boolean {
    return this.contextMenuVisible;
  }

  /**
   * Get current context menu configuration
   */
  public get config(): ContextMenuConfig | null {
    return this.contextMenuConfig;
  }

  /**
   * Check if custom context menus are enabled
   */
  public get isCustomContextMenuEnabled(): boolean {
    const debugSettings = get(debugState);
    return debugSettings.useCustomContextMenu;
  }
}

/**
 * Global context menu manager instance
 */
export const contextMenuManager = ContextMenuManager.getInstance();

/**
 * Helper function for components to handle context menus
 * Returns true if custom context menu was shown, false if browser should handle it
 */
export function handleContextMenu(
  event: MouseEvent,
  provider: ContextMenuProvider,
  context: Omit<ContextMenuContext, 'event'>
): boolean {
  return contextMenuManager.showContextMenu(event, provider, context);
}
