import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { writable, derived, get, type Readable, type Writable } from 'svelte/store';
import { logger } from './logging';
import { invokeWithPerf } from './performance';
import {
  buildGraph as buildGraphInternal,
  buildGraphFromFiles as buildGraphFromFilesInternal,
} from './playbackGraphUtils';
import { createTypedEventChannelWithLoggingAndStatusMessages } from '$lib/utils/channelMaker';

/**
 * Response after building a graph
 */
export interface BuildGraphResponse {
  operationCount: number;
  totalDurationSeconds: number;
  sampleRate: number;
  channels: number;
}

/**
 * Child input for a merge operation
 */
export interface MergeInputRequest {
  /** File path to load samples from */
  filePath?: string;
  /** Pre-loaded samples (f32, interleaved) */
  samples?: number[];
  /** Offset time in seconds within the merge operation */
  offset: number;
  /** Gain for this input (0.0 to 1.0+) */
  gain?: number;
}

/**
 * Request to add an operation to the playback graph
 */
export interface AddOpRequest {
  /** Unique name for this operation */
  name: string;
  /** Type of operation (sample or merge) */
  opType?: 'sample' | 'merge';
  /** File path to load samples from (for sample-based ops) */
  filePath?: string;
  /** Pre-loaded samples (f32, interleaved) - usually not used from frontend */
  samples?: number[];
  /** Start time in seconds on the timeline */
  startTime: number;
  /** End time in seconds on the timeline (if undefined, uses operation duration) */
  endTime?: number;
  /** Gain for this operation (0.0 to 1.0+) */
  gain?: number;
  /** Child inputs for merge operations */
  inputs?: MergeInputRequest[];
}

/**
 * Request to build a playback graph
 */
export interface BuildGraphRequest {
  /** Operations to add to the graph */
  operations: AddOpRequest[];
  /** Sample rate for playback (default: 44100) */
  sampleRate?: number;
  /** Number of channels (default: 2) */
  channels?: number;
  /** Whether to loop playback (default: true) */
  loopPlayback?: boolean;
}

/**
 * Build graph event types from Rust backend (timeline-aware)
 */
export interface OpPlaybackBuildGraphEvent {
  event: 'started' | 'progress' | 'finished';
  data:
    | {
        timelineId: string;
        operationCount: number;
      } // started
    | {
        timelineId: string;
        operationName: string;
        operationIndex: number;
        totalOperations: number;
        durationSeconds: number;
      } // progress
    | {
        timelineId: string;
        operationCount: number;
        totalDurationSeconds: number;
        sampleRate: number;
        channels: number;
      }; // finished
}

/**
 * Timeline progress event from Rust backend
 */
export interface OpTimelineProgressEvent {
  timelineId: string | null;
  progress: number;
}

/**
 * Build graph state for tracking progress
 */
export interface BuildGraphState {
  /** Whether a build is in progress */
  isBuilding: boolean;
  /** Current operation being processed */
  currentOperation?: string;
  /** Current operation index */
  currentIndex: number;
  /** Total number of operations */
  totalOperations: number;
  /** Build progress (0.0 to 1.0) */
  buildProgress: number;
}

/**
 * Playback state
 */
export interface OpPlaybackState {
  /** Whether playback is active */
  isPlaying: boolean;
  /** Whether playback is paused */
  isPaused: boolean;
  /** Current playback progress (0.0 to 1.0) */
  progress: number;
  /** Current playback position in seconds */
  positionSeconds: number;
  /** Total duration in seconds */
  durationSeconds: number;
  /** Whether the graph is loaded */
  hasGraph: boolean;
  /** Build graph state */
  buildState: BuildGraphState;
  /** Whether playback should loop */
  loopEnabled: boolean;
  /** Current volume (0.0 to 1.0) */
  volume: number;
}

// ============================================================================
// STORES
// ============================================================================

/**
 * Internal state store
 */
const internalState = writable<OpPlaybackState>({
  isPlaying: false,
  isPaused: false,
  progress: 0,
  positionSeconds: 0,
  durationSeconds: 0,
  hasGraph: false,
  loopEnabled: true,
  volume: 1.0,
  buildState: {
    isBuilding: false,
    currentIndex: 0,
    totalOperations: 0,
    buildProgress: 0,
  },
});

