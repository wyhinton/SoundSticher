import { writable, derived, get } from 'svelte/store';
import { logger } from './logging';

export type SelectionMode = 'replace' | 'add' | 'remove' | 'toggle';

export interface SelectionOperation {
  mode: SelectionMode;
  ids: number[];
  source?: 'groups' | 'timeline' | 'shortcut' | 'api';
}

export interface SelectionState {
  ids: Set<number>;
  source: string | null;
  timestamp: number;
}

export interface PreviewState {
  ids: Set<number>;
  source: string | null;
  timestamp: number;
  active: boolean; // Whether preview is currently active
}

// Internal store state
const createSelectionStore = () => {
  const { subscribe, set, update } = writable<SelectionState>({
    ids: new Set<number>(),
    source: null,
    timestamp: Date.now(),
  });

  return {
    subscribe,

    // Core selection operations
    set: (ids: Iterable<number>, source: string = 'api') => {
      const newIds = new Set(ids);
      logger.selection.action(`Set selection to ${newIds.size} items from ${source}`, {
        ids: Array.from(newIds),
        source,
      });
      set({
        ids: newIds,
        source,
        timestamp: Date.now(),
      });
    },

    add: (id: number, source: string = 'api') => {
      logger.selection.action(`Add segment ${id} from ${source}`, { id, source });
      update(state => ({
        ids: new Set([...state.ids, id]),
        source,
        timestamp: Date.now(),
      }));
    },

    remove: (id: number, source: string = 'api') => {
      logger.selection.action(`Remove segment ${id} from ${source}`, { id, source });
      update(state => {
        const newIds = new Set(state.ids);
        newIds.delete(id);
        return {
          ids: newIds,
          source,
          timestamp: Date.now(),
        };
      });
    },

    toggle: (id: number, source: string = 'api') => {
      update(state => {
        const newIds = new Set(state.ids);
        const wasSelected = newIds.has(id);
        if (wasSelected) {
          newIds.delete(id);
        } else {
          newIds.add(id);
        }
        logger.selection.action(
          `Toggle segment ${id} (${wasSelected ? 'removed' : 'added'}) from ${source}`,
          {
            id,
            source,
            action: wasSelected ? 'removed' : 'added',
          }
        );
        return {
          ids: newIds,
          source,
          timestamp: Date.now(),
        };
      });
    },

    clear: (source: string = 'api') => {
      logger.selection.clear(`Clear all selections from ${source}`, { source });
      set({
        ids: new Set<number>(),
        source,
        timestamp: Date.now(),
      });
    },

    // High-level operation for complex selections
    apply: (operation: SelectionOperation) => {
      const currentState = get({ subscribe });
      let newIds: Set<number>;

      logger.selection.action(
        `Apply ${operation.mode} operation with ${operation.ids.length} items from ${operation.source || 'api'}`,
        {
          mode: operation.mode,
          ids: operation.ids,
          source: operation.source,
          currentCount: currentState.ids.size,
        }
      );

      switch (operation.mode) {
        case 'replace':
          newIds = new Set(operation.ids);
          break;

        case 'add':
          newIds = new Set([...currentState.ids, ...operation.ids]);
          break;

        case 'remove':
          newIds = new Set(currentState.ids);
          operation.ids.forEach(id => newIds.delete(id));
          break;

        case 'toggle':
          newIds = new Set(currentState.ids);
          operation.ids.forEach(id => {
            if (newIds.has(id)) {
              newIds.delete(id);
            } else {
              newIds.add(id);
            }
          });
          break;

        default:
          throw new Error(`Unknown selection mode: ${operation.mode}`);
      }

      logger.selection.change(`Applied ${operation.mode} operation. New count: ${newIds.size}`, {
        mode: operation.mode,
        finalCount: newIds.size,
        source: operation.source,
      });

      set({
        ids: newIds,
        source: operation.source || 'api',
        timestamp: Date.now(),
      });
    },

    // Multi-select helpers for Timeline interactions
    selectRange: (startId: number, endId: number, source: string = 'timeline') => {
      const ids: number[] = [];
      for (let i = Math.min(startId, endId); i <= Math.max(startId, endId); i++) {
        ids.push(i);
      }

      logger.selection.action(
        `Select range ${startId}-${endId} (${ids.length} items) from ${source}`,
        {
          startId,
          endId,
          count: ids.length,
          source,
        }
      );

      update(state => ({
        ids: new Set([...state.ids, ...ids]),
        source,
        timestamp: Date.now(),
      }));
    },

    // Handle multi-select modes (Ctrl, Shift, etc.)
    handleClick: (
      segmentIndex: number,
      options: {
        isMultiSelect?: boolean;
        isShiftSelect?: boolean;
        lastSelectedIndex?: number | null;
        source?: string;
      } = {}
    ) => {
      const {
        isMultiSelect = false,
        isShiftSelect = false,
        lastSelectedIndex = null,
        source = 'timeline',
      } = options;

      if (isShiftSelect && lastSelectedIndex !== null) {
        // Shift-select: add range to selection
        const start = Math.min(lastSelectedIndex, segmentIndex);
        const end = Math.max(lastSelectedIndex, segmentIndex);
        const rangeIds: number[] = [];
        for (let i = start; i <= end; i++) {
          rangeIds.push(i);
        }

        logger.selection.action(
          `Shift-click range selection ${start}-${end} (${rangeIds.length} items) from ${source}`,
          {
            start,
            end,
            count: rangeIds.length,
            source,
            type: 'shift-select',
          }
        );

        update(state => ({
          ids: new Set([...state.ids, ...rangeIds]),
          source,
          timestamp: Date.now(),
        }));
      } else if (isMultiSelect) {
        // Ctrl-select: toggle single item
        update(state => {
          const newIds = new Set(state.ids);
          const wasSelected = newIds.has(segmentIndex);
          if (wasSelected) {
            newIds.delete(segmentIndex);
          } else {
            newIds.add(segmentIndex);
          }

          logger.selection.action(
            `Ctrl-click segment ${segmentIndex} (${wasSelected ? 'removed' : 'added'}) from ${source}`,
            {
              segmentIndex,
              source,
              type: 'ctrl-select',
              action: wasSelected ? 'removed' : 'added',
            }
          );

          return {
            ids: newIds,
            source,
            timestamp: Date.now(),
          };
        });
      } else {
        // Single select: replace selection
        logger.selection.action(`Single-click segment ${segmentIndex} from ${source}`, {
          segmentIndex,
          source,
          type: 'single-select',
        });

        set({
          ids: new Set([segmentIndex]),
          source,
          timestamp: Date.now(),
        });
      }
    },
  };
};

