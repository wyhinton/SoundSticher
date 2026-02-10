import { get } from 'svelte/store';
import { loggingState } from '../logging';
import { TimelinesState } from '../timeline/timelines';
import { Command } from './undo';

/**
 * Apply timeline-specific command effects to the timelines store
 * This handles commands that need to update timeline state alongside app state
 */
export function applyTimelineCommand(
  timelinesState: TimelinesState,
  command: Command
): TimelinesState {
  //   const isLogging = get(loggingState).operationsLog;
  const isLogging = true;

  // Only certain commands affect timeline state
  switch (command.type) {
    case 'toggle-timeline-visibility': {
      if (isLogging) {
        console.log(`🔄 Timeline: Applying ${command.type} to timelines store`);
      }
      // Timeline toggle logic would go here
      // For now, return unchanged state since the actual implementation is commented out
      return timelinesState;
    }

    case 'delete-operation': {
      if (isLogging) {
        console.log(
          `🔄 Timeline: Cleaning up timeline for deleted operation ${command.operationId}`
        );
      }
      // Remove any timelines associated with the deleted operation
      const updatedTimelines = { ...timelinesState.timelines };
      const timelineToRemove = Object.keys(updatedTimelines).find(timelineId => {
        const timeline = updatedTimelines[timelineId];
        return (
          timeline?.source.kind === 'operation' &&
          timeline.source.operationId === command.operationId
        );
      });

      if (timelineToRemove) {
        delete updatedTimelines[timelineToRemove];
        return {
          ...timelinesState,
          timelines: updatedTimelines,
          activeTimelineId:
            timelinesState.activeTimelineId === timelineToRemove
              ? null
              : timelinesState.activeTimelineId,
        };
      }
      return timelinesState;
    }

    case 'delete-multiple-operations': {
      if (isLogging) {
        console.log(
          `🔄 Timeline: Cleaning up timelines for deleted operations [${command.operationIds.join(', ')}]`
        );
      }
      // Remove timelines for all deleted operations
      const modifiedTimelines = { ...timelinesState.timelines };
      let newActiveTimelineId = timelinesState.activeTimelineId;

      for (const operationId of command.operationIds) {
        const timelineToRemove = Object.keys(modifiedTimelines).find(timelineId => {
          const timeline = modifiedTimelines[timelineId];
          return (
            timeline?.source.kind === 'operation' && timeline.source.operationId === operationId
          );
        });

        if (timelineToRemove) {
          delete modifiedTimelines[timelineToRemove];
          if (newActiveTimelineId === timelineToRemove) {
            newActiveTimelineId = null;
          }
        }
      }

      return {
        ...timelinesState,
        timelines: modifiedTimelines,
        activeTimelineId: newActiveTimelineId,
      };
    }

    default:
      // Most commands don't affect timeline state
      return timelinesState;
  }
}
