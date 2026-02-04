/**
 * Timeline Playback Service - Per-timeline playback management
 *
 * This service provides playback functionality scoped to individual timelines,
 * allowing multiple timelines to have independent playheads and transport controls.
 */

import { derived, get, type Readable } from 'svelte/store';
import { playbackStore, type TimelineId, type TimelinePlayback } from './timelines';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export interface TimelinePlaybackService {
  // Derived stores for reactive access
  progress: Readable<number>;
  currentTime: Readable<number>;
  duration: Readable<number>;
  isPlaying: Readable<boolean>;
  
  // Full playback state
  playback: Readable<TimelinePlayback>;
  
  // Transport actions
  play: () => Promise<void>;
  pause: () => Promise<void>;
  stop: () => Promise<void>;
  seek: (time: number) => Promise<void>;
  seekToProgress: (progress: number) => Promise<void>;
  
  // State setters
  setProgress: (progress: number) => void;
  setCurrentTime: (time: number) => void;
  setDuration: (duration: number) => void;
  setPlaying: (isPlaying: boolean) => void;
  
  // Progress listener
  initProgressListener: () => Promise<void>;
  cleanupProgressListener: () => void;
  
  // Getters
  getProgress: () => number;
  getCurrentTime: () => number;
  getDuration: () => number;
  getIsPlaying: () => boolean;
}

/**
 * Create a playback service scoped to a specific timeline
 */
export function createTimelinePlaybackService(
  timelineId: TimelineId,
  options: {
    useBackend?: boolean; // Whether to use Tauri backend for playback
    backendPrefix?: string; // Prefix for backend commands (e.g., 'op_playback')
  } = {}
): TimelinePlaybackService {
  const { useBackend = true, backendPrefix = 'op_playback' } = options;
  
  // Initialize playback for this timeline
  playbackStore.init(timelineId);
  
  // Progress listener cleanup
  let progressUnlisten: UnlistenFn | null = null;

  // Create derived stores
  const playback = playbackStore.forTimeline(timelineId);
  
  const progress = derived(playback, ($p) => $p.progress);
  const currentTime = derived(playback, ($p) => $p.currentTime);
  const duration = derived(playback, ($p) => $p.duration);
  const isPlaying = derived(playback, ($p) => $p.isPlaying);

  return {
    playback,
    progress,
    currentTime,
    duration,
    isPlaying,

    async play() {
      if (useBackend) {
        try {
          await invoke(`${backendPrefix}_play`);
          playbackStore.setPlaying(timelineId, true);
        } catch (err) {
          console.error(`[TimelinePlayback:${timelineId}] Failed to play:`, err);
          throw err;
        }
      } else {
        playbackStore.setPlaying(timelineId, true);
      }
    },

    async pause() {
      if (useBackend) {
        try {
          await invoke(`${backendPrefix}_pause`);
          playbackStore.setPlaying(timelineId, false);
        } catch (err) {
          console.error(`[TimelinePlayback:${timelineId}] Failed to pause:`, err);
          throw err;
        }
      } else {
        playbackStore.setPlaying(timelineId, false);
      }
    },

    async stop() {
      if (useBackend) {
        try {
          await invoke(`${backendPrefix}_stop`);
          playbackStore.setPlaying(timelineId, false);
          playbackStore.setCurrentTime(timelineId, 0);
        } catch (err) {
          console.error(`[TimelinePlayback:${timelineId}] Failed to stop:`, err);
          throw err;
        }
      } else {
        playbackStore.setPlaying(timelineId, false);
        playbackStore.setCurrentTime(timelineId, 0);
      }
    },

    async seek(time: number) {
      const currentDuration = playbackStore.get(timelineId).duration;
      const clampedTime = Math.max(0, Math.min(time, currentDuration));
      
      if (useBackend) {
        try {
          await invoke(`${backendPrefix}_seek`, { position: clampedTime });
          playbackStore.setCurrentTime(timelineId, clampedTime);
        } catch (err) {
          console.error(`[TimelinePlayback:${timelineId}] Failed to seek:`, err);
          throw err;
        }
      } else {
        playbackStore.setCurrentTime(timelineId, clampedTime);
      }
    },

    async seekToProgress(progressValue: number) {
      const currentDuration = playbackStore.get(timelineId).duration;
      const time = progressValue * currentDuration;
      await this.seek(time);
    },

    setProgress(progressValue: number) {
      playbackStore.setProgress(timelineId, progressValue);
    },

    setCurrentTime(time: number) {
      playbackStore.setCurrentTime(timelineId, time);
    },

    setDuration(durationValue: number) {
      playbackStore.setDuration(timelineId, durationValue);
    },

    setPlaying(isPlayingValue: boolean) {
      playbackStore.setPlaying(timelineId, isPlayingValue);
    },

    async initProgressListener() {
      if (!useBackend) return;
      
      // Clean up any existing listener
      this.cleanupProgressListener();
      
      try {
        // Listen for progress events from backend
        // The event name could be customized per timeline if needed
        progressUnlisten = await listen<{ progress: number; position: number }>(
          `${backendPrefix}_progress`,
          (event) => {
            const { progress: progressValue, position } = event.payload;
            playbackStore.update(timelineId, {
              progress: progressValue,
              currentTime: position,
            });
          }
        );
        
        console.log(`[TimelinePlayback:${timelineId}] Progress listener initialized`);
      } catch (err) {
        console.error(`[TimelinePlayback:${timelineId}] Failed to init progress listener:`, err);
      }
    },

    cleanupProgressListener() {
      if (progressUnlisten) {
        progressUnlisten();
        progressUnlisten = null;
        console.log(`[TimelinePlayback:${timelineId}] Progress listener cleaned up`);
      }
    },

    getProgress() {
      return playbackStore.get(timelineId).progress;
    },

    getCurrentTime() {
      return playbackStore.get(timelineId).currentTime;
    },

    getDuration() {
      return playbackStore.get(timelineId).duration;
    },

    getIsPlaying() {
      return playbackStore.get(timelineId).isPlaying;
    },
  };
}

/**
 * Factory for creating timeline playback services
 */
export const timelinePlaybackFactory = {
  services: new Map<TimelineId, TimelinePlaybackService>(),

  /**
   * Get or create a playback service for a timeline
   */
  forTimeline(
    timelineId: TimelineId,
    options?: {
      useBackend?: boolean;
      backendPrefix?: string;
    }
  ): TimelinePlaybackService {
    if (!this.services.has(timelineId)) {
      this.services.set(timelineId, createTimelinePlaybackService(timelineId, options));
    }
    return this.services.get(timelineId)!;
  },

  /**
   * Remove a playback service
   */
  remove(timelineId: TimelineId): void {
    const service = this.services.get(timelineId);
    if (service) {
      service.cleanupProgressListener();
    }
    this.services.delete(timelineId);
    playbackStore.remove(timelineId);
  },

  /**
   * Clear all playback services
   */
  clear(): void {
    for (const service of this.services.values()) {
      service.cleanupProgressListener();
    }
    this.services.clear();
  },
};
