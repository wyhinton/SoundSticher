import type { D3TimelineManager, TimelineItem } from './D3TimelineManager';
import { invokeWithPerf, updateInputs } from '../../state/performance';
import { generateProgressChannel, type SortAudioEvent } from '../../state/events';
import { Channel } from '@tauri-apps/api/core';
import { type AppState } from '../../state/state.svelte';
import { get, type Writable } from 'svelte/store';
import { writable, type Readable } from 'svelte/store';
import { logger } from '../../state/logging';

export type DragDropState = {
  isDragging: boolean;
  draggedSegmentIndex: number;
  dropIndicatorIndex: number;
  dropIndicatorX: number;
  segmentsToMove: number[];
};

export const DEFAULT_DD: DragDropState = {
  isDragging: false,
  draggedSegmentIndex: -1,
  dropIndicatorIndex: -1,
  dropIndicatorX: 0,
  segmentsToMove: [],
};

export interface DragStartEvent {
  index: number;
  startPos: { x: number; y: number };
  segmentId: number;
}

export interface DragMoveEvent {
  index: number;
  mousePos: { x: number; y: number };
  dragDistance: number;
  event: d3.D3DragEvent<SVGGElement, unknown, d3.SubjectPosition>;
}

export interface DragEndEvent {
  index: number;
  endPos: { x: number; y: number };
  dragDistance: number;
  event: d3.D3DragEvent<SVGGElement, unknown, d3.SubjectPosition>;
}

export class DragDropManager {
  private _state: DragDropState = DEFAULT_DD;

  private _stateStore = writable<DragDropState>(this._state);
  public readonly state: Readable<DragDropState> = {
    subscribe: this._stateStore.subscribe,
  };

  private d3Manager: D3TimelineManager | null = null;
  private container: HTMLElement | null = null;
  private appStateStore: Writable<AppState>;
  private selectedSegments: Set<number> = new Set();
  /** Pre-computed segments to move (for group drag operations) */
  private precomputedSegmentsToMove: Set<number> | null = null;

  constructor(appStateStore: Writable<AppState>) {
    this.appStateStore = appStateStore;
  }

  /**
   * Update the selected segments that should be moved together during drag operations
   */
  setSelectedSegments(selectedSegments: Set<number>): void {
    this.selectedSegments = new Set(selectedSegments);
  }

  /**
   * Set pre-computed segments to move (for MergeOp group drag)
   * Call this before handleDragStart when dragging a group
   */
  setSegmentsToMove(segments: Set<number>): void {
    this.precomputedSegmentsToMove = new Set(segments);
    logger.dragdrop.info(`Pre-computed ${segments.size} segments for group drag`);
  }

  private setState(next: DragDropState) {
    this._state = next;
    this._stateStore.set(next);
  }

  getState(): DragDropState {
    return this._state;
  }

  /**
   * Initialize the drag drop manager with required dependencies
   */
  initialize(d3Manager: D3TimelineManager, container: HTMLElement) {
    this.d3Manager = d3Manager;
    this.container = container;
  }

  /**
   * Handle drag start event
   */
  handleDragStart(event: DragStartEvent): void {
    logger.dragdrop.start(`Started dragging segment ${event.index}`);

    // Determine which segments to move
    let segmentsToMove: number[];

    // Check if we have pre-computed segments for group drag (MergeOp)
    if (this.precomputedSegmentsToMove && this.precomputedSegmentsToMove.size > 0) {
      segmentsToMove = Array.from(this.precomputedSegmentsToMove).sort((a, b) => a - b);
      logger.dragdrop.info(
        `Using pre-computed group drag: ${segmentsToMove.length} segments:`,
        segmentsToMove
      );
      // Clear the pre-computed segments after use
      this.precomputedSegmentsToMove = null;
    } else if (this.selectedSegments.size > 1 && this.selectedSegments.has(event.index)) {
      // Move all selected segments as a group
      segmentsToMove = Array.from(this.selectedSegments).sort((a, b) => a - b);
      logger.dragdrop.info(`Will move ${segmentsToMove.length} selected segments:`, segmentsToMove);
    } else {
      // Move only the dragged segment
      segmentsToMove = [event.index];
      logger.dragdrop.info(`Will move single segment: ${event.index}`);
    }

    this.setState({
      ...this._state,
      isDragging: true,
      draggedSegmentIndex: event.index,
      dropIndicatorIndex: -1,
      dropIndicatorX: 0,
      segmentsToMove,
    });
  }

  /**
   * Handle drag move event
   */
  handleDragMove(event: DragMoveEvent): void {
    const currentState = this.getState();
    if (!currentState.isDragging || !this.d3Manager || !this.container) {
      logger.dragdrop.info(
        `Drag move skipped - isDragging: ${currentState.isDragging}, d3Manager: ${!!this.d3Manager}, container: ${!!this.container}`
      );
      return;
    }

    const appState = get(this.appStateStore);
    if (!appState?.timelineItems) {
      logger.dragdrop.info('Drag move skipped - no timeline items in app state');
      return;
    }

    // Calculate drop position using the D3 manager
    const containerRect = this.container.getBoundingClientRect();
    const relativeX = event.mousePos.x - containerRect.left;

    logger.dragdrop.move(
      `Drag move - segment ${event.index}, mouseX: ${event.mousePos.x}, relativeX: ${relativeX}, dragDistance: ${event.dragDistance}`
    );

    const dropPosition = this.d3Manager.calculateDropPosition(
      relativeX,
      appState.timelineItems as TimelineItem[]
    );

    logger.dragdrop.move(
      `Drop position calculated - index: ${dropPosition.index}, x: ${dropPosition.x}`
    );

    this.setState({
      ...this._state,
      dropIndicatorIndex: dropPosition.index,
      dropIndicatorX: dropPosition.x,
    });
  }

