// dragStore.ts
import { writable } from 'svelte/store';
import { logger } from './logging';

export type DragItem<T = any> = {
  type: 'operation'; // e.g. "sample", "clip", "node"
  payload?: T;
  sourceId?: string;
};

type DragState = {
  item: DragItem | null;
  overTargetId: string | null;
};

const initialState: DragState = {
  item: null,
  overTargetId: null,
};

export const dragStore = writable<DragState>(initialState);

// Wrap store to add logging
export const dragStoreWithLogging = {
  subscribe: dragStore.subscribe,
  set: (state: DragState) => {
    logger.dragStore.state(`Setting drag state`, state);
    if (state.item) {
      logger.dragStore.dragItem(
        `Dragging item of type "${state.item.type}" from source "${state.item.sourceId || 'unknown'}"`,
        state.item
      );
    }
    if (state.overTargetId) {
      logger.dragStore.overTarget(`Over target: ${state.overTargetId}`, state.overTargetId);
    }
    dragStore.set(state);
  },
  update: (fn: (value: DragState) => DragState) => {
    dragStore.update(current => {
      const next = fn(current);
      logger.dragStore.state(`Updating drag state`, { from: current, to: next });
      return next;
    });
  },
  clear: () => {
    logger.dragStore.clear('Clearing drag state');
    dragStore.set(initialState);
  },
};
