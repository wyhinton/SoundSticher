import { writable } from 'svelte/store';

interface DebugState {
  useCustomContextMenu: boolean;
  // Add other debug modes here as needed
}

const initialDebugState: DebugState = {
  useCustomContextMenu: false, // Default to custom context menu
};

export const debugState = writable<DebugState>(initialDebugState);

// Convenience functions for specific debug modes
export const customContextMenu = {
  subscribe: debugState.subscribe,
  toggle: () =>
    debugState.update(state => ({
      ...state,
      useCustomContextMenu: !state.useCustomContextMenu,
    })),
  set: (value: boolean) =>
    debugState.update(state => ({
      ...state,
      useCustomContextMenu: value,
    })),
};
