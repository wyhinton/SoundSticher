import type { D3TimelineManager, TimelineItem } from './D3TimelineManager';
import { invokeWithPerf, updateInputs } from '../../state/performance';
import { generateProgressChannel, type SortAudioEvent } from '../../state/events';
import { Channel } from '@tauri-apps/api/core';
import { applySyncIndexes, type AppState } from '../../state/state.svelte';
import { get, type Writable } from 'svelte/store';
import { writable, type Readable } from 'svelte/store';

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
  private isDev: boolean;
  private selectedSegments: Set<number> = new Set();

  constructor(appStateStore: Writable<AppState>) {
    this.appStateStore = appStateStore;
    // Check if we're in development mode
    this.isDev =
      typeof import.meta !== 'undefined' &&
      typeof (import.meta as any).env !== 'undefined' &&
      (import.meta as any).env.DEV === true;
  }

  /**
   * Update the selected segments that should be moved together during drag operations
   */
  setSelectedSegments(selectedSegments: Set<number>): void {
    this.selectedSegments = new Set(selectedSegments);
  }

  private setState(next: DragDropState) {
    this._state = next;
    this._stateStore.set(next);
  }

  getState(): DragDropState {
    return this._state;
  }

  /**
   * Log to console only in development mode with styling
   */
  private log(message: string, ...args: any[]): void {
    if (this.isDev) {
      console.log(
        `%c🎯 DragDrop %c${message}`,
        'background: #4CAF50; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
        'color: #4CAF50; font-weight: normal;',
        ...args
      );
    }
  }

  /**
   * Log errors to console with styling (always logged)
   */
  private logError(message: string, ...args: any[]): void {
    console.error(
      `%c❌ DragDrop Error %c${message}`,
      'background: #f44336; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
      'color: #f44336; font-weight: normal;',
      ...args
    );
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
    this.log(`Started dragging segment ${event.index}`);

    // Determine which segments to move
    let segmentsToMove: number[];
    if (this.selectedSegments.size > 1 && this.selectedSegments.has(event.index)) {
      // Move all selected segments as a group
      segmentsToMove = Array.from(this.selectedSegments).sort((a, b) => a - b);
      this.log(`Will move ${segmentsToMove.length} selected segments:`, segmentsToMove);
    } else {
      // Move only the dragged segment
      segmentsToMove = [event.index];
      this.log(`Will move single segment: ${event.index}`);
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
      this.log(
        `Drag move skipped - isDragging: ${currentState.isDragging}, d3Manager: ${!!this.d3Manager}, container: ${!!this.container}`
      );
      return;
    }

    const appState = get(this.appStateStore);
    if (!appState?.timelineItems) {
      this.log('Drag move skipped - no timeline items in app state');
      return;
    }

    // Calculate drop position using the D3 manager
    const containerRect = this.container.getBoundingClientRect();
    const relativeX = event.mousePos.x - containerRect.left;

    this.log(
      `Drag move - segment ${event.index}, mouseX: ${event.mousePos.x}, relativeX: ${relativeX}, dragDistance: ${event.dragDistance}`
    );

    const dropPosition = this.d3Manager.calculateDropPosition(
      relativeX,
      appState.timelineItems as TimelineItem[]
    );

    this.log(`Drop position calculated - index: ${dropPosition.index}, x: ${dropPosition.x}`);

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

    this.log(
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
    this.log(`Reordering segment ${sourceIndex} to position ${targetIndex}`);

    // Create a copy of the timeline items array
    const items = [...appState.timelineItems];

    // Use the segments to move from the state
    const segmentsToMove = this._state.segmentsToMove;
    this.log(`Moving ${segmentsToMove.length} segments:`, segmentsToMove);

    // Validate all segments exist
    for (const index of segmentsToMove) {
      if (!items[index]) {
        this.logError('No item found at source index', index);
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

    this.log('Reorder updates:', updates);

    try {
      // Create progress channel for the reorder operation
      const onEvent = generateProgressChannel<SortAudioEvent>(Channel, {
        started: () => {
          this.log('Reorder started');
        },
        progress: data => {
          this.log('Reorder progress:', data);
        },
        finished: () => {
          this.log('Reorder finished');
        },
      });

      // Call backend update_sorting function
      const newOrder = await invokeWithPerf<[string, number][]>('update_sorting', {
        updates,
        onEvent,
      });

      this.log('Received new order from backend:', newOrder);

      // Update inputs after state change
      updateInputs(appState.sections);

      // Use the reusable index syncing function
      if (newOrder.ok && newOrder.value) {
        applySyncIndexes(newOrder.value);
      }

      this.log('Reorder completed successfully');
    } catch (error) {
      this.logError('Failed to reorder timeline items:', error);
      throw error;
    }
  }

  /**
   * Reset the drag state
   */
  private resetDragState(): void {
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
