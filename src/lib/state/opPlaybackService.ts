// Operation Playback Service
//
// Frontend service for the pull-based operation playback system.
// This provides a clean API for building playback graphs and controlling playback.

import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { writable, derived, get, type Readable, type Writable } from 'svelte/store';
import { logger } from './logging';

// ============================================================================
// TYPES
// ============================================================================

/**
 * Request to add an operation to the playback graph
 */
export interface AddOpRequest {
  /** Unique name for this operation */
  name: string;
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
}

/**
 * Response after adding an operation
 */
export interface AddOpResponse {
  name: string;
  opId: number;
  durationSeconds: number;
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
 * Response after building a graph
 */
export interface BuildGraphResponse {
  operationCount: number;
  totalDurationSeconds: number;
  sampleRate: number;
  channels: number;
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
// EVENT LISTENER
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

    internalState.update(s => ({
      ...s,
      progress,
      positionSeconds: progress * s.durationSeconds,
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

// ============================================================================
// API FUNCTIONS
// ============================================================================

/**
 * Build a playback graph from operations
 */
export async function buildGraph(request: BuildGraphRequest): Promise<BuildGraphResponse> {
  logger.opPlayback.info(`Building playback graph with ${request.operations.length} operations`);

  try {
    const response = await invoke<BuildGraphResponse>('op_playback_build_graph', { request });

    internalState.update(s => ({
      ...s,
      hasGraph: true,
      durationSeconds: response.totalDurationSeconds,
      progress: 0,
      positionSeconds: 0,
      isPlaying: false,
      isPaused: false,
    }));

    // Ensure progress listener is initialized
    await initProgressListener();

    logger.opPlayback.success(
      `Graph built: ${response.operationCount} ops, ${response.totalDurationSeconds.toFixed(2)}s duration`
    );

    return response;
  } catch (error) {
    logger.opPlayback.error('Failed to build graph:', error);
    throw error;
  }
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
  const { gap = 0 } = options;

  // For now, we estimate duration from file - in practice the backend will determine actual duration
  // We'll schedule operations sequentially with estimated 5 second durations as placeholder
  // The backend will use actual durations

  let currentTime = 0;
  const operations: AddOpRequest[] = filePaths.map((filePath, index) => {
    const op: AddOpRequest = {
      name: `file_${index}`,
      filePath,
      startTime: currentTime,
      gain: 1.0,
    };
    // Estimate 5 seconds per file for now - backend will override with actual
    currentTime += 5 + gap;
    return op;
  });

  return buildGraph({
    operations,
    sampleRate: options.sampleRate,
    channels: options.channels,
    loopPlayback: options.loopPlayback,
  });
}

/**
 * Start playback
 */
export async function play(startSeconds?: number): Promise<void> {
  logger.opPlayback.info(
    `Starting playback${startSeconds !== undefined ? ` at ${startSeconds.toFixed(2)}s` : ''}`
  );

  try {
    await invoke('op_playback_play', { startSeconds: startSeconds ?? null });

    internalState.update(s => ({
      ...s,
      isPlaying: true,
      isPaused: false,
    }));

    logger.opPlayback.success('Playback started');
  } catch (error) {
    logger.opPlayback.error('Failed to start playback:', error);
    throw error;
  }
}

/**
 * Pause playback
 */
export async function pause(): Promise<void> {
  logger.opPlayback.info('Pausing playback');

  try {
    await invoke('op_playback_pause');

    internalState.update(s => ({
      ...s,
      isPaused: true,
    }));

    logger.opPlayback.success('Playback paused');
  } catch (error) {
    logger.opPlayback.error('Failed to pause playback:', error);
    throw error;
  }
}

/**
 * Resume playback
 */
export async function resume(): Promise<void> {
  logger.opPlayback.info('Resuming playback');

  try {
    await invoke('op_playback_resume');

    internalState.update(s => ({
      ...s,
      isPaused: false,
    }));

    logger.opPlayback.success('Playback resumed');
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
    await invoke('op_playback_stop');

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
    await invoke('op_playback_seek', { positionSeconds });

    const state = get(internalState);
    const progress = state.durationSeconds > 0 ? positionSeconds / state.durationSeconds : 0;

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
    await invoke('op_playback_set_volume', { volume });

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
    await invoke('op_playback_set_loop', { loopPlayback: enabled });

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
  return invoke<number>('op_playback_get_progress');
}

/**
 * Clear the current playback graph
 */
export async function clearGraph(): Promise<void> {
  logger.opPlayback.info('Clearing playback graph');

  try {
    await invoke('op_playback_clear_graph');

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

  if (state.isPlaying && !state.isPaused) {
    await pause();
  } else if (state.isPaused) {
    await resume();
  } else {
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
};

export default opPlaybackService;
