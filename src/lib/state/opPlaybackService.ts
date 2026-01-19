import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { writable, derived, get, type Readable, type Writable } from 'svelte/store';
import { logger } from './logging';
import { invokeWithPerf } from './performance';
import { buildGraph as buildGraphInternal, buildGraphFromFiles as buildGraphFromFilesInternal } from './playbackGraphUtils';

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
 * Build graph event types from Rust backend
 */
export interface OpPlaybackBuildGraphEvent {
  event: 'started' | 'progress' | 'finished';
  data:
    | { operationCount: number } // started
    | {
        operationName: string;
        operationIndex: number;
        totalOperations: number;
        durationSeconds: number;
      } // progress
    | {
        operationCount: number;
        totalDurationSeconds: number;
        sampleRate: number;
        channels: number;
      }; // finished
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
 * Initialize the progress event listener
 */
async function initProgressListener(): Promise<void> {
  if (progressUnlisten) return;

  progressUnlisten = await listen<number>('op-timeline-progress', event => {
    const progress = event.payload;
    const state = get(internalState);
    const newPositionSeconds = progress * state.durationSeconds;

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
  });

  logger.opPlayback.info('Progress listener initialized');
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
 * Build a playback graph from operations
 */
export async function buildGraph(request: BuildGraphRequest): Promise<BuildGraphResponse> {
  // Ensure progress listener is initialized
  await initProgressListener();
  
  // Use the utility function with state updater
  const result = await buildGraphInternal(request, internalState.update);
  
  return result;
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
 * Start playback
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
    const result = await invokeWithPerf('op_playback_play', { startSeconds: actualStartSeconds });

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
 * Seek to a position
 */
export async function seek(positionSeconds: number): Promise<void> {
  logger.opPlayback.info(`Seeking to ${positionSeconds.toFixed(2)}s`);

  try {
    const result = await invokeWithPerf('op_playback_seek', { positionSeconds });

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
 * Set loop mode
 */
export async function setLoop(enabled: boolean): Promise<void> {
  logger.opPlayback.info(`Setting loop mode to ${enabled}`);

  try {
    const result = await invokeWithPerf('op_playback_set_loop', { loopPlayback: enabled });

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

  // Control functions
  buildGraph,
  buildGraphFromFiles,
  play,
  pause,
  resume,
  stop,
  seek,
  seekToProgress,
  setVolume,
  setLoop,
  togglePlayPause,
  clearGraph,

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
