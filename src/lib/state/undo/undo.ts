import { get } from 'svelte/store';
import { loggingState } from '../logging';
import type { OperationId, OperationDef, OperationSource, RenderPolicy } from '../operation';
import { appState, type AppState, type TimelineItem } from '../state.svelte';
import type { TimelineSource, TimelineViewState } from '../timeline/timelines';
import { applyCommand } from './applyCommand';
import { invertCommand } from './invertCommand';

// ============================================================================
// SERIALIZABLE COMMAND TYPES
// ============================================================================

/**
 * Serializable commands representing user intentions.
 * These are plain data objectsnow that can be JSON-serialized, logged, and persisted.
 */
export type Command =
  | DeleteOperationCommand
  | DeleteMultipleOperationsCommand
  | AddOperationCommand
  | UpdateOperationCommand
  | ReorderOperationsCommand
  | AddOperationSourceCommand
  | RemoveOperationSourceCommand
  | ReorderOperationSourcesCommand
  | RemoveOperationSourcesFromCurrentOpCommand
  | SetRenderPolicyCommand
  | ToggleTimelineVisibilityCommand
  | CommandBatch;

// Individual command types
export interface DeleteOperationCommand {
  type: 'delete-operation';
  operationId: OperationId;
  // Captured data for undo
  deletedOperation?: OperationDef;
  deletedIndex?: number;
  modifiedOperations?: Array<{
    operationId: OperationId;
    originalSources: OperationSource[];
    originalOperations?: OperationId[];
  }>;
  deletedPipelineReferences?: Array<{
    pipelineName: string;
    originalPipeline: OperationId[];
  }>;
  // Cascade deleted operations (operations that were only used by this operation)
  // These operations' full definitions are stored so they can be recreated on undo
  cascadeDeletedOperations?: Array<{
    operationId: OperationId;
    operation: OperationDef;
    index: number;
  }>;
}

export interface DeleteMultipleOperationsCommand {
  type: 'delete-multiple-operations';
  operationIds: OperationId[];
  // Captured data for undo
  deletedOperations?: Array<{
    operationId: OperationId;
    operation: OperationDef;
    index: number;
  }>;
  modifiedOperations?: Array<{
    operationId: OperationId;
    originalSources: OperationSource[];
    originalOperations?: OperationId[];
  }>;
  deletedPipelineReferences?: Array<{
    pipelineName: string;
    originalPipeline: OperationId[];
  }>;
}

export interface AddOperationCommand {
  type: 'add-operation';
  operation: Omit<OperationDef, 'id'>;
  operationId?: OperationId; // Generated during execution
  index?: number; // Position in order array
}

export interface UpdateOperationCommand {
  type: 'update-operation';
  operationId: OperationId;
  patch: Partial<Omit<OperationDef, 'id'>>;
  // Captured data for undo
  originalOperation?: OperationDef;
}

export interface ReorderOperationsCommand {
  type: 'reorder-operations';
  from: number;
  to: number;
  // Captured data for undo
  originalOrder?: OperationId[];
}

export interface AddOperationSourceCommand {
  type: 'add-operation-source';
  targetOperationId: OperationId;
  source: OperationSource;
  index?: number; // Position to insert at (default: end)
}

export interface RemoveOperationSourceCommand {
  type: 'remove-operation-source';
  targetOperationId: OperationId;
  index: number;
  // Captured data for undo
  removedSource?: OperationSource;
}

export interface ReorderOperationSourcesCommand {
  type: 'reorder-operation-sources';
  targetOperationId: OperationId;
  from: number;
  to: number;
  // Captured data for undo
  originalSources?: OperationSource[];
}

export interface RemoveOperationSourcesFromCurrentOpCommand {
  type: 'remove-operation-sources-from-current-op';
  operationIdsToRemove: OperationId[];
  // Captured data for undo
  targetOperationId?: OperationId;
  removedSources?: Array<{
    index: number;
    source: OperationSource;
  }>;
}

export interface SetRenderPolicyCommand {
  type: 'set-render-policy';
  operationId: OperationId;
  policy: RenderPolicy;
  // Captured data for undo
  previousPolicy?: RenderPolicy;
}

export interface ToggleTimelineVisibilityCommand {
  type: 'toggle-timeline-visibility';
  operationId: OperationId;
  // Captured data for undo
  wasVisible?: boolean;
  timelineId?: string;
  timelineData?: {
    source: TimelineSource;
    serializedItems?: TimelineItem[];
  };
  viewState?: TimelineViewState;
}

