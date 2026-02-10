/**
 * @deprecated Timeline state is now managed in appState. Timeline-related
 * command effects are handled directly in applyCommand.ts.
 * This file is retained for reference but is no longer used.
 */

import type { Command } from './undo';

/**
 * @deprecated No longer needed — timeline state is managed in appState.
 * All timeline command effects are now handled in applyCommand.ts.
 */
export function applyTimelineCommand(_state: unknown, _command: Command): unknown {
  // No-op: timeline state is now part of appState and handled by applyCommand
  return _state;
}
