/**
 * Timeline Playback Service
 *
 * This module provides a timeline-aware playback API that uses the new
 * architecture with TimelinePlaybackManager on the Rust side.
 *
 * Key concepts:
 * - TimelineSource: What kind of audio source (Operation, AudioFile, LiveInput)
 * - Timeline: A named playback session with a source
 * - This service delegates to the appropriate backend based on source type
 */

import { writable, derived, get, type Readable } from 'svelte/store';
import { logger } from './logging';
import type { BuildOpPlaybackGraphRequest, BuildGraphResponse } from './opPlaybackService';
import { invokeWithPerf } from './performance';
import { timelinePlaybackState as perTimelinePlaybackState } from './timeline/timelinePlaybackState';
import { getActiveTimelineId } from './timeline/timelines';
import { createTypedEventChannelWithLoggingAndStatusMessages } from '$lib/utils/channelMaker';
// ============================================================================
// TYPES
// ============================================================================

/**
 * Timeline source types - determines how audio is produced
 */
export type TimelineSource =
  | {
      type: 'operation';
      /** The build request for the operation graph */
      request: BuildOpPlaybackGraphRequest;
    }
  | {
      type: 'audioFile';
      /** Path to the audio file */
      filePath: string;
    }
  | {
      type: 'liveInput';
      /** Audio device ID */
      deviceId: string;
    };

/**
 * Events emitted during timeline playback build
 */
export interface TimelinePlaybackEvent {
  event: 'buildStarted' | 'buildProgress' | 'buildFinished' | 'buildError';
  data:
    | {
        timelineId: string;
        operationCount: number;
      }
    | {
        timelineId: string;
        operationName: string;
        operationIndex: number;
        totalOperations: number;
        durationSeconds: number;
      }
    | {
        timelineId: string;
        operationCount: number;
        totalDurationSeconds: number;
        sampleRate: number;
        channels: number;
      }
    | {
        timelineId: string;
        error: string;
      };
}

/**
 * Timeline playback state
 */
export interface TimelinePlaybackState {
  /** Currently active timeline ID (only one plays at a time) */
  activeTimelineId: string | null;
  /** Whether audio is currently playing */
  isPlaying: boolean;
  /** Whether playback is paused */
  isPaused: boolean;
  /** Currently building timeline ID (if any) */
  buildingTimelineId: string | null;
}

// ============================================================================
// STORES
// ============================================================================

const internalState = writable<TimelinePlaybackState>({
  activeTimelineId: null,
  isPlaying: false,
  isPaused: false,
  buildingTimelineId: null,
});

export const timelinePlaybackState: Readable<TimelinePlaybackState> = derived(
  internalState,
  $state => $state
);

export const activeTimelineId: Readable<string | null> = derived(
  internalState,
  $state => $state.activeTimelineId
);

export const isTimelinePlaying: Readable<boolean> = derived(
  internalState,
  $state => $state.isPlaying
);

export const isTimelinePaused: Readable<boolean> = derived(
  internalState,
  $state => $state.isPaused
);

// ============================================================================
// API FUNCTIONS
// ============================================================================

/**
 * Build a timeline from a source
 *
 * This is the main entry point for building timeline playback.
 * The backend will use the appropriate builder based on the source type.
 */