/**
 * Public read-only state store
 */
export const opPlaybackState: Readable<OpPlaybackState> = derived(internalState, $state => $state);

/**
 * Sample cache toggle - when false, uses legacy build without cache
 */
export const useSampleCache = writable<boolean>(true);

/**
 * Current progress (0.0 to 1.0)
 */
export const opPlaybackProgress: Readable<number> = derived(
  internalState,
  $state => $state.progress
);

/**
 * Whether currently playing
 */
export const opIsPlaying: Readable<boolean> = derived(internalState, $state => $state.isPlaying);

/**
 * Whether currently paused
 */
export const opIsPaused: Readable<boolean> = derived(internalState, $state => $state.isPaused);

// ============================================================================
// EVENT LISTENERS
// ============================================================================

let progressUnlisten: UnlistenFn | null = null;

/**
 * Initialize the progress event listener (legacy global listener)
 *
 * NOTE: This is being phased out in favor of timeline-specific progress handling.
 * New timeline progress events include timelineId and should be handled in timelines.ts
 */
async function initProgressListener(): Promise<void> {
  if (progressUnlisten) return;

  progressUnlisten = await listen<OpTimelineProgressEvent>('op-timeline-progress', event => {
    const { timelineId, progress } = event.payload;
    const state = get(internalState);
    const newPositionSeconds = progress * state.durationSeconds;

    // Only update global state if this is for the "active" timeline or no specific timeline
    // This maintains backward compatibility for legacy single-timeline usage
    if (!timelineId || timelineId === 'global') {
      // Add detailed progress logging
      if (Math.abs(newPositionSeconds - state.positionSeconds) > 0.1) {
        logger.opPlayback.info(
          `Progress update: ${(progress * 100).toFixed(1)}% -> ${newPositionSeconds.toFixed(2)}s (was ${state.positionSeconds.toFixed(2)}s)`
        );
      }

      internalState.update(s => ({
        ...s,
        progress,
        positionSeconds: newPositionSeconds,
      }));
    } else {
      // Timeline-specific progress - should be handled by timeline stores
      logger.opPlayback.info(
        `Timeline-specific progress for '${timelineId}': ${(progress * 100).toFixed(1)}% (handled by timeline store)`
      );
    }
  });

  logger.opPlayback.info('Progress listener initialized (legacy mode)');
}

/**
 * Cleanup the progress event listener
 */
function cleanupProgressListener(): void {
  if (progressUnlisten) {
    progressUnlisten();
    progressUnlisten = null;
    logger.opPlayback.info('Progress listener cleaned up');
  }
}

/**
 * Initialize all event listeners
 */
async function initEventListeners(): Promise<void> {
  await initProgressListener();
}

/**
 * Cleanup all event listeners
 */
function cleanupEventListeners(): void {
  cleanupProgressListener();
}

// ============================================================================
// API FUNCTIONS
// ============================================================================

/**
 * Build a playback graph for a specific timeline
 */
