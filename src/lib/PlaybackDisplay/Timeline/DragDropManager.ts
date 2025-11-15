import type { D3TimelineManager, TimelineItem } from './D3TimelineManager';
import { invokeWithPerf, updateInputs } from '../../state/performance';
import { generateProgressChannel, type SortAudioEvent } from '../../state/events';
import { Channel } from '@tauri-apps/api/core';
import { applySyncIndexes, type AppState } from '../../state/state.svelte';
import { get, type Writable } from 'svelte/store';

export interface DragDropState {
  isDragging: boolean;
  draggedSegmentIndex: number;
  dropIndicatorIndex: number;
  dropIndicatorX: number;
}

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
  private state: DragDropState;
  private d3Manager: D3TimelineManager | null = null;
  private container: HTMLElement | null = null;
  private appStateStore: Writable<AppState>;
  private isDev: boolean;

  constructor(appStateStore: Writable<AppState>) {
    this.appStateStore = appStateStore;
    // Check if we're in development mode
    this.isDev = typeof import.meta !== 'undefined' && 
                 typeof (import.meta as any).env !== 'undefined' && 
                 (import.meta as any).env.DEV === true;
    this.state = {
      isDragging: false,
      draggedSegmentIndex: -1,
      dropIndicatorIndex: -1,
      dropIndicatorX: 0,
    };
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
   * Get the current drag drop state
   */
  getState(): Readonly<DragDropState> {
    return { ...this.state };
  }

  /**
   * Handle drag start event
   */
  handleDragStart(event: DragStartEvent): void {
    this.log(`Started dragging segment ${event.index}`);
    
    this.state.isDragging = true;
    this.state.draggedSegmentIndex = event.index;
    this.state.dropIndicatorIndex = -1;
    this.state.dropIndicatorX = 0;
  }

  /**
   * Handle drag move event
   */
  handleDragMove(event: DragMoveEvent): void {
    if (!this.state.isDragging || !this.d3Manager || !this.container) return;

    const appState = get(this.appStateStore);
    if (!appState?.timelineItems) return;

    // Calculate drop position using the D3 manager
    const containerRect = this.container.getBoundingClientRect();
    const relativeX = event.mousePos.x - containerRect.left;

    const dropPosition = this.d3Manager.calculateDropPosition(
      relativeX,
      appState.timelineItems as TimelineItem[]
    );

    this.state.dropIndicatorIndex = dropPosition.index;
    this.state.dropIndicatorX = dropPosition.x;
  }

  /**
   * Handle drag end event
   */
  async handleDragEnd(event: DragEndEvent): Promise<void> {
    if (!this.state.isDragging) return;

    this.log(`Ended dragging segment ${event.index} to position ${this.state.dropIndicatorIndex}`);

    const appState = get(this.appStateStore);
    
    // Perform the reorder if we have a valid drop position
    if (
      this.state.dropIndicatorIndex >= 0 &&
      this.state.dropIndicatorIndex !== event.index &&
      this.state.dropIndicatorIndex !== event.index + 1 &&
      appState?.timelineItems
    ) {
      await this.performReorder(event.index, this.state.dropIndicatorIndex, appState);
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
        progress: (data) => {
          this.log('Reorder progress:', data);
        },
        finished: () => {
          this.log('Reorder finished');
        },
      });

      // Call backend update_sorting function
      const newOrder = await invokeWithPerf<[string, number][]>('update_sorting', { updates, onEvent });
      
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
    this.state.isDragging = false;
    this.state.draggedSegmentIndex = -1;
    this.state.dropIndicatorIndex = -1;
    this.state.dropIndicatorX = 0;
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
    return this.state.isDragging;
  }

  /**
   * Get the current dragged segment index
   */
  get draggedSegmentIndex(): number {
    return this.state.draggedSegmentIndex;
  }

  /**
   * Get the current drop indicator index
   */
  get dropIndicatorIndex(): number {
    return this.state.dropIndicatorIndex;
  }

  /**
   * Get the current drop indicator X position
   */
  get dropIndicatorX(): number {
    return this.state.dropIndicatorX;
  }
}
