import { writable } from 'svelte/store';
import type { ContextMenuConfig, ContextMenuProvider, ContextMenuContext } from './types';

interface ContextMenuState {
  visible: boolean;
  config: ContextMenuConfig | null;
}

const initialState: ContextMenuState = {
  visible: false,
  config: null,
};

export const contextMenuState = writable<ContextMenuState>(initialState);

class ContextMenuManager {
  private providers: Map<string, ContextMenuProvider> = new Map();

  registerProvider(id: string, provider: ContextMenuProvider) {
    this.providers.set(id, provider);
    console.log(`Registered context menu provider: ${id}`);
  }

  unregisterProvider(id: string) {
    this.providers.delete(id);
    console.log(`Unregistered context menu provider: ${id}`);
  }

  show(context: ContextMenuContext, providerId?: string) {
    let config: ContextMenuConfig | null = null;

    if (providerId && this.providers.has(providerId)) {
      // Use specific provider
      const provider = this.providers.get(providerId)!;
      config = provider(context);
    } else {
      // Try all providers until one returns a config
      for (const [id, provider] of this.providers.entries()) {
        try {
          config = provider(context);
          if (config) {
            console.log(`Context menu provided by: ${id}`);
            break;
          }
        } catch (error) {
          console.error(`Error in context menu provider ${id}:`, error);
        }
      }
    }

    if (config) {
      contextMenuState.set({
        visible: true,
        config,
      });
    }
  }

  hide() {
    contextMenuState.set({
      visible: false,
      config: null,
    });
  }
}

export const contextMenuManager = new ContextMenuManager();