export async function buildGraphForTimeline(
  timelineId: string,
  request: BuildGraphRequest
): Promise<BuildGraphResponse> {
  // Ensure progress listener is initialized
  await initProgressListener();
  console.log(timelineId);
  logger.opPlayback.info(
    `Building graph for timeline '${timelineId}' with ${request.operations.length} operations`
  );

  // Track build time
  let buildStartTime: number;

  try {
    // Create typed event channel with automatic logging and status publishing

    const onBuildGraphEvent =
      createTypedEventChannelWithLoggingAndStatusMessages<OpPlaybackBuildGraphEvent>(
        'BuildGraphForTimeline',
        {
          source: `build-graph-${timelineId}`,
          startedMessage: data =>
            `Building graph for timeline '${timelineId}' (${(data as any).operationCount} operations)...`,
          progressMessage: data => {
            const progressData = data as any;
            return `Building: ${progressData.operationName} (${progressData.operationIndex + 1}/${progressData.totalOperations}) for timeline '${timelineId}'`;
          },
          finishedMessage: data => {
            const buildTimeMs = Date.now() - buildStartTime;
            const buildTimeSec = (buildTimeMs / 1000).toFixed(2);
            const finishedData = data as any;
            const audioDuration = finishedData.totalDurationSeconds.toFixed(1);
            return `Built ${finishedData.operationCount} ops in ${buildTimeSec}s → ${audioDuration}s audio for timeline '${timelineId}'`;
          },
          getProgress: data => {
            const progressData = data as any;
            return progressData.operationIndex
              ? (progressData.operationIndex + 1) / progressData.totalOperations
              : 0;
          },
          autoClearSuccess: 2000,
        },
        {
          onStarted: data => {
            // Record build start time
            buildStartTime = Date.now();
            logger.opPlayback.info(`Started building graph for timeline '${timelineId}'`);
          },
          onProgress: data => {
            const progressData = data as any;
            const buildProgress = (progressData.operationIndex + 1) / progressData.totalOperations;
            logger.opPlayback.info(
              `Building timeline '${timelineId}': ${progressData.operationName} (${(buildProgress * 100).toFixed(1)}%)`
            );
          },
          onFinished: data => {
            logger.opPlayback.info(`Finished building graph for timeline '${timelineId}'`);
          },
        }
      );

    const result = await invokeWithPerf<BuildGraphResponse>('op_playback_build_graph', {
      timelineId,
      request,
      onEvent: onBuildGraphEvent,
    });

    if (!result.ok) {
      throw new Error(
        `Failed to build graph for timeline '${timelineId}': ${result.error.message}`
      );
    }

    // Update global state for backward compatibility (if this becomes the active timeline)
    internalState.update(s => ({
      ...s,
      hasGraph: true,
      durationSeconds: result.value.totalDurationSeconds,
      buildState: {
        ...s.buildState,
        isBuilding: false,
        buildProgress: 1.0,
      },
    }));

    logger.opPlayback.success(
      `Graph built for timeline '${timelineId}': ${result.value.operationCount} operations, ${result.value.totalDurationSeconds.toFixed(2)}s`
    );
    return result.value;
  } catch (error) {
    logger.opPlayback.error(`Failed to build graph for timeline '${timelineId}':`, error);

    // Error status is handled by re-importing for this specific case
    const { publishStatus, clearSource } = await import('./status');
    clearSource(`build-graph-${timelineId}`);
    publishStatus({
      source: `build-graph-${timelineId}`,
      level: 'error',
      message: `Failed to build graph for timeline '${timelineId}': ${error instanceof Error ? error.message : 'Unknown error'}`,
      sticky: true,
    });

    throw error;
  }
}

/**
 * Build a playback graph from operations (legacy - uses global timeline)
 */
export async function buildGraph(request: BuildGraphRequest): Promise<BuildGraphResponse> {
  // For backward compatibility, use a default timeline ID
  return buildGraphForTimeline('global', request);
}

/**
 * Build a graph from file paths for a specific timeline
 */
export async function buildGraphFromFilesForTimeline(
  timelineId: string,
  filePaths: string[],
  options: {
    sampleRate?: number;
    channels?: number;
    loopPlayback?: boolean;
    gap?: number; // Gap between files in seconds
  } = {}
): Promise<BuildGraphResponse> {
  // Create operations for each file
  const operations: AddOpRequest[] = [];
  let currentTime = 0;

  for (const filePath of filePaths) {
    operations.push({
      name: `file-${operations.length}`,
      opType: 'sample',
      filePath,
      startTime: currentTime,
      // endTime will be determined by file duration
    });

    // For now, assume a duration and add gap
    // In a real implementation, you'd get the actual duration
    const estimatedDuration = 30; // seconds - placeholder
    currentTime += estimatedDuration + (options.gap ?? 0);
  }

  const request: BuildGraphRequest = {
    operations,
    sampleRate: options.sampleRate,
    channels: options.channels,
    loopPlayback: options.loopPlayback,
  };

  return buildGraphForTimeline(timelineId, request);
}