export interface CommandBatch {
  type: 'batch';
  label?: string;
  commands: Command[];
}

// ============================================================================
// HISTORY ENTRY
// ============================================================================

export interface HistoryEntry {
  forward: Command;
  inverse: Command;
  timestamp: number;
  label: string;
  id: string;
}

// ============================================================================
// HISTORY MANAGER
// ============================================================================

export interface UndoRedoState {
  history: HistoryEntry[];
  currentIndex: number; // Points to the latest applied command
  maxHistorySize: number;
  isExecuting: boolean;
}

class UndoRedoManager {
  private state: UndoRedoState = {
    history: [],
    currentIndex: -1, // -1 means no commands executed yet
    maxHistorySize: 50,
    isExecuting: false,
  };

  /**
   * Execute a command and add it to history
   */
  dispatch(command: Command, label?: string): void {
    const isLogging = get(loggingState).operationsLog;

    if (this.state.isExecuting) {
      console.warn('⚠️ UndoRedo: Cannot dispatch command while another command is executing');
      return;
    }

    try {
      this.state.isExecuting = true;

      const commandLabel = label || getCommandLabel(command);

      if (isLogging) {
        console.log(`🔄 UndoRedo: Dispatching command "${commandLabel}"`, command);
      }

      // Apply the command and capture any side effects using transactional updates
      let finalCommand: Command = command;
      updateStoresTransactionally(stores => {
        finalCommand = applyCommand(stores.appState, command);
      });

      // Create the inverse command for undo
      const inverseCommand = invertCommand(finalCommand);

      // Create history entry
      const historyEntry: HistoryEntry = {
        forward: finalCommand,
        inverse: inverseCommand,
        timestamp: Date.now(),
        label: commandLabel,
        id: generateHistoryId(),
      };

      // Remove any "future" history if we're not at the end
      if (this.state.currentIndex < this.state.history.length - 1) {
        this.state.history = this.state.history.slice(0, this.state.currentIndex + 1);
      }

      // Add to history
      this.state.history.push(historyEntry);
      this.state.currentIndex = this.state.history.length - 1;

      // Trim history if too long
      if (this.state.history.length > this.state.maxHistorySize) {
        this.state.history.shift();
        this.state.currentIndex--;
      }

      if (isLogging) {
        console.log(`✅ UndoRedo: Command "${commandLabel}" dispatched successfully`);
        console.log(
          `📊 UndoRedo: History size: ${this.state.history.length}, Current index: ${this.state.currentIndex}`
        );
      }
    } catch (error) {
      console.error(`❌ UndoRedo: Error dispatching command:`, error);
      throw error;
    } finally {
      this.state.isExecuting = false;
    }
  }

  /**
   * Undo the last command
   */
  undo(): boolean {
    const isLogging = get(loggingState).operationsLog;

    if (this.state.isExecuting) {
      console.warn('⚠️ UndoRedo: Cannot undo while command is executing');
      return false;
    }

    if (this.state.currentIndex < 0) {
      if (isLogging) {
        console.log('📭 UndoRedo: Nothing to undo');
      }
      return false;
    }

    try {
      this.state.isExecuting = true;

      const historyEntry = this.state.history[this.state.currentIndex];
      if (!historyEntry) {
        throw new Error('History entry not found');
      }

      if (isLogging) {
        console.log(`↶ UndoRedo: Undoing "${historyEntry.label}"`);
      }

      // Apply the inverse command using transactional updates
      updateStoresTransactionally(stores => {
        applyCommand(stores.appState, historyEntry.inverse);
      });

      // Move the current index back
      this.state.currentIndex--;

      if (isLogging) {
        console.log(`✅ UndoRedo: Undid "${historyEntry.label}" successfully`);
      }

      return true;
    } catch (error) {
      console.error(`❌ UndoRedo: Error undoing command:`, error);
      throw error;
    } finally {
      this.state.isExecuting = false;
    }
  }