export async function buildTimeline(
  timelineId: string,
  source: TimelineSource
): Promise<BuildGraphResponse> {
  logger.opPlayback.info(`Building timeline '${timelineId}' from source type: ${source.type}`);

  let buildStartTime: number;

  try {
    internalState.update(s => ({
      ...s,
      buildingTimelineId: timelineId,
    }));

    const onBuildEvent = createTypedEventChannelWithLoggingAndStatusMessages<TimelinePlaybackEvent>(
      'BuildTimeline',
      {
        source: `timeline-build-${timelineId}`,
        startedMessage: data =>
          `Building timeline '${timelineId}' (${(data as any).operationCount} operations)...`,
        progressMessage: data => {
          const progressData = data as any;
          return `Building: ${progressData.operationName} (${progressData.operationIndex + 1}/${progressData.totalOperations})`;
        },
        finishedMessage: data => {
          const buildTimeMs = Date.now() - buildStartTime;
          const buildTimeSec = (buildTimeMs / 1000).toFixed(2);
          const finishedData = data as any;
          const audioDuration = finishedData.totalDurationSeconds.toFixed(1);
          return `Built ${finishedData.operationCount} ops in ${buildTimeSec}s → ${audioDuration}s audio`;
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
        onStarted: () => {
          buildStartTime = Date.now();
          logger.opPlayback.info(`Started building timeline '${timelineId}'`);
        },
        onProgress: data => {
          const progressData = data as any;
          const buildProgress = (progressData.operationIndex + 1) / progressData.totalOperations;
          logger.opPlayback.info(
            `Building timeline '${timelineId}': ${progressData.operationName} (${(buildProgress * 100).toFixed(1)}%)`
          );
        },
        onFinished: () => {
          logger.opPlayback.info(`Finished building timeline '${timelineId}'`);
        },
      }
    );

    const result = await invokeWithPerf<BuildGraphResponse>('timeline_build_playback', {
      timelineId,
      source,
      onEvent: onBuildEvent,
    });

    if (!result.ok) {
      throw new Error(`Failed to build timeline '${timelineId}': ${result.error.message}`);
    }

    internalState.update(s => ({
      ...s,
      buildingTimelineId: null,
    }));

    logger.opPlayback.success(
      `Timeline '${timelineId}' built: ${result.value.operationCount} operations, ${result.value.totalDurationSeconds.toFixed(2)}s`
    );

    return result.value;
  } catch (error) {
    internalState.update(s => ({
      ...s,
      buildingTimelineId: null,
    }));

    logger.opPlayback.error(`Failed to build timeline '${timelineId}':`, error);
    throw error;
  }
}

/**
 * Build a timeline from a BuildOpPlaybackGraphRequest (convenience wrapper)
 *
 * This is for the common case of building from an operation graph.
 */
export async function buildTimelineFromRequest(
  timelineId: string,
  request: BuildOpPlaybackGraphRequest
): Promise<BuildGraphResponse> {
  return buildTimeline(timelineId, {
    type: 'operation',
    request,
  });
}

/**
 * Play a timeline
 */
export async function playTimeline(timelineId: string, startSeconds?: number): Promise<void> {
  logger.opPlayback.info(
    `Playing timeline '${timelineId}' from ${startSeconds?.toFixed(2) ?? 'current position'}s`
  );

  try {
    const result = await invokeWithPerf('timeline_play', {
      timelineId,
      startSeconds,
    });

    if (!result.ok) {
      throw new Error(`Failed to play timeline '${timelineId}': ${result.error.message}`);
    }

    internalState.update(s => ({
      ...s,
      activeTimelineId: timelineId,
      isPlaying: true,
      isPaused: false,
    }));

    // Also update the per-timeline playback state so the play button disables
    perTimelinePlaybackState.update(state => ({
      ...state,
      [timelineId]: {
        playheadTime: state[timelineId]?.playheadTime ?? 0,
        looping: state[timelineId]?.looping ?? false,
        isPlaying: true,
      },
    }));

    logger.opPlayback.success(`Timeline '${timelineId}' playing`);
  } catch (error) {
    logger.opPlayback.error(`Failed to play timeline '${timelineId}':`, error);
    throw error;
  }
}

/**
 * Pause the specified timeline
 */
export async function pauseTimeline(timelineId: string): Promise<void> {
  logger.opPlayback.info(`Pausing timeline '${timelineId}'`);

  try {
    const result = await invokeWithPerf('timeline_pause', {
      timelineId,
    });

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
 * Resume the specified timeline
 */
export async function resumeTimeline(timelineId: string): Promise<void> {
  logger.opPlayback.info(`Resuming timeline '${timelineId}'`);

  try {
    const result = await invokeWithPerf('timeline_resume', {
      timelineId,
    });

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
 * Stop playback completely
 */
export async function stopTimeline(timelineId: string): Promise<void> {
  logger.opPlayback.info(`Stopping timeline '${timelineId}'`);

  try {
    const result = await invokeWithPerf('timeline_stop', {
      timelineId,
    });

    if (!result.ok) {
      throw new Error(`Failed to stop timeline '${timelineId}': ${result.error.message}`);
    }

    internalState.update(s => ({
      ...s,
      activeTimelineId: null,
      isPlaying: false,
      isPaused: false,
    }));

    logger.opPlayback.success(`Timeline '${timelineId}' stopped`);
  } catch (error) {
    logger.opPlayback.error(`Failed to stop timeline '${timelineId}':`, error);
    throw error;
  }
}

/**
 * Seek to a position in a timeline
 */
export async function seekTimeline(timelineId: string, positionSeconds: number): Promise<void> {
  logger.opPlayback.info(`Seeking timeline '${timelineId}' to ${positionSeconds.toFixed(2)}s`);

  try {
    const result = await invokeWithPerf('timeline_seek', {
      timelineId,
      positionSeconds,
    });

    if (!result.ok) {
      throw new Error(`Failed to seek timeline '${timelineId}': ${result.error.message}`);
    }

    logger.opPlayback.success(`Timeline '${timelineId}' seeked to ${positionSeconds.toFixed(2)}s`);
  } catch (error) {
    logger.opPlayback.error(`Failed to seek timeline '${timelineId}':`, error);
    throw error;
  }
}

/**
 * Set loop mode for a timeline
 */
export async function setTimelineLoop(timelineId: string, enabled: boolean): Promise<void> {
  logger.opPlayback.info(`Setting loop mode to ${enabled} for timeline '${timelineId}'`);

  try {
    const result = await invokeWithPerf('timeline_set_loop', {
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
 * Set playback volume
 */
export async function setVolume(volume: number): Promise<void> {
  logger.opPlayback.info(`Setting volume to ${(volume * 100).toFixed(0)}%`);

  try {
    const result = await invokeWithPerf('timeline_set_volume', { volume });

    if (!result.ok) {
      throw new Error(`Failed to set volume: ${result.error.message}`);
    }

    logger.opPlayback.success(`Volume set to ${(volume * 100).toFixed(0)}%`);
  } catch (error) {
    logger.opPlayback.error('Failed to set volume:', error);
    throw error;
  }
}

/**
 * Get progress for a timeline
 */
export async function getTimelineProgress(timelineId: string): Promise<number> {
  const result = await invokeWithPerf<number>('timeline_get_progress', { timelineId });

  if (!result.ok) {
    throw new Error(`Failed to get progress for timeline '${timelineId}': ${result.error.message}`);
  }

  return result.value;
}

/**
 * Clear a specific timeline
 */
export async function clearTimeline(timelineId: string): Promise<void> {
  logger.opPlayback.info(`Clearing timeline '${timelineId}'`);

  try {
    const result = await invokeWithPerf('timeline_clear', { timelineId });

    if (!result.ok) {
      throw new Error(`Failed to clear timeline '${timelineId}': ${result.error.message}`);
    }

    // If this was the active timeline, clear state
    const state = get(internalState);
    if (state.activeTimelineId === timelineId) {
      internalState.update(s => ({
        ...s,
        activeTimelineId: null,
        isPlaying: false,
        isPaused: false,
      }));
    }

    logger.opPlayback.success(`Timeline '${timelineId}' cleared`);
  } catch (error) {
    logger.opPlayback.error(`Failed to clear timeline '${timelineId}':`, error);
    throw error;
  }
}

/**
 * Clear all timelines
 */
export async function clearAllTimelines(): Promise<void> {
  logger.opPlayback.info('Clearing all timelines');

  try {
    const result = await invokeWithPerf('timeline_clear_all');

    if (!result.ok) {
      throw new Error(`Failed to clear all timelines: ${result.error.message}`);
    }

    internalState.update(s => ({
      ...s,
      activeTimelineId: null,
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
 * Toggle play/pause for the active timeline
 *
 * This function:
 * - Gets the active timeline from timelinesStore
 * - Checks the current playback state
 * - Plays, pauses, or resumes as appropriate
 * - Returns early if no timeline is active
 */
export async function togglePlayPauseActiveTimeline(): Promise<void> {
  const activeTimelineId = getActiveTimelineId();
  if (!activeTimelineId) {
    logger.opPlayback.warning('No active timeline to toggle play/pause');
    return;
  }

  const state = get(internalState);

  // If this timeline is currently playing and not paused, pause it
  if (state.isPlaying && !state.isPaused && state.activeTimelineId === activeTimelineId) {
    await pauseTimeline(activeTimelineId);
  }
  // If this timeline is currently paused, resume it
  else if (state.isPaused && state.activeTimelineId === activeTimelineId) {
    await resumeTimeline(activeTimelineId);
  }
  // Otherwise, start playing this timeline
  else {
    await playTimeline(activeTimelineId);
  }
}

// ============================================================================
// NAMESPACE EXPORT
// ============================================================================

/**
 * Timeline Playback Service - the new architecture API
 */
export const timelinePlaybackService = {
  // State stores
  state: timelinePlaybackState,
  activeTimelineId,
  isPlaying: isTimelinePlaying,
  isPaused: isTimelinePaused,

  // Build functions
  buildTimeline,
  buildTimelineFromRequest,

  // Transport controls
  playTimeline,
  pauseTimeline,
  resumeTimeline,
  stopTimeline,
  seekTimeline,
  togglePlayPauseActiveTimeline,

  // Settings
  setTimelineLoop,
  setVolume,

  // Progress
  getTimelineProgress,

  // Cleanup
  clearTimeline,
  clearAllTimelines,
};

export default timelinePlaybackService;