/**
 * Build a graph from file paths with automatic timing
 * Operations are scheduled sequentially (one after another)
 */
export async function buildGraphFromFiles(
  filePaths: string[],
  options: {
    sampleRate?: number;
    channels?: number;
    loopPlayback?: boolean;
    gap?: number; // Gap between files in seconds
  } = {}
): Promise<BuildGraphResponse> {
  // Ensure progress listener is initialized
  await initProgressListener();

  // Use the utility function with state updater
  return buildGraphFromFilesInternal(filePaths, internalState.update, options);
}

/**
 * Start playback for a specific timeline
 */
export async function playTimeline(timelineId: string, startSeconds?: number): Promise<void> {
  logger.opPlayback.info(
    `Starting playback for timeline '${timelineId}' at ${startSeconds?.toFixed(2) ?? 'current position'}s`
  );

  try {
    const result = await invokeWithPerf('op_playback_play', {
      timelineId,
      startSeconds,
    });

    if (!result.ok) {
      throw new Error(
        `Failed to start playback for timeline '${timelineId}': ${result.error.message}`
      );
    }

    internalState.update(s => ({
      ...s,
      isPlaying: true,
      isPaused: false,
    }));

    logger.opPlayback.success(`Playback started for timeline '${timelineId}'`);
  } catch (error) {
    logger.opPlayback.error(`Failed to start playback for timeline '${timelineId}':`, error);
    throw error;
  }
}

/**
 * Start playback (legacy - uses global timeline behavior)
 */
export async function play(startSeconds?: number): Promise<void> {
  const currentState = get(internalState);
  const actualStartSeconds = startSeconds ?? currentState.positionSeconds;

  logger.opPlayback.info(
    `Starting playback at ${actualStartSeconds.toFixed(2)}s (${startSeconds !== undefined ? 'explicit' : 'current position'})`
  );
  logger.opPlayback.info(
    `Current state - position: ${currentState.positionSeconds.toFixed(2)}s, progress: ${(currentState.progress * 100).toFixed(1)}%, isPaused: ${currentState.isPaused}`
  );

  try {
    const result = await invokeWithPerf('op_playback_play', {
      timelineId: 'global',
      startSeconds: actualStartSeconds,
    });

    if (!result.ok) {
      throw new Error(`Failed to start playback: ${result.error.message}`);
    }

    internalState.update(s => ({
      ...s,
      isPlaying: true,
      isPaused: false,
    }));

    logger.opPlayback.success(`Playback started at ${actualStartSeconds.toFixed(2)}s`);
  } catch (error) {
    logger.opPlayback.error('Failed to start playback:', error);
    throw error;
  }
}

/**
 * Pause playback
 */
export async function pause(): Promise<void> {
  const currentState = get(internalState);
  logger.opPlayback.info(
    `Pausing playback at position ${currentState.positionSeconds.toFixed(2)}s (progress: ${(currentState.progress * 100).toFixed(1)}%)`
  );

  try {
    const result = await invokeWithPerf('op_playback_pause');

    if (!result.ok) {
      throw new Error(`Failed to pause playback: ${result.error.message}`);
    }

    internalState.update(s => ({
      ...s,
      isPaused: true,
    }));

    const newState = get(internalState);
    logger.opPlayback.success(
      `Playback paused - Position preserved: ${newState.positionSeconds.toFixed(2)}s`
    );
  } catch (error) {
    logger.opPlayback.error('Failed to pause playback:', error);
    throw error;
  }
}

/**
 * Pause playback for a specific timeline
 */
export async function pauseTimeline(timelineId: string): Promise<void> {
  logger.opPlayback.info(`Pausing playback for timeline '${timelineId}'`);

  try {
    const result = await invokeWithPerf('op_playback_pause', { timelineId });

    if (!result.ok) {
      throw new Error(`Failed to pause timeline '${timelineId}': ${result.error.message}`);
    }

    internalState.update(s => ({
      ...s,
      isPaused: true,
    }));

    logger.opPlayback.success(`Timeline '${timelineId}' paused`);
  } catch (error) {
    logger.opPlayback.error(`Failed to pause timeline '${timelineId}':`, error);
    throw error;
  }
}

