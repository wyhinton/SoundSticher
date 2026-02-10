import { derived, writable } from 'svelte/store';
import { TimelineId } from './timelines';

export interface TimelinePlaybackState {
  playheadTime: number;
  looping: boolean;
  isPlaying: boolean;
}

/**
 * Non-persisted runtime store for playhead position tracking
 * This is NOT persisted to avoid waveform churn and serialization overhead
 * Playhead position updates every frame during playback, so persisting would be wasteful
 */
export const timelinePlaybackState = writable<Record<TimelineId, TimelinePlaybackState>>({});

/**
 * Service class for managing timeline playback state operations
 */
export class TimelinePlaybackStoreService {
  /**
   * Add a new timeline with default playback state
   */
  addTimeline(timelineId: TimelineId, initialState?: Partial<TimelinePlaybackState>) {
    timelinePlaybackState.update(state => ({
      ...state,
      [timelineId]: {
        playheadTime: 0,
        looping: true,
        isPlaying: false,
        ...initialState,
      },
    }));
  }

  /**
   * Remove a timeline from playback state
   */
  removeTimeline(timelineId: TimelineId) {
    timelinePlaybackState.update(state => {
      const newState = { ...state };
      delete newState[timelineId];
      return newState;
    });
  }

  /**
   * Update playhead time for a specific timeline
   */
  setPlayheadTime(timelineId: TimelineId, time: number) {
    timelinePlaybackState.update(state => {
      const currentState = state[timelineId] || {
        playheadTime: 0,
        looping: false,
        isPlaying: false,
      };
      return {
        ...state,
        [timelineId]: {
          ...currentState,
          playheadTime: time,
        },
      };
    });
  }

  /**
   * Toggle looping state for a specific timeline
   */
  toggleLooping(timelineId: TimelineId) {
    timelinePlaybackState.update(state => {
      const currentState = state[timelineId] || {
        playheadTime: 0,
        looping: false,
        isPlaying: false,
      };
      return {
        ...state,
        [timelineId]: {
          ...currentState,
          looping: !currentState.looping,
        },
      };
    });
  }

  /**
   * Set looping state for a specific timeline
   */
  setLooping(timelineId: TimelineId, looping: boolean) {
    timelinePlaybackState.update(state => {
      const currentState = state[timelineId] || {
        playheadTime: 0,
        looping: false,
        isPlaying: false,
      };
      return {
        ...state,
        [timelineId]: {
          ...currentState,
          looping,
        },
      };
    });
  }

  /**
   * Start playback for a specific timeline
   */
  startPlayback(timelineId: TimelineId) {
    timelinePlaybackState.update(state => {
      const currentState = state[timelineId] || {
        playheadTime: 0,
        looping: false,
        isPlaying: false,
      };
      return {
        ...state,
        [timelineId]: {
          ...currentState,
          isPlaying: true,
        },
      };
    });
  }

  /**
   * Stop playback for a specific timeline
   */
  stopPlayback(timelineId: TimelineId) {
    timelinePlaybackState.update(state => {
      const currentState = state[timelineId] || {
        playheadTime: 0,
        looping: false,
        isPlaying: false,
      };
      return {
        ...state,
        [timelineId]: {
          ...currentState,
          isPlaying: false,
        },
      };
    });
  }

  /**
   * Toggle playback state for a specific timeline
   */
  togglePlayback(timelineId: TimelineId) {
    timelinePlaybackState.update(state => {
      const currentState = state[timelineId] || {
        playheadTime: 0,
        looping: false,
        isPlaying: false,
      };
      return {
        ...state,
        [timelineId]: {
          ...currentState,
          isPlaying: !currentState.isPlaying,
        },
      };
    });
  }

  /**
   * Stop all timelines
   */
  stopAllTimelines() {
    timelinePlaybackState.update(state => {
      const newState: Record<TimelineId, TimelinePlaybackState> = {};
      Object.entries(state).forEach(([timelineId, currentState]) => {
        newState[timelineId as TimelineId] = {
          ...currentState,
          isPlaying: false,
        };
      });
      return newState;
    });
  }

  /**
   * Reset a timeline to default state
   */
  resetTimeline(timelineId: TimelineId) {
    timelinePlaybackState.update(state => ({
      ...state,
      [timelineId]: {
        playheadTime: 0,
        looping: false,
        isPlaying: false,
      },
    }));
  }

  /**
   * Get current playback state for a timeline (snapshot)
   */
  getTimelineState(timelineId: TimelineId): TimelinePlaybackState | null {
    let currentState: TimelinePlaybackState | null = null;
    timelinePlaybackState.subscribe(state => {
      currentState = state[timelineId] || null;
    })();
    return currentState;
  }

  /**
   * Check if a timeline exists in the playback state
   */
  hasTimeline(timelineId: TimelineId): boolean {
    let exists = false;
    timelinePlaybackState.subscribe(state => {
      exists = timelineId in state;
    })();
    return exists;
  }

  /**
   * Get all timeline IDs that are currently tracked
   */
  getAllTimelineIds(): TimelineId[] {
    let ids: TimelineId[] = [];
    timelinePlaybackState.subscribe(state => {
      ids = Object.keys(state) as TimelineId[];
    })();
    return ids;
  }

  /**
   * Clear all timeline playback states
   */
  clearAll() {
    timelinePlaybackState.set({});
  }
}

export const timelinePlaybackStoreService = new TimelinePlaybackStoreService();

/**
 * Derived store to get playhead time for a specific timeline
 * Returns 0 if timeline is not found
 */
export function timelinePlayhead(timelineId: TimelineId) {
  return derived(timelinePlaybackState, $state => $state[timelineId]?.playheadTime ?? 0);
}

/**
 * Derived store to get looping state for a specific timeline
 * Returns false if timeline is not found
 */
export function timelineLooping(timelineId: TimelineId) {
  return derived(timelinePlaybackState, $state => $state[timelineId]?.looping ?? false);
}

/**
 * Derived store to get playing state for a specific timeline
 * Returns false if timeline is not found
 */
export function timelineIsPlaying(timelineId: TimelineId) {
  return derived(timelinePlaybackState, $state => $state[timelineId]?.isPlaying ?? false);
}
