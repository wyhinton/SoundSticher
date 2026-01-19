/**
 * Status Publishers
 *
 * This module sets up automatic status publishing for various system events.
 * Import and call initializeStatusPublishers() in your app initialization.
 */

import { listen } from '@tauri-apps/api/event';
import { publishStatus, clearSource } from './status';

/**
 * Initialize all automatic status publishers
 * Call this once during app initialization
 */
export function initializeStatusPublishers() {
  setupBufferingStatusPublisher();
  // Add more publishers here as needed
}

/**
 * Set up automatic status publishing for buffering events
 */
function setupBufferingStatusPublisher() {
  listen<number>('buffering-progress', e => {
    const progress = e.payload;

    if (progress < 100) {
      // Clear previous buffering status and publish new one
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
  });
}

// Add more event-based status publishers as needed