/**
 * Resume playback
 */
export async function resume(): Promise<void> {
  const currentState = get(internalState);
  logger.opPlayback.info(
    `Resuming playback from position ${currentState.positionSeconds.toFixed(2)}s (progress: ${(currentState.progress * 100).toFixed(1)}%)`
  );

  try {
    const result = await invokeWithPerf('op_playback_resume');

    if (!result.ok) {
      throw new Error(`Failed to resume playback: ${result.error.message}`);
    }

    internalState.update(s => ({
      ...s,
      isPaused: false,
    }));

    const newState = get(internalState);
    logger.opPlayback.success(
      `Playback resumed from position ${newState.positionSeconds.toFixed(2)}s`
    );
  } catch (error) {
    logger.opPlayback.error('Failed to resume playback:', error);
    throw error;
  }
}

/**
 * Resume playback for a specific timeline
 */
export async function resumeTimeline(timelineId: string): Promise<void> {
  logger.opPlayback.info(`Resuming playback for timeline '${timelineId}'`);

  try {
    const result = await invokeWithPerf('op_playback_resume', { timelineId });

    if (!result.ok) {
      throw new Error(`Failed to resume timeline '${timelineId}': ${result.error.message}`);
    }

    internalState.update(s => ({
      ...s,
      isPaused: false,
    }));

    logger.opPlayback.success(`Timeline '${timelineId}' resumed`);
  } catch (error) {
    logger.opPlayback.error(`Failed to resume timeline '${timelineId}':`, error);
    throw error;
  }
}

/**
 * Stop playback
 */
export async function stop(): Promise<void> {
  logger.opPlayback.info('Stopping playback');

  try {
    const result = await invokeWithPerf('op_playback_stop');

    if (!result.ok) {
      throw new Error(`Failed to stop playback: ${result.error.message}`);
    }

    internalState.update(s => ({
      ...s,
      isPlaying: false,
      isPaused: false,
      progress: 0,
      positionSeconds: 0,
    }));

    logger.opPlayback.success('Playback stopped');
  } catch (error) {
    logger.opPlayback.error('Failed to stop playback:', error);
    throw error;
  }
}

/**
 * Stop playback for a specific timeline
 */
export async function stopTimeline(timelineId: string): Promise<void> {
  logger.opPlayback.info(`Stopping playback for timeline '${timelineId}'`);

  try {
    const result = await invokeWithPerf('op_playback_stop', { timelineId });

    if (!result.ok) {
      throw new Error(`Failed to stop timeline '${timelineId}': ${result.error.message}`);
    }

    internalState.update(s => ({
      ...s,
      isPlaying: false,
      isPaused: false,
      progress: 0,
      positionSeconds: 0,
    }));

    logger.opPlayback.success(`Timeline '${timelineId}' stopped`);
  } catch (error) {
    logger.opPlayback.error(`Failed to stop timeline '${timelineId}':`, error);
    throw error;
  }
}

/**
 * Seek to a position in a specific timeline
 */
export async function seekTimeline(timelineId: string, positionSeconds: number): Promise<void> {
  logger.opPlayback.info(`Seeking timeline '${timelineId}' to ${positionSeconds.toFixed(2)}s`);

  try {
    const result = await invokeWithPerf('op_playback_seek', {
      timelineId,
      positionSeconds,
    });

    if (!result.ok) {
      throw new Error(`Failed to seek timeline '${timelineId}': ${result.error.message}`);
    }

    logger.opPlayback.success(`Seeked timeline '${timelineId}' to ${positionSeconds.toFixed(2)}s`);
  } catch (error) {
    logger.opPlayback.error(`Failed to seek timeline '${timelineId}':`, error);
    throw error;
  }
}

/**
 * Seek to a position (legacy - uses global timeline)
 */