// Create preview selection store
const createPreviewStore = () => {
  const { subscribe, set, update } = writable<PreviewState>({
    ids: new Set<number>(),
    source: null,
    timestamp: Date.now(),
    active: false,
  });

  return {
    subscribe,

    // Set preview selection (for hover events)
    setPreview: (ids: Iterable<number>, source: string = 'groups') => {
      const newIds = new Set(ids);
      logger.selection.action(`Preview ${newIds.size} items from ${source}`, {
        ids: Array.from(newIds),
        source,
        type: 'preview',
      });
      set({
        ids: newIds,
        source,
        timestamp: Date.now(),
        active: true,
      });
    },

    // Clear preview selection (for hover leave events)
    clearPreview: (source: string = 'groups') => {
      logger.selection.action(`Clear preview from ${source}`, {
        source,
        type: 'preview-clear',
      });
      set({
        ids: new Set<number>(),
        source,
        timestamp: Date.now(),
        active: false,
      });
    },

    // Update preview with new IDs without full reset
    updatePreview: (ids: Iterable<number>, source: string = 'groups') => {
      const newIds = new Set(ids);
      update(state => ({
        ...state,
        ids: newIds,
        source,
        timestamp: Date.now(),
        active: newIds.size > 0,
      }));
    },
  };
};

// Export the services
export const selectionService = createSelectionStore();
export const previewService = createPreviewStore();

// Derived stores for convenience
export const selectedIds = derived(selectionService, $selection => $selection.ids);
export const selectedCount = derived(selectedIds, $ids => $ids.size);
export const selectionSource = derived(selectionService, $selection => $selection.source);

// Preview derived stores
export const previewIds = derived(previewService, $preview => $preview.ids);
export const previewCount = derived(previewIds, $ids => $ids.size);
export const previewActive = derived(previewService, $preview => $preview.active);
export const previewSource = derived(previewService, $preview => $preview.source);

// Combined derived store for UI convenience (actual selection + preview overlay)
export const visualIds = derived(
  [selectedIds, previewIds, previewActive],
  ([$selectedIds, $previewIds, $previewActive]) => {
    if ($previewActive && $previewIds.size > 0) {
      // When preview is active, show preview instead of selection
      return $previewIds;
    }
    return $selectedIds;
  }
);

// Combined display data for debug UI
export const selectionDisplayData = derived(
  [
    selectedIds,
    selectedCount,
    selectionSource,
    previewIds,
    previewCount,
    previewActive,
    previewSource,
  ],
  ([
    $selectedIds,
    $selectedCount,
    $selectionSource,
    $previewIds,
    $previewCount,
    $previewActive,
    $previewSource,
  ]) => ({
    selection: {
      count: $selectedCount,
      ids: Array.from($selectedIds),
      source: $selectionSource,
    },
    preview: {
      count: $previewCount,
      ids: Array.from($previewIds),
      active: $previewActive,
      source: $previewSource,
    },
  })
);

// Helper functions
export const isSelected = (id: number) => {
  return get(selectedIds).has(id);
};

export const isInPreview = (id: number) => {
  return get(previewIds).has(id);
};

export const isVisuallySelected = (id: number) => {
  return get(visualIds).has(id);
};

export const getSelectedArray = () => {
  return Array.from(get(selectedIds));
};

export const getPreviewArray = () => {
  return Array.from(get(previewIds));
};

export const getVisualArray = () => {
  return Array.from(get(visualIds));
};
