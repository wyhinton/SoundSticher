import { get } from 'svelte/store';
import { logger } from '../logging';
import type { OperationId } from '../operation';
import { appState, type TimelineData } from '../state.svelte';
import {
  timelineViewStates,
  getTimelineViewState,
  type TimelineId,
  type TimelineViewState,
} from './timelines';

/**
 * Service layer for timeline management
 * Uses appState for timeline data instead of timelinesStore
 */
class TimelineStoreService {
  /**
   * Find timeline ID for a given operation
   */
  findTimelineByOperation(operationId: OperationId): TimelineId | null {
    const currentState = get(appState);
    const timelines = currentState.timelines?.timelines;
    if (!timelines) return null;

    const foundTimelineId = Object.keys(timelines).find(timelineId => {
      const timeline = timelines[timelineId];
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
  createTimelineForOperation(operationId: OperationId): TimelineData {
    logger.waveform.info(`Creating timeline for operation: ${operationId}`);

    const timelineId = `tl_op_${operationId}`;
    const newTimeline: TimelineData = {
      id: timelineId,
      source: { kind: 'operation', operationId },
      items: [],
    };

    appState.update(state => ({
      ...state,
      timelines: {
        timelines: {
          ...(state.timelines?.timelines ?? {}),
          [timelineId]: newTimeline,
        },
        activeTimelineId: state.timelines?.activeTimelineId ?? timelineId,
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
  ): TimelineData {
    logger.waveform.info(`Creating timeline with ID ${timelineId} for operation: ${operationId}`);

    const newTimeline: TimelineData = {
      id: timelineId,
      source: { kind: 'operation', operationId },
      items: [],
    };

    appState.update(state => ({
      ...state,
      timelines: {
        timelines: {
          ...(state.timelines?.timelines ?? {}),
          [timelineId]: newTimeline,
        },
        activeTimelineId: state.timelines?.activeTimelineId ?? timelineId,
      },
    }));

    // Restore view state if provided
    if (viewState) {
      timelineViewStates.update(states => ({
        ...states,
        [timelineId]: viewState,
      }));
    }

    return newTimeline;
  }

  /**
   * Delete a timeline by ID
   */
  deleteTimeline(timelineId: TimelineId): void {
    logger.waveform.info(`Deleting timeline: ${timelineId}`);

    appState.update(state => {
      const newTimelines = { ...(state.timelines?.timelines ?? {}) };
      delete newTimelines[timelineId];

      return {
        ...state,
        timelines: {
          timelines: newTimelines,
          activeTimelineId:
            state.timelines?.activeTimelineId === timelineId
              ? (Object.keys(newTimelines)[0] ?? null)
              : (state.timelines?.activeTimelineId ?? null),
        },
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
  getTimeline(timelineId: TimelineId): TimelineData | null {
    const currentState = get(appState);
    return currentState.timelines?.timelines[timelineId] || null;
  }

  /**
   * Get timeline for a specific operation
   */
  getTimelineForOperation(operationId: OperationId): TimelineData | null {
    const timelineId = this.findTimelineByOperation(operationId);
    return timelineId ? this.getTimeline(timelineId) : null;
  }

  /**
   * Get all timelines
   */
  getAllTimelines(): TimelineData[] {
    const currentState = get(appState);
    const timelines = currentState.timelines?.timelines ?? {};
    return Object.values(timelines).filter(
      (timeline): timeline is TimelineData => timeline != null
    );
  }

  /**
   * Get all operation-based timelines
   */
  getOperationTimelines(): TimelineData[] {
    return this.getAllTimelines().filter(timeline => timeline.source.kind === 'operation');
  }

  /**
   * Set active timeline
   */
  setActiveTimeline(timelineId: TimelineId | null): void {
    appState.update(state => ({
      ...state,
      timelines: {
        ...(state.timelines ?? { timelines: {}, activeTimelineId: null }),
        activeTimelineId: timelineId,
      },
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
    const currentState = get(appState);
    return currentState.timelines?.activeTimelineId ?? null;
  }

  /**
   * Get active timeline
   */
  getActiveTimeline(): TimelineData | null {
    const currentState = get(appState);
    const activeId = currentState.timelines?.activeTimelineId;
    return activeId ? currentState.timelines?.timelines[activeId] || null : null;
  }

  /**
   * Capture timeline state for undo operations
   */
  captureTimelineState(timelineId: TimelineId): {
    timeline: TimelineData;
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
    timeline?: TimelineData;
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
  ): TimelineData {
    return this.createTimelineWithId(timelineId, operationId, viewState);
  }
}

/**
 * Singleton timeline service instance
 */
export const timelineService = new TimelineStoreService();
