import { get } from 'svelte/store';
import { initializeAutoRenderSubscription } from './autoRender';
import { initializeGroupsSubscription } from './groups';
import { logger } from './logging';
import {
  initializeOperationsSubscription,
  getOperationById,
  type OperationDef,
  buildOperationDurationMap,
  buildBackendGraphForTimeline,
} from './operation';
import {
  buildTimelineForOp,
  type BuildOpPlaybackGraphRequest,
  type AddOpRequest,
  type MergeInputRequest,
} from './opPlaybackService';
import { appState } from './state.svelte';
import { initializeStatusPublishers } from './status-publishers';
import { timelinePlaybackStoreService } from './timeline/timelinePlaybackState';
import { initializeTimelineSync } from './timeline/timelines';
import { timelinePlaybackService } from './timelinePlaybackService';
import { undo, redo, canUndo, canRedo } from './undo/undo';
import { initWaveformService } from './waveformCache';
// import { subscribeForTimelineStoreSerialization } from './timeline/persistentTimeline';

/**
 * Build backend playback graphs for all existing timelines in the store
 * This ensures that timelines can be played immediately after initialization
 */
async function buildBackendGraphsForAllTimelines(): Promise<void> {
  const currentAppState = get(appState);
  const timelinesState = currentAppState.timelines;

  if (!currentAppState.operations?.defs) {
    logger.opPlayback.info('No operations available, skipping timeline graph building');
    return;
  }

  if (!timelinesState?.timelines) {
    logger.opPlayback.info('No timelines state available, skipping timeline graph building');
    return;
  }

  const timelineIds = Object.keys(timelinesState.timelines);
  if (timelineIds.length === 0) {
    logger.opPlayback.info('No timelines to build backend graphs for');
    return;
  }

  logger.opPlayback.info(`Building backend graphs for ${timelineIds.length} timelines`);

  // Build graphs for all timelines in parallel
  const buildPromises = timelineIds.map(async timelineId => {
    const timeline = timelinesState.timelines[timelineId];
    timelinePlaybackStoreService.addTimeline(timelineId);
    if (!timeline || timeline.source.kind !== 'operation') {
      logger.opPlayback.warning(`Skipping timeline ${timelineId}: not an operation-based timeline`);
      return;
    }

    const operationId = timeline.source.operationId;
    await buildBackendGraphForTimeline(timelineId, operationId);
  });

  // Wait for all graphs to be built
  await Promise.allSettled(buildPromises);
  logger.opPlayback.info('Finished building backend graphs for all timelines');
}

/**
 * Initialize all frontend systems and services
 * Called once on application mount
 *
 * @returns Cleanup function to call on application destroy
 */
export function initializeFrontend(): () => void {
  // Initialize subscriptions to avoid circular dependency issues
  initializeGroupsSubscription();
  initializeOperationsSubscription();

  // Initialize render of ops with auto render policy after rev is bumped
  initializeAutoRenderSubscription();
  // Initialize automatic status publishers (buffering, etc.)
  initializeStatusPublishers();
  initializeTimelineSync();
  // subscribeForTimelineStoreSerialization();
  // Initialize waveform service (handles loading waveforms when operation changes)
  const cleanupWaveformService = initWaveformService();

  // Build backend playback graphs for all existing timelines
  buildBackendGraphsForAllTimelines().catch(error => {
    logger.opPlayback.error(
      'Failed to build backend graphs for timelines during initialization:',
      error
    );
  });

  // Setup keyboard shortcuts
  const handleKeyPress = (ev: KeyboardEvent) => {
    // Handle spacebar for play/pause
    if (ev.code === 'Space' && !ev.shiftKey && !ev.ctrlKey && !ev.metaKey) {
      // Only handle spacebar if not focused on an input element
      if (ev.target instanceof HTMLInputElement || ev.target instanceof HTMLTextAreaElement) {
        return;
      }
      ev.preventDefault(); // Prevent default scrolling
      // Use the timeline playback service for the active timeline

      timelinePlaybackService.togglePlayPauseActiveTimeline().catch((err: Error) => {
        console.error('Error toggling playback:', err);
      });
      return;
    }

    // Handle undo/redo shortcuts
    if ((ev.ctrlKey || ev.metaKey) && !ev.altKey) {
      if (ev.key === 'z' && !ev.shiftKey) {
        // Ctrl+Z or Cmd+Z for undo
        ev.preventDefault();
        if (canUndo()) {
          undo();
          console.log('🔄 Undo triggered via keyboard shortcut');
        }
        return;
      }

      if (ev.key === 'y' || (ev.key === 'z' && ev.shiftKey)) {
        // Ctrl+Y or Ctrl+Shift+Z or Cmd+Y or Cmd+Shift+Z for redo
        ev.preventDefault();
        if (canRedo()) {
          redo();
          console.log('🔄 Redo triggered via keyboard shortcut');
        }
        return;
      }
    }
  };

  window.addEventListener('keydown', handleKeyPress);

  // Return cleanup function
  return () => {
    window.removeEventListener('keydown', handleKeyPress);
    cleanupWaveformService?.();
  };
}