export async function seek(positionSeconds: number): Promise<void> {
  logger.opPlayback.info(`Seeking to ${positionSeconds.toFixed(2)}s`);

  try {
    const result = await invokeWithPerf('op_playback_seek', {
      timelineId: 'global',
      positionSeconds,
    });

    if (!result.ok) {
      throw new Error(`Failed to seek: ${result.error.message}`);
    }

    const state = get(internalState);
    const progress = state.durationSeconds > 0 ? positionSeconds / state.durationSeconds : 0;

    logger.opPlayback.info(
      `Seek - calculated progress: ${(progress * 100).toFixed(1)}%, clamped position: ${Math.max(0, positionSeconds).toFixed(2)}s`
    );

    internalState.update(s => ({
      ...s,
      progress: Math.max(0, Math.min(1, progress)),
      positionSeconds: Math.max(0, positionSeconds),
    }));

    logger.opPlayback.success(`Seeked to ${positionSeconds.toFixed(2)}s`);
  } catch (error) {
    logger.opPlayback.error('Failed to seek:', error);
    throw error;
  }
}

/**
 * Seek to a normalized progress position (0.0 to 1.0)
 */
export async function seekToProgress(progress: number): Promise<void> {
  const state = get(internalState);
  const positionSeconds = progress * state.durationSeconds;
  await seek(positionSeconds);
}

/**
 * Set playback volume
 */
export async function setVolume(volume: number): Promise<void> {
  logger.opPlayback.info(`Setting volume to ${(volume * 100).toFixed(0)}%`);

  try {
    const result = await invokeWithPerf('op_playback_set_volume', { volume });

    if (!result.ok) {
      throw new Error(`Failed to set volume: ${result.error.message}`);
    }

    internalState.update(s => ({
      ...s,
      volume: Math.max(0, Math.min(2, volume)), // Allow up to 2x volume
    }));

    logger.opPlayback.success(`Volume set to ${(volume * 100).toFixed(0)}%`);
  } catch (error) {
    logger.opPlayback.error('Failed to set volume:', error);
    throw error;
  }
}

/**
 * Set loop mode for a specific timeline
 */
export async function setTimelineLoop(timelineId: string, enabled: boolean): Promise<void> {
  logger.opPlayback.info(`Setting loop mode to ${enabled} for timeline '${timelineId}'`);

  try {
    const result = await invokeWithPerf('op_playback_set_loop', {
      timelineId,
      loopPlayback: enabled,
    });

    if (!result.ok) {
      throw new Error(
        `Failed to set loop mode for timeline '${timelineId}': ${result.error.message}`
      );
    }

    logger.opPlayback.success(`Loop mode set to ${enabled} for timeline '${timelineId}'`);
  } catch (error) {
    logger.opPlayback.error(`Failed to set loop mode for timeline '${timelineId}':`, error);
    throw error;
  }
}

/**
 * Set loop mode (legacy - uses global timeline)
 */
export async function setLoop(enabled: boolean): Promise<void> {
  logger.opPlayback.info(`Setting loop mode to ${enabled}`);

  try {
    const result = await invokeWithPerf('op_playback_set_loop', {
      timelineId: 'global',
      loopPlayback: enabled,
    });

    if (!result.ok) {
      throw new Error(`Failed to set loop mode: ${result.error.message}`);
    }

    internalState.update(s => ({
      ...s,
      loopEnabled: enabled,
    }));

    logger.opPlayback.success(`Loop mode set to ${enabled}`);
  } catch (error) {
    logger.opPlayback.error('Failed to set loop mode:', error);
    throw error;
  }
}

/**
 * Get current progress for a specific timeline
 */
export async function getTimelineProgress(timelineId: string): Promise<number> {
  const result = await invokeWithPerf<number>('op_playback_get_progress', { timelineId });

  if (!result.ok) {
    throw new Error(`Failed to get progress for timeline '${timelineId}': ${result.error.message}`);
  }

  return result.value;
}

/**
 * Clear a specific timeline's playback session
 */
