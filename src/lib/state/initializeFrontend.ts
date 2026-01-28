import { initWaveformService } from './waveformCache';
import { initializeGroupsSubscription } from './groups';
import { initializeOperationsSubscription } from './operation';
import { initializeStatusPublishers } from './status-publishers';
import { undo, redo, canUndo, canRedo } from './undo/undo';
import { opPlaybackService } from './opPlaybackService';
import { initializeAutoRenderSubscription } from './autoRender';

/**
 * Initialize all frontend systems and services
 * Called once on application mount
 *
 * @returns Cleanup function to call on application destroy
 */
export function initializeFrontend(): () => void {
  // Initialize subscriptions to avoid circular dependency issues
  initializeGroupsSubscription();
  initializeOperationsSubscription();

  // Initialize render of ops with auto render policy after rev is bumped
  initializeAutoRenderSubscription();
  // Initialize automatic status publishers (buffering, etc.)
  initializeStatusPublishers();

  // Initialize waveform service (handles loading waveforms when operation changes)
  const cleanupWaveformService = initWaveformService();

  // Setup keyboard shortcuts
  const handleKeyPress = (ev: KeyboardEvent) => {
    // Handle spacebar for play/pause
    if (ev.code === 'Space' && !ev.shiftKey && !ev.ctrlKey && !ev.metaKey) {
      // Only handle spacebar if not focused on an input element
      if (ev.target instanceof HTMLInputElement || ev.target instanceof HTMLTextAreaElement) {
        return;
      }

      ev.preventDefault(); // Prevent default scrolling
      // Use the operation playback service
      opPlaybackService.togglePlayPause().catch((err: Error) => {
        console.error('Error toggling playback:', err);
      });
      return;
    }

    // Handle undo/redo shortcuts
    if ((ev.ctrlKey || ev.metaKey) && !ev.altKey) {
      if (ev.key === 'z' && !ev.shiftKey) {
        // Ctrl+Z or Cmd+Z for undo
        ev.preventDefault();
        if (canUndo()) {
          undo();
          console.log('🔄 Undo triggered via keyboard shortcut');
        }
        return;
      }

      if (ev.key === 'y' || (ev.key === 'z' && ev.shiftKey)) {
        // Ctrl+Y or Ctrl+Shift+Z or Cmd+Y or Cmd+Shift+Z for redo
        ev.preventDefault();
        if (canRedo()) {
          redo();
          console.log('🔄 Redo triggered via keyboard shortcut');
        }
        return;
      }
    }
  };

  window.addEventListener('keydown', handleKeyPress);

  // Return cleanup function
  return () => {
    window.removeEventListener('keydown', handleKeyPress);
    cleanupWaveformService?.();
  };
}
