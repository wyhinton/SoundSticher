import { writable } from 'svelte/store';

interface DebugState {
  timelineDebugMode: boolean;
  useCustomContextMenu: boolean;
  // Add other debug modes here as needed
}

const initialDebugState: DebugState = {
  timelineDebugMode: false,
  useCustomContextMenu: false, // Default to custom context menu
};

export const debugState = writable<DebugState>(initialDebugState);

// Convenience functions for specific debug modes
export const timelineDebugMode = {
  subscribe: debugState.subscribe,
  toggle: () =>
    debugState.update(state => ({
      ...state,
      timelineDebugMode: !state.timelineDebugMode,
    })),
  set: (value: boolean) =>
    debugState.update(state => ({
      ...state,
      timelineDebugMode: value,
    })),
};

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
