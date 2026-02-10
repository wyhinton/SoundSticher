import { get } from 'svelte/store';
import type { OperationId } from '../operation';
import { appState, type TimelineData } from '../state.svelte';
import { TimelineViewer } from './TimelineViewer';

/**
 * Timeline operations for the new AppState-based system
 */
export class TimelineOperations {
  /**
   * Toggle timeline visibility for a specific operation ID
   * Sets operation.visible property and manages timeline data in appState
   */
  static toggleTimelineVisibilityByOpId(operationId: OperationId): void {
    console.info(`Toggling timeline visibility for operation: ${operationId}`);

    appState.update(state => {
      if (!state.operations?.defs[operationId]) {
        console.warn(`Operation ${operationId} not found`);
        return state;
      }

      const operation = state.operations.defs[operationId];
      const isCurrentlyVisible = operation.visible || false;
      const timelineId = `tl_op_${operationId}`;

      // Toggle the visibility on the operation
      const newDefs = { ...state.operations.defs };
      const existingOp = newDefs[operationId];
      if (existingOp) {
        newDefs[operationId] = { ...existingOp, visible: !isCurrentlyVisible } as typeof existingOp;
      }

      const existingTimelines = state.timelines?.timelines ?? {};

      if (!isCurrentlyVisible) {
        // Operation is becoming visible - create serializable timeline data
        const newTimeline: TimelineData = {
          id: timelineId,
          source: { kind: 'operation', operationId },
          items: [],
        };

        const newActiveId = state.timelines?.activeTimelineId ?? timelineId;

        console.info(`Showing timeline for operation: ${operationId}`);

        return {
          ...state,
          operations: { ...state.operations, defs: newDefs },
          timelines: {
            timelines: { ...existingTimelines, [timelineId]: newTimeline },
            activeTimelineId: newActiveId,
          },
        };
      } else {
        // Operation is becoming hidden - remove timeline data
        const { [timelineId]: _removed, ...remainingTimelines } = existingTimelines;

        const newActiveId =
          state.timelines?.activeTimelineId === timelineId
            ? (Object.keys(remainingTimelines)[0] ?? null)
            : (state.timelines?.activeTimelineId ?? null);

        console.info(`Hiding timeline for operation: ${operationId}`);

        return {
          ...state,
          operations: { ...state.operations, defs: newDefs },
          timelines: {
            timelines: remainingTimelines,
            activeTimelineId: newActiveId,
          },
        };
      }
    });
  }

  /**
   * Check if an operation's timeline is visible
   */
  static isOperationTimelineVisible(operationId: OperationId): boolean {
    const state = get(appState);
    const operation = state.operations?.defs[operationId];
    return operation?.visible || false;
  }

  /**
   * Get all visible operation IDs
   */
  static getVisibleOperationIds(): OperationId[] {
    const state = get(appState);
    if (!state.operations?.defs) return [];

    return Object.keys(state.operations.defs).filter(
      opId => state.operations?.defs[opId]?.visible || false
    );
  }

  /**
   * Create a TimelineViewer for a given operation
   */
  static createTimelineViewer(operationId: OperationId): TimelineViewer {
    return new TimelineViewer(operationId);
  }

  /**
   * Set the active timeline
   */
  static setActiveTimeline(timelineId: string | null): void {
    appState.update(state => ({
      ...state,
      timelines: {
        ...(state.timelines ?? { timelines: {}, activeTimelineId: null }),
        activeTimelineId: timelineId,
      },
    }));
  }

  /**
   * Get the active timeline ID
   */
  static getActiveTimelineId(): string | null {
    const state = get(appState);
    return state.timelines?.activeTimelineId || null;
  }
}
