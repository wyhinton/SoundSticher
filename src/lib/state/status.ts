import { writable, derived } from 'svelte/store';

// Status levels with semantic meaning
export type StatusLevel = 'idle' | 'info' | 'working' | 'success' | 'warning' | 'error';

// Status event model
export type StatusEvent = {
  id?: string;
  message: string;
  level?: StatusLevel;
  progress?: number; // 0–1
  sticky?: boolean; // stays until cleared
  timestamp?: number;
  source?: string;
};

// Internal store for all status events
const statuses = writable<StatusEvent[]>([]);

/**
 * Publish a new status event to the status system
 * @param status - The status event to publish
 */
export const publishStatus = (status: StatusEvent) => {
  statuses.update(list => [...list, { timestamp: Date.now(), level: 'info', ...status }]);
};

/**
 * Clear status events from the system
 * @param predicate - Optional filter function; if not provided, clears all
 */
export const clearStatus = (predicate?: (s: StatusEvent) => boolean) => {
  statuses.update(list => (predicate ? list.filter(s => !predicate(s)) : []));
};

/**
 * Clear non-sticky statuses (useful for cleanup)
 */
export const clearTransient = () => {
  clearStatus(s => !s.sticky);
};

/**
 * Clear statuses from a specific source
 */
export const clearSource = (source: string) => {
  clearStatus(s => s.source === source);
};

// Priority order for status resolution (highest to lowest)
const priority: StatusLevel[] = ['error', 'warning', 'working', 'success', 'info', 'idle'];

/**
 * Derived store that resolves the currently active status
 * based on priority and timestamp.
 * Always returns a status (never undefined).
 */
export const activeStatus = derived(statuses, ($statuses): StatusEvent => {
  if ($statuses.length === 0) {
    return { message: 'Ready', level: 'idle' as StatusLevel, timestamp: Date.now() };
  }

  // Sort by priority first, then by most recent timestamp
  const sorted = [...$statuses].sort(
    (a, b) => priority.indexOf(a.level!) - priority.indexOf(b.level!) || b.timestamp! - a.timestamp!
  );

  return sorted[0]!;
});

/**
 * Export the underlying statuses store for advanced use cases
 * (e.g., displaying all statuses, filtering by source, etc.)
 */
export const allStatuses = derived(statuses, $statuses => $statuses);
