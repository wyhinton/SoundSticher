/**
 * Timeline Selection Service - Per-timeline selection management
 *
 * This service provides selection functionality scoped to individual timelines,
 * allowing multiple timelines to have independent selections.
 */

import { derived, get, type Readable } from 'svelte/store';
import { selectionStore, type TimelineId, type TimelineSelection } from './timelines';

export interface TimelineSelectionService {
  // Derived stores for reactive access
  selectedIds: Readable<Set<number>>;
  previewIds: Readable<Set<number>>;
  previewActive: Readable<boolean>;
  lastSelectedIndex: Readable<number | null>;
  
  // Full selection state
  selection: Readable<TimelineSelection>;
  
  // Actions
  handleClick: (
    index: number,
    options?: {
      isMultiSelect?: boolean;
      isShiftSelect?: boolean;
      source?: string;
    }
  ) => void;
  
  select: (index: number) => void;
  toggle: (index: number) => void;
  clear: () => void;
  setSelected: (ids: Set<number>) => void;
  addToSelection: (index: number) => void;
  removeFromSelection: (index: number) => void;
  
  setPreview: (ids: Set<number>, active?: boolean) => void;
  clearPreview: () => void;
  
  getSelectedIds: () => Set<number>;
  isSelected: (index: number) => boolean;
}

/**
 * Create a selection service scoped to a specific timeline
 */
export function createTimelineSelectionService(
  timelineId: TimelineId,
  getItemCount: () => number
): TimelineSelectionService {
  // Initialize selection for this timeline
  selectionStore.init(timelineId);

  // Create derived stores
  const selection = selectionStore.forTimeline(timelineId);
  
  const selectedIds = derived(selection, ($s) => $s.selectedIds);
  const previewIds = derived(selection, ($s) => $s.previewIds);
  const previewActive = derived(selection, ($s) => $s.previewActive);
  const lastSelectedIndex = derived(selection, ($s) => $s.lastSelectedIndex);

  return {
    selection,
    selectedIds,
    previewIds,
    previewActive,
    lastSelectedIndex,

    handleClick(index, options = {}) {
      const { isMultiSelect = false, isShiftSelect = false } = options;
      
      selectionStore.handleClick(timelineId, index, {
        isMultiSelect,
        isShiftSelect,
        itemCount: getItemCount(),
      });
    },

    select(index) {
      selectionStore.setSelected(timelineId, new Set([index]));
      selectionStore.setLastSelected(timelineId, index);
    },

    toggle(index) {
      selectionStore.toggleSelection(timelineId, index);
      selectionStore.setLastSelected(timelineId, index);
    },

    clear() {
      selectionStore.clear(timelineId);
    },

    setSelected(ids) {
      selectionStore.setSelected(timelineId, ids);
    },

    addToSelection(index) {
      selectionStore.addToSelection(timelineId, index);
    },

    removeFromSelection(index) {
      selectionStore.removeFromSelection(timelineId, index);
    },

    setPreview(ids, active = true) {
      selectionStore.setPreview(timelineId, ids, active);
    },

    clearPreview() {
      selectionStore.clearPreview(timelineId);
    },

    getSelectedIds() {
      return selectionStore.get(timelineId).selectedIds;
    },

    isSelected(index) {
      return selectionStore.get(timelineId).selectedIds.has(index);
    },
  };
}

/**
 * Factory for creating timeline selection services
 */
export const timelineSelectionFactory = {
  services: new Map<TimelineId, TimelineSelectionService>(),

  /**
   * Get or create a selection service for a timeline
   */
  forTimeline(timelineId: TimelineId, getItemCount: () => number): TimelineSelectionService {
    if (!this.services.has(timelineId)) {
      this.services.set(timelineId, createTimelineSelectionService(timelineId, getItemCount));
    }
    return this.services.get(timelineId)!;
  },

  /**
   * Remove a selection service
   */
  remove(timelineId: TimelineId): void {
    this.services.delete(timelineId);
    selectionStore.remove(timelineId);
  },

  /**
   * Clear all selection services
   */
  clear(): void {
    this.services.clear();
  },
};
