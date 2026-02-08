import { get } from 'svelte/store';
import { initializeAutoRenderSubscription } from './autoRender';
import { durationCache } from './durationCache';
import { initializeGroupsSubscription } from './groups';
import { logger } from './logging';
import { initializeOperationsSubscription } from './operation';
import type { OperationDef } from './operation';
import {
  buildTimelineForOp,
  type BuildOpPlaybackGraphRequest,
  type AddOpRequest,
  type MergeInputRequest,
} from './opPlaybackService';
import { appState } from './state.svelte';
import { initializeStatusPublishers } from './status-publishers';
import { timelinesStore } from './timeline/timelines';
import { timelinePlaybackService } from './timelinePlaybackService';
import { undo, redo, canUndo, canRedo } from './undo/undo';
import { initWaveformService } from './waveformCache';
// import { subscribeForTimelineStoreSerialization } from './timeline/persistentTimeline';

/**
 * Build backend playback graphs for all existing timelines in the store
 * This ensures that timelines can be played immediately after initialization
 */
async function buildBackendGraphsForAllTimelines(): Promise<void> {
  const currentTimelinesState = get(timelinesStore);
  const currentAppState = get(appState);

  if (!currentAppState.operations?.defs) {
    logger.opPlayback.info('No operations available, skipping timeline graph building');
    return;
  }

  const timelineIds = Object.keys(currentTimelinesState.timelines);
  if (timelineIds.length === 0) {
    logger.opPlayback.info('No timelines to build backend graphs for');
    return;
  }

  logger.opPlayback.info(`Building backend graphs for ${timelineIds.length} timelines`);

  // Build graphs for all timelines in parallel
  const buildPromises = timelineIds.map(async timelineId => {
    const timeline = currentTimelinesState.timelines[timelineId];

    if (!timeline || timeline.source.kind !== 'operation') {
      logger.opPlayback.warning(`Skipping timeline ${timelineId}: not an operation-based timeline`);
      return;
    }

    const operationId = timeline.source.operationId;
    const operation = currentAppState.operations?.defs?.[operationId];

    if (!operation) {
      logger.opPlayback.error(`Operation ${operationId} not found for timeline ${timelineId}`);
      return;
    }

    try {
      logger.opPlayback.info(
        `Building backend graph for timeline ${timelineId}, operation ${operationId} (${operation.kind})`
      );

      const operationDefs = currentAppState.operations?.defs;
      if (!operationDefs) {
        logger.opPlayback.error('No operation definitions available');
        return;
      }

      // Get durations from cache for accurate timing
      const durationsMap = new Map<string, number>();

      // Recursive function to collect all file paths from an operation
      function collectFilePaths(op: OperationDef): string[] {
        const paths: string[] = [];

        if (op.kind === 'sample') {
          const fileSource = op.sources.find(s => s.type === 'file');
          if (fileSource && fileSource.type === 'file') {
            paths.push(fileSource.fileId);
          }
        } else if (op.kind === 'merge') {
          for (const source of op.sources) {
            if (source.type === 'operation' && operationDefs) {
              const sourceOp = operationDefs[source.operationId];
              if (sourceOp) {
                paths.push(...collectFilePaths(sourceOp));
              }
            }
          }
        }

        return paths;
      }

      // Collect all file paths and load their durations
      const filePaths = collectFilePaths(operation);
      if (filePaths.length > 0) {
        try {
          const durations = await durationCache.getBatch(filePaths);
          for (const [filePath, duration] of durations.entries()) {
            if (duration && duration > 0) {
              durationsMap.set(filePath, duration);
            }
          }
        } catch (error) {
          logger.opPlayback.warning(
            `Failed to load durations for timeline ${timelineId}, using placeholder durations`
          );
        }
      }

      // Recursive function to convert operations to AddOpRequest (same as buildPlaybackGraphFromMergeOp)
      function convertOperationToAddOpRequest(
        op: OperationDef,
        opId: string,
        startTime: number
      ): { operations: AddOpRequest[]; totalDuration: number } {
        const result: AddOpRequest[] = [];
        let totalDuration = 0;

        if (op.kind === 'sample') {
          // Handle sample operation
          const fileSource = op.sources.find(s => s.type === 'file');
          if (fileSource && fileSource.type === 'file') {
            const duration = durationsMap.get(fileSource.fileId);

            if (!duration) {
              logger.opPlayback.warning(
                `No duration cached for ${fileSource.fileId}, skipping from playback graph`
              );
              return { operations: result, totalDuration: 0 };
            }

            result.push({
              name: `${op.name}_sample`,
              opType: 'sample',
              filePath: fileSource.fileId,
              startTime: startTime,
              endTime: startTime + duration,
              gain: 1.0,
            });

            totalDuration = duration;
          }
        } else if (op.kind === 'merge') {
          // Handle merge operation - create separate operations for each source
          let currentOffset = startTime;
          const mergeInputs: MergeInputRequest[] = [];

          for (let i = 0; i < op.sources.length; i++) {
            const source = op.sources[i];
            if (!source || source.type !== 'operation') {
              logger.opPlayback.warning(
                `Unsupported source type "${source?.type}" in MergeOp, skipping`
              );
              continue;
            }

            const sourceOp = operationDefs?.[source.operationId];
            if (!sourceOp) {
              logger.opPlayback.warning(
                `Referenced operation id="${source.operationId}" not found`
              );
              continue;
            }

            if (sourceOp.kind === 'sample') {
              // For sample operations, add them as merge inputs
              const fileSource = sourceOp.sources.find(s => s.type === 'file');
              if (fileSource && fileSource.type === 'file') {
                const duration = durationsMap.get(fileSource.fileId);

                if (!duration) {
                  logger.opPlayback.warning(
                    `No duration cached for ${fileSource.fileId}, skipping from merge`
                  );
                  continue;
                }

                mergeInputs.push({
                  filePath: fileSource.fileId,
                  offset: currentOffset - startTime, // Offset relative to merge start
                  gain: 1.0,
                });

                currentOffset += duration;
              }
            } else if (sourceOp.kind === 'merge') {
              // For nested merge operations, recursively convert them
              const nestedResult = convertOperationToAddOpRequest(
                sourceOp,
                sourceOp.id,
                currentOffset
              );

              result.push(...nestedResult.operations);
              currentOffset += nestedResult.totalDuration;
            }
          }

          // If we have merge inputs, create a merge operation
          if (mergeInputs.length > 0) {
            result.push({
              name: `${op.name}_merge`,
              opType: 'merge',
              startTime: startTime,
              endTime: currentOffset,
              gain: 1.0,
              inputs: mergeInputs,
            });
          }

          totalDuration = currentOffset - startTime;
        }

        return { operations: result, totalDuration };
      }

      // Convert the selected operation using the recursive approach
      const conversionResult = convertOperationToAddOpRequest(operation, operationId, 0);
      const operations = conversionResult.operations;

      if (operations.length === 0) {
        logger.opPlayback.warning(`No valid operations generated for timeline ${timelineId}`);
        return;
      }

      // Create the build graph request
      const request: BuildOpPlaybackGraphRequest = {
        operations,
        sampleRate: 44100,
        channels: 2,
        loopPlayback: true,
      };

      await buildTimelineForOp(timelineId, request);
      logger.opPlayback.info(
        `Successfully built backend graph for timeline ${timelineId}: ${operations.length} ops, ${conversionResult.totalDuration.toFixed(2)}s`
      );
    } catch (error) {
      logger.opPlayback.error(`Failed to build backend graph for timeline ${timelineId}:`, error);
    }
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
