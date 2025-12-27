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
};

export const DEFAULT_DD: DragDropState = {
  isDragging: false,
  draggedSegmentIndex: -1,
  dropIndicatorIndex: -1,
  dropIndicatorX: 0,
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

  constructor(appStateStore: Writable<AppState>) {
    this.appStateStore = appStateStore;
    // Check if we're in development mode
    this.isDev =
      typeof import.meta !== 'undefined' &&
      typeof (import.meta as any).env !== 'undefined' &&
      (import.meta as any).env.DEV === true;
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

    this.setState({
      ...this._state,
      isDragging: true,
      draggedSegmentIndex: event.index,
      dropIndicatorIndex: -1,
      dropIndicatorX: 0,
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

    // Create a copy of the timeline items array and perform the reorder
    const items = [...appState.timelineItems];
    const draggedItem = items[sourceIndex];

    if (!draggedItem) {
      this.logError('No item found at source index', sourceIndex);
      return;
    }

    // Remove the dragged item from its current position
    items.splice(sourceIndex, 1);

    // Insert it at the new position (adjust index if moving forward)
    const insertIndex = targetIndex > sourceIndex ? targetIndex - 1 : targetIndex;
    items.splice(insertIndex, 0, draggedItem);

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