  /**
   * Redo the next command
   */
  redo(): boolean {
    const isLogging = get(loggingState).operationsLog;

    if (this.state.isExecuting) {
      console.warn('⚠️ UndoRedo: Cannot redo while command is executing');
      return false;
    }

    if (this.state.currentIndex >= this.state.history.length - 1) {
      if (isLogging) {
        console.log('📭 UndoRedo: Nothing to redo');
      }
      return false;
    }

    try {
      this.state.isExecuting = true;

      const nextIndex = this.state.currentIndex + 1;
      const historyEntry = this.state.history[nextIndex];
      if (!historyEntry) {
        throw new Error('History entry not found');
      }

      if (isLogging) {
        console.log(`↷ UndoRedo: Redoing "${historyEntry.label}"`);
      }

      // Apply the forward command using transactional updates
      updateStoresTransactionally(stores => {
        applyCommand(stores.appState, historyEntry.forward);
      });

      // Move the current index forward
      this.state.currentIndex = nextIndex;

      if (isLogging) {
        console.log(`✅ UndoRedo: Redid "${historyEntry.label}" successfully`);
      }

      return true;
    } catch (error) {
      console.error(`❌ UndoRedo: Error redoing command:`, error);
      throw error;
    } finally {
      this.state.isExecuting = false;
    }
  }

  /**
   * Check if undo is available
   */
  canUndo(): boolean {
    return this.state.currentIndex >= 0 && !this.state.isExecuting;
  }

  /**
   * Check if redo is available
   */
  canRedo(): boolean {
    return this.state.currentIndex < this.state.history.length - 1 && !this.state.isExecuting;
  }

  /**
   * Get the label of the next command that would be undone
   */
  getUndoLabel(): string | null {
    if (this.state.currentIndex < 0) return null;
    return this.state.history[this.state.currentIndex]?.label ?? null;
  }

  /**
   * Get the label of the next command that would be redone
   */
  getRedoLabel(): string | null {
    const nextIndex = this.state.currentIndex + 1;
    if (nextIndex >= this.state.history.length) return null;
    return this.state.history[nextIndex]?.label ?? null;
  }

  /**
   * Clear all history
   */
  clearHistory(): void {
    const isLogging = get(loggingState).operationsLog;

    if (isLogging) {
      console.log(`🗑️ UndoRedo: Clearing history (${this.state.history.length} entries)`);
    }

    this.state.history = [];
    this.state.currentIndex = -1;
  }

  /**
   * Get current state for debugging
   */
  getState(): Readonly<UndoRedoState> {
    return { ...this.state };
  }

  /**
   * Get history for debugging
   */
  getHistory(): ReadonlyArray<Readonly<HistoryEntry>> {
    return this.state.history.map(entry => ({ ...entry }));
  }

  /**
   * Get undo stack information
   */
  getUndoStackInfo(): ReadonlyArray<{ label: string; id: string; timestamp: number }> {
    return this.state.history.slice(0, this.state.currentIndex + 1).map(entry => ({
      label: entry.label,
      id: entry.id,
      timestamp: entry.timestamp,
    }));
  }

  /**
   * Get redo stack information
   */
  getRedoStackInfo(): ReadonlyArray<{ label: string; id: string; timestamp: number }> {
    return this.state.history.slice(this.state.currentIndex + 1).map(entry => ({
      label: entry.label,
      id: entry.id,
      timestamp: entry.timestamp,
    }));
  }
}

// ============================================================================
// TRANSACTIONAL STORE UPDATES
// ============================================================================

/**
 * Store states for transactional updates
 */
interface StoreStates {
  appState: AppState;
}

/**
 * Update stores transactionally to maintain consistency.
 * Since timelines are now part of appState, we only need to update one store.
 */
function updateStoresTransactionally(updater: (stores: StoreStates) => void): void {
  const isLogging = get(loggingState).operationsLog;
  if (isLogging) {
    console.log('🔄 UndoRedo: Starting transactional store update');
  }
  try {
    // Step 1: Get current value from appState
    const stores: StoreStates = {
      appState: get(appState),
    };
    // Step 2: Let the caller modify the stores
    updater(stores);
    // Step 3: Write updated store back
    appState.set(stores.appState);
    if (isLogging) {
      console.log('✅ UndoRedo: Transactional store update completed successfully');
    }
  } catch (error) {
    console.error('❌ UndoRedo: Error during transactional store update:', error);
    throw error;
  }
}

// ============================================================================
// UTILITIES
// ============================================================================

