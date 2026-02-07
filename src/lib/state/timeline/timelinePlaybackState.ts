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