  /**
   * Handle drag end event
   */
  async handleDragEnd(event: DragEndEvent): Promise<void> {
    const currentState = this.getState();
    if (!currentState.isDragging) return;

    logger.dragdrop.end(
      `Ended dragging segment ${event.index} to position ${currentState.dropIndicatorIndex}`
    );

    const appState = get(this.appStateStore);

    // Perform the reorder if we have a valid drop position
    if (
      currentState.dropIndicatorIndex >= 0 &&
      currentState.dropIndicatorIndex !== event.index &&
      currentState.dropIndicatorIndex !== event.index + 1 &&
      appState?.timelineItems
    ) {
      await this.performReorder(event.index, currentState.dropIndicatorIndex, appState);
    }

    // Reset drag state
    this.resetDragState();
  }

  /**
   * Perform the actual reorder operation
   */
  private async performReorder(
    sourceIndex: number,
    targetIndex: number,
    appState: AppState
  ): Promise<void> {
    logger.dragdrop.reorder(`Reordering segment ${sourceIndex} to position ${targetIndex}`);

    // Create a copy of the timeline items array
    const items = [...appState.timelineItems];

    // Use the segments to move from the state
    const segmentsToMove = this._state.segmentsToMove;
    logger.dragdrop.reorder(`Moving ${segmentsToMove.length} segments:`, segmentsToMove);

    // Validate all segments exist
    for (const index of segmentsToMove) {
      if (!items[index]) {
        logger.dragdrop.error('No item found at source index', index);
        return;
      }
    }

    // Extract the items to move
    const itemsToMove = segmentsToMove
      .map(index => items[index])
      .filter(item => item !== undefined);

    // Remove the segments from their current positions (in reverse order to maintain indices)
    const segmentsToRemove = [...segmentsToMove].sort((a, b) => b - a);
    for (const index of segmentsToRemove) {
      items.splice(index, 1);
    }

    // Calculate the insertion point
    // We need to adjust for the segments we've already removed
    let insertIndex = targetIndex;
    for (const removedIndex of segmentsToMove) {
      if (removedIndex < targetIndex) {
        insertIndex--;
      }
    }

    // Ensure insertIndex is valid
    insertIndex = Math.max(0, Math.min(insertIndex, items.length));

    // Insert all moved items at the new position
    items.splice(insertIndex, 0, ...itemsToMove);

    // Build array for Rust backend: { id, index }
    const updates = items.map((item, newIndex) => ({
      id: item.id,
      index: newIndex,
    }));

    logger.dragdrop.reorder('Reorder updates:', updates);

    try {
      // Create progress channel for the reorder operation
      const onEvent = generateProgressChannel<SortAudioEvent>(Channel, {
        started: () => {
          logger.dragdrop.reorder('Reorder started');
        },
        progress: data => {
          logger.dragdrop.reorder('Reorder progress:', data);
        },
        finished: () => {
          logger.dragdrop.reorder('Reorder finished');
        },
      });

      // Call backend update_sorting function
      const newOrder = await invokeWithPerf<[string, number][]>('update_sorting', {
        updates,
        onEvent,
      });

      logger.dragdrop.reorder('Received new order from backend:', newOrder);

      // Note: updateInputs call removed - operations should manage their own sections
      // updateInputs(appState.sections);

      // Use the reusable index syncing function
      if (newOrder.ok && newOrder.value) {
        // Note: applySyncIndexes removed - operations should handle their own index sync
        // applySyncIndexes(newOrder.value);
      }

      logger.dragdrop.reorder('Reorder completed successfully');
    } catch (error) {
      logger.dragdrop.error('Failed to reorder timeline items:', error);
      throw error;
    }
  }

  /**
   * Reset the drag state
   */
  private resetDragState(): void {
    this.precomputedSegmentsToMove = null;
    this.setState(DEFAULT_DD);
  }

  /**
   * Clean up resources
   */
  destroy(): void {
    this.resetDragState();
    this.d3Manager = null;
    this.container = null;
  }

  /**
   * Check if currently dragging
   */
  get isDragging(): boolean {
    return this._state.isDragging;
  }

  /**
   * Get the current dragged segment index
   */
  get draggedSegmentIndex(): number {
    return this._state.draggedSegmentIndex;
  }

  /**
   * Get the current drop indicator index
   */
  get dropIndicatorIndex(): number {
    return this._state.dropIndicatorIndex;
  }

  /**
   * Get the current drop indicator X position
   */
  get dropIndicatorX(): number {
    return this._state.dropIndicatorX;
  }
}
