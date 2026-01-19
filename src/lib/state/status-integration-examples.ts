/**
 * Status Integration Examples
 *
 * This file demonstrates how to integrate the status system into various parts of the application.
 * Use these patterns when converting operations to publish status updates.
 */

import { publishStatus, clearStatus, clearSource } from './status';

// ============================================================================
// Example 1: Export Operation
// ============================================================================

export function startExport() {
  // Clear any previous export statuses
  clearSource('export');

  // Publish initial status
  publishStatus({
    source: 'export',
    level: 'working',
    message: 'Preparing export...',
    progress: 0,
  });
}

export function updateExportProgress(progress: number, message?: string) {
  // Clear previous export statuses
  clearSource('export');

  publishStatus({
    source: 'export',
    level: 'working',
    message: message || `Exporting audio... ${(progress * 100).toFixed(0)}%`,
    progress,
  });
}

export function completeExport(outputPath: string) {
  clearSource('export');

  publishStatus({
    source: 'export',
    level: 'success',
    message: `Export completed: ${outputPath}`,
    sticky: true, // Stays until user does something else
  });
}

export function failExport(error: string) {
  clearSource('export');

  publishStatus({
    source: 'export',
    level: 'error',
    message: `Export failed: ${error}`,
    sticky: true,
  });
}

// ============================================================================
// Example 2: Playback Status
// ============================================================================

export function publishPlaybackStatus(isPlaying: boolean, isPaused: boolean = false) {
  clearSource('playback');

  if (isPlaying) {
    publishStatus({
      source: 'playback',
      level: 'info',
      message: 'Playing',
    });
  } else if (isPaused) {
    publishStatus({
      source: 'playback',
      level: 'info',
      message: 'Paused',
    });
  } else {
    // Playback stopped, clear the status
    clearSource('playback');
  }
}

// ============================================================================
// Example 3: Audio Processing/Combining
// ============================================================================

export function startAudioProcessing() {
  clearSource('processing');

  publishStatus({
    source: 'processing',
    level: 'working',
    message: 'Processing audio...',
  });
}

export function updateProcessingProgress(progress: number) {
  clearSource('processing');

  publishStatus({
    source: 'processing',
    level: 'working',
    message: 'Processing audio...',
    progress,
  });
}

export function completeProcessing() {
  clearSource('processing');

  publishStatus({
    source: 'processing',
    level: 'success',
    message: 'Processing complete',
  });

  // Auto-clear success message after 3 seconds
  setTimeout(() => clearSource('processing'), 3000);
}

// ============================================================================
// Example 4: Buffering Status (from Tauri events)
// ============================================================================

export function updateBufferingStatus(progress: number) {
  if (progress < 100) {
    clearSource('buffering');
    publishStatus({
      source: 'buffering',
      level: 'working',
      message: `Buffering... ${progress.toFixed(1)}%`,
      progress: progress / 100,
    });
  } else {
    // Buffering complete, clear the status
    clearSource('buffering');
  }
}

// ============================================================================
// Example 5: File Loading
// ============================================================================

export function startFileLoading(filename: string) {
  publishStatus({
    source: 'file-loading',
    level: 'working',
    message: `Loading ${filename}...`,
  });
}

export function completeFileLoading(filename: string) {
  clearSource('file-loading');

  publishStatus({
    source: 'file-loading',
    level: 'success',
    message: `Loaded ${filename}`,
  });

  // Auto-clear after 2 seconds
  setTimeout(() => clearSource('file-loading'), 2000);
}

export function failFileLoading(filename: string, error: string) {
  clearSource('file-loading');

  publishStatus({
    source: 'file-loading',
    level: 'error',
    message: `Failed to load ${filename}: ${error}`,
    sticky: true,
  });
}

// ============================================================================
// Integration Points in Existing Code
// ============================================================================

/**
 * Where to add status publishing in your existing code:
 *
 * 1. In Export.svelte or export handler:
 *    - Call startExport() when export begins
 *    - Call updateExportProgress() on each progress update
 *    - Call completeExport() on success
 *    - Call failExport() on error
 *
 * 2. In playback state updates (state.svelte.ts):
 *    - Call publishPlaybackStatus() when playingCombined changes
 *
 * 3. In audio combining logic:
 *    - Call startAudioProcessing() when combining starts
 *    - Call updateProcessingProgress() during processing
 *    - Call completeProcessing() when done
 *
 * 4. In Tauri event listeners:
 *    - Replace direct statusMessage updates with publishStatus calls
 *    - Call updateBufferingStatus() for buffering events
 *
 * 5. General pattern for any async operation:
 *    - Publish 'working' status at start
 *    - Update with progress if available
 *    - Publish 'success' or 'error' at completion
 *    - Clear source when appropriate
 */