function getCommandLabel(command: Command): string {
  switch (command.type) {
    case 'delete-operation':
      return 'Delete Operation';
    case 'delete-multiple-operations':
      return `Delete ${command.operationIds.length} Operations`;
    case 'add-operation':
      return 'Add Operation';
    case 'update-operation':
      return 'Update Operation';
    case 'reorder-operations':
      return 'Reorder Operations';
    case 'add-operation-source':
      return 'Add Source';
    case 'remove-operation-source':
      return 'Remove Source';
    case 'reorder-operation-sources':
      return 'Reorder Sources';
    case 'remove-operation-sources-from-current-op':
      return `Remove ${command.operationIdsToRemove.length} Source(s) from Current Operation`;
    case 'set-render-policy':
      return 'Set Render Policy';
    case 'toggle-timeline-visibility':
      return 'Toggle Timeline Visibility';
    case 'batch':
      return command.label || `Batch (${command.commands.length} commands)`;
    default:
      return 'Unknown Command';
  }
}

function generateHistoryId(): string {
  const timestamp = Date.now().toString(36);
  const random = Math.random().toString(36).substring(2, 8);
  return `hist_${timestamp}_${random}`;
}

// ============================================================================
// SINGLETON INSTANCE & PUBLIC API
// ============================================================================

export const undoRedoManager = new UndoRedoManager();

/**
 * Dispatch a command for execution with undo/redo support
 */
export function dispatch(command: Command, label?: string): void {
  undoRedoManager.dispatch(command, label);
}

/**
 * Undo the last command
 */
export function undo(): boolean {
  return undoRedoManager.undo();
}

/**
 * Redo the last undone command
 */
export function redo(): boolean {
  return undoRedoManager.redo();
}

/**
 * Check if undo is available
 */
export function canUndo(): boolean {
  return undoRedoManager.canUndo();
}

/**
 * Check if redo is available
 */
export function canRedo(): boolean {
  return undoRedoManager.canRedo();
}

/**
 * Get undo/redo labels for UI display
 */
export function getUndoRedoLabels(): { undo: string | null; redo: string | null } {
  return {
    undo: undoRedoManager.getUndoLabel(),
    redo: undoRedoManager.getRedoLabel(),
  };
}

/**
 * Clear all undo/redo history
 */
export function clearUndoRedoHistory(): void {
  undoRedoManager.clearHistory();
}

/**
 * Get stack sizes for debugging
 */
export function getUndoRedoStackSizes(): { undoCount: number; redoCount: number } {
  const state = undoRedoManager.getState();
  return {
    undoCount: state.currentIndex + 1,
    redoCount: state.history.length - state.currentIndex - 1,
  };
}

/**
 * Get undo stack information for debugging
 */
export function getUndoStack(): ReadonlyArray<{ label: string; id: string; timestamp: number }> {
  return undoRedoManager.getUndoStackInfo();
}

/**
 * Get redo stack information for debugging
 */
export function getRedoStack(): ReadonlyArray<{ label: string; id: string; timestamp: number }> {
  return undoRedoManager.getRedoStackInfo();
}

/**
 * Get full history for debugging
 */
export function getHistory(): ReadonlyArray<Readonly<HistoryEntry>> {
  return undoRedoManager.getHistory();
}

// ============================================================================
// CONVENIENCE COMMAND CREATORS
// ============================================================================

/**
 * Create and dispatch a command to delete an operation by ID
 */
export function deleteOperationByIdCommand(id: OperationId): void {
  const command: DeleteOperationCommand = {
    type: 'delete-operation',
    operationId: id,
  };
  dispatch(command);
}

/**
 * Create and dispatch a command to delete multiple operations by ID
 */
export function deleteOperationsByIdCommand(ids: OperationId[]): void {
  const command: DeleteMultipleOperationsCommand = {
    type: 'delete-multiple-operations',
    operationIds: ids,
  };
  dispatch(command);
}

/**
 * Create and dispatch a command to remove operation sources from the current operation
 */
export function removeOperationSourcesFromCurrentOpCommand(operationIds: OperationId[]): void {
  const command: RemoveOperationSourcesFromCurrentOpCommand = {
    type: 'remove-operation-sources-from-current-op',
    operationIdsToRemove: operationIds,
  };
  dispatch(command, `Remove ${operationIds.length} Source(s) from Current Operation`);
}

/**
 * Create and dispatch a command to set render policy for an operation
 */
export function setRenderPolicyCommand(
  operationId: OperationId,
  policy: RenderPolicy,
  label?: string
): void {
  const command: SetRenderPolicyCommand = {
    type: 'set-render-policy',
    operationId,
    policy,
  };

  const policyLabels: Record<RenderPolicy, string> = {
    auto: 'Enable Auto-Render',
    frozen: 'Freeze Operation',
    manual: 'Set Manual Render',
  };

  dispatch(command, label || policyLabels[policy]);
}