export async function clearTimeline(timelineId: string): Promise<void> {
  logger.opPlayback.info(`Clearing timeline '${timelineId}'`);

  try {
    const result = await invokeWithPerf('op_playback_clear_timeline', { timelineId });

    if (!result.ok) {
      throw new Error(`Failed to clear timeline '${timelineId}': ${result.error.message}`);
    }

    logger.opPlayback.success(`Timeline '${timelineId}' cleared`);
  } catch (error) {
    logger.opPlayback.error(`Failed to clear timeline '${timelineId}':`, error);
    throw error;
  }
}

/**
 * Clear all timeline playback sessions
 */
export async function clearAllTimelines(): Promise<void> {
  logger.opPlayback.info('Clearing all timelines');

  try {
    const result = await invokeWithPerf('op_playback_clear_all_timelines');

    if (!result.ok) {
      throw new Error(`Failed to clear all timelines: ${result.error.message}`);
    }

    // Reset global state
    internalState.update(s => ({
      ...s,
      hasGraph: false,
      durationSeconds: 0,
      progress: 0,
      positionSeconds: 0,
      isPlaying: false,
      isPaused: false,
    }));

    logger.opPlayback.success('All timelines cleared');
  } catch (error) {
    logger.opPlayback.error('Failed to clear all timelines:', error);
    throw error;
  }
}

/**
 * Get current progress (sync, from store)
 */
export function getProgress(): number {
  return get(internalState).progress;
}

/**
 * Get current progress from backend (async)
 */
export async function getProgressAsync(): Promise<number> {
  const result = await invokeWithPerf<number>('op_playback_get_progress');

  if (!result.ok) {
    throw new Error(`Failed to get progress: ${result.error.message}`);
  }

  return result.value;
}

/**
 * Clear the current playback graph
 */
export async function clearGraph(): Promise<void> {
  logger.opPlayback.info('Clearing playback graph');

  try {
    const result = await invokeWithPerf('op_playback_clear_graph');

    if (!result.ok) {
      throw new Error(`Failed to clear graph: ${result.error.message}`);
    }

    internalState.update(s => ({
      ...s,
      hasGraph: false,
      isPlaying: false,
      isPaused: false,
      progress: 0,
      positionSeconds: 0,
      durationSeconds: 0,
    }));

    logger.opPlayback.success('Playback graph cleared');
  } catch (error) {
    logger.opPlayback.error('Failed to clear graph:', error);
    throw error;
  }
}

/**
 * Toggle play/pause
 */
export async function togglePlayPause(): Promise<void> {
  const state = get(internalState);

  if (!state.hasGraph) {
    logger.opPlayback.warning('No playback graph available');
    return;
  }

  logger.opPlayback.info(
    `Toggle play/pause - Current state: playing=${state.isPlaying}, paused=${state.isPaused}, position=${state.positionSeconds.toFixed(2)}s`
  );

  if (state.isPlaying && !state.isPaused) {
    logger.opPlayback.info('Currently playing -> pausing');
    await pause();
  } else if (state.isPaused) {
    logger.opPlayback.info('Currently paused -> resuming');
    await resume();
  } else {
    logger.opPlayback.info('Currently stopped -> playing from current position');
    await play();
  }
}

// ============================================================================
// NAMESPACE EXPORT
// ============================================================================

/**
 * Operation playback service - all functions in one namespace
 */
export const opPlaybackService = {
  // State stores
  state: opPlaybackState,
  progress: opPlaybackProgress,
  isPlaying: opIsPlaying,
  isPaused: opIsPaused,

  // Legacy single-timeline control functions
  buildGraph,
  buildGraphFromFiles,
  // play,
  // pause,
  // resume,
  // stop,
  // seek,
  seekToProgress,
  setVolume,
  setLoop,
  togglePlayPause,
  clearGraph,

  // New timeline-aware control functions
  buildGraphForTimeline,
  buildGraphFromFilesForTimeline,
  playTimeline,
  pauseTimeline,
  resumeTimeline,
  stopTimeline,
  seekTimeline,
  setTimelineLoop,
  getTimelineProgress,
  clearTimeline,
  clearAllTimelines,

  // Progress functions
  getProgress,
  getProgressAsync,

  // Lifecycle
  initProgressListener,
  cleanupProgressListener,
  initEventListeners,
  cleanupEventListeners,
};

export default opPlaybackService;
