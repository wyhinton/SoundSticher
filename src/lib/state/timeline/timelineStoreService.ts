import { get } from 'svelte/store';
import { logger } from '../logging';
import type { OperationId } from '../operation';
import {
  timelinesStore,
  timelineViewStates,
  createTimelineStateForOp,
  getTimelineViewState,
  type Timeline,
  type TimelineId,
  type TimelineViewState,
  type TimelinesState,
} from './timelines';

/**
 * Service layer for timeline management
 * Provides clean API methods for timeline operations
 */
class TimelineStoreService {
  /**
   * Find timeline ID for a given operation
   */
  findTimelineByOperation(operationId: OperationId): TimelineId | null {
    const currentState = get(timelinesStore);

    const foundTimelineId = Object.keys(currentState.timelines).find(timelineId => {
      const timeline = currentState.timelines[timelineId];
      return (
        timeline &&
        timeline.source.kind === 'operation' &&
        timeline.source.operationId === operationId
      );
    });

    return foundTimelineId || null;
  }

  /**
   * Check if an operation has a visible timeline
   */
  isOperationTimelineVisible(operationId: OperationId): boolean {
    return this.findTimelineByOperation(operationId) !== null;
  }

  /**
   * Create a new timeline for an operation
   */
  createTimelineForOperation(operationId: OperationId): Timeline {
    logger.waveform.info(`Creating timeline for operation: ${operationId}`);

    const newTimeline = createTimelineStateForOp(operationId);

    timelinesStore.update(state => ({
      ...state,
      timelines: {
        ...state.timelines,
        [newTimeline.id]: newTimeline,
      },
    }));

    return newTimeline;
  }

  /**
   * Create a timeline with a specific ID (used for restoration from undo data)
   */
  createTimelineWithId(
    timelineId: TimelineId,
    operationId: OperationId,
    viewState?: TimelineViewState
  ): Timeline {
    logger.waveform.info(`Creating timeline with ID ${timelineId} for operation: ${operationId}`);

    const newTimeline = createTimelineStateForOp(operationId);

    // Update the timeline with the specified ID
    const restoredTimeline: Timeline = {
      ...newTimeline,
      id: timelineId,
    };

    timelinesStore.update(state => ({
      ...state,
      timelines: {
        ...state.timelines,
        [timelineId]: restoredTimeline,
      },
    }));

    // Restore view state if provided
    if (viewState) {
      timelineViewStates.update(states => ({
        ...states,
        [timelineId]: viewState,
      }));
    }

    return restoredTimeline;
  }

  /**
   * Delete a timeline by ID
   */
  deleteTimeline(timelineId: TimelineId): void {
    logger.waveform.info(`Deleting timeline: ${timelineId}`);

    // Remove timeline from store
    timelinesStore.update(state => {
      const newTimelines = { ...state.timelines };
      delete newTimelines[timelineId];

      return {
        ...state,
        timelines: newTimelines,
        // Clear active timeline if it's the one being deleted
        activeTimelineId: state.activeTimelineId === timelineId ? null : state.activeTimelineId,
      };
    });

    // Clean up view state
    timelineViewStates.update(states => {
      const newStates = { ...states };
      delete newStates[timelineId];
      return newStates;
    });
  }

  /**
   * Delete timeline for a specific operation
   */
  deleteTimelineForOperation(operationId: OperationId): boolean {
    const timelineId = this.findTimelineByOperation(operationId);

    if (timelineId) {
      this.deleteTimeline(timelineId);
      return true;
    }

    return false;
  }

  /**
   * Get timeline by ID
   */
  getTimeline(timelineId: TimelineId): Timeline | null {
    const currentState = get(timelinesStore);
    return currentState.timelines[timelineId] || null;
  }

  /**
   * Get timeline for a specific operation
   */
  getTimelineForOperation(operationId: OperationId): Timeline | null {
    const timelineId = this.findTimelineByOperation(operationId);
    return timelineId ? this.getTimeline(timelineId) : null;
  }

  /**
   * Get all timelines
   */
  getAllTimelines(): Timeline[] {
    const currentState = get(timelinesStore);
    return Object.values(currentState.timelines).filter(timeline => timeline !== undefined);
  }

  /**
   * Get all operation-based timelines
   */
  getOperationTimelines(): Timeline[] {
    return this.getAllTimelines().filter(timeline => timeline.source.kind === 'operation');
  }

  /**
   * Set active timeline
   */
  setActiveTimeline(timelineId: TimelineId | null): void {
    timelinesStore.update(state => ({
      ...state,
      activeTimelineId: timelineId,
    }));

    if (timelineId) {
      logger.timeline?.info(`Set active timeline: ${timelineId}`);
    } else {
      logger.timeline?.info('Cleared active timeline');
    }
  }

  /**
   * Get active timeline ID
   */
  getActiveTimelineId(): TimelineId | null {
    return get(timelinesStore).activeTimelineId;
  }

  /**
   * Get active timeline
   */
  getActiveTimeline(): Timeline | null {
    const state = get(timelinesStore);
    const activeId = state.activeTimelineId;
    return activeId ? state.timelines[activeId] || null : null;
  }

  /**
   * Capture timeline state for undo operations
   */
  captureTimelineState(timelineId: TimelineId): {
    timeline: Timeline;
    viewState: TimelineViewState;
  } | null {
    const timeline = this.getTimeline(timelineId);
    if (!timeline) return null;

    const viewState = getTimelineViewState(timelineId);

    return {
      timeline,
      viewState,
    };
  }

  /**
   * Toggle timeline visibility for an operation (direct store manipulation)
   * This is used by the undo system's applyCommand function
   */
  toggleTimelineVisibility(operationId: OperationId): {
    wasVisible: boolean;
    timelineId?: TimelineId;
    timeline?: Timeline;
    viewState?: TimelineViewState;
  } {
    const existingTimelineId = this.findTimelineByOperation(operationId);

    if (existingTimelineId) {
      // Timeline exists - capture state and remove it
      const capturedState = this.captureTimelineState(existingTimelineId);
      this.deleteTimeline(existingTimelineId);

      return {
        wasVisible: true,
        timelineId: existingTimelineId,
        timeline: capturedState?.timeline,
        viewState: capturedState?.viewState,
      };
    } else {
      // No timeline exists - create one
      const newTimeline = this.createTimelineForOperation(operationId);

      return {
        wasVisible: false,
        timelineId: newTimeline.id,
        timeline: newTimeline,
      };
    }
  }

  /**
   * Restore timeline from captured state (used by undo system)
   */
  restoreTimeline(
    timelineId: TimelineId,
    operationId: OperationId,
    viewState?: TimelineViewState
  ): Timeline {
    return this.createTimelineWithId(timelineId, operationId, viewState);
  }
}

/**
 * Singleton timeline service instance
 */
export const timelineService = new TimelineStoreService();
