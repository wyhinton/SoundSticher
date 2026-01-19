import { get } from 'svelte/store';
import { appState, type AppState } from './state.svelte';
import { loggingState } from './logging';
import type { OperationId, OperationDef, OperationSource } from './operation';

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
// COMMAND INTERPRETER (PURE FUNCTIONS)
// ============================================================================

/**
 * Apply a command to the app state. This is a pure function that mutates the state.
 * Returns the command with any captured data for undo.
 */
export function applyCommand(state: AppState, cmd: Command): Command {
  const isLogging = get(loggingState).operationsLog;

  if (isLogging) {
    console.log(`🔄 UndoRedo: Applying command`, { type: cmd.type, cmd });
  }

  switch (cmd.type) {
    case 'delete-operation':
      return applyDeleteOperation(state, cmd);

    case 'delete-multiple-operations':
      return applyDeleteMultipleOperations(state, cmd);

    case 'add-operation':
      return applyAddOperation(state, cmd);

    case 'update-operation':
      return applyUpdateOperation(state, cmd);

    case 'reorder-operations':
      return applyReorderOperations(state, cmd);

    case 'add-operation-source':
      return applyAddOperationSource(state, cmd);

    case 'remove-operation-source':
      return applyRemoveOperationSource(state, cmd);

    case 'reorder-operation-sources':
      return applyReorderOperationSources(state, cmd);

    case 'remove-operation-sources-from-current-op':
      return applyRemoveOperationSourcesFromCurrentOp(state, cmd);

    case 'batch':
      return applyCommandBatch(state, cmd);

    default:
      throw new Error(`Unknown command type: ${(cmd as any).type}`);
  }
}

/**
 * Create the inverse command for undo operations
 */
export function invertCommand(cmd: Command): Command {
  switch (cmd.type) {
    case 'delete-operation':
      if (!cmd.deletedOperation) {
        throw new Error('Cannot invert delete-operation without deletedOperation data');
      }
      return {
        type: 'add-operation',
        operation: cmd.deletedOperation,
        operationId: cmd.operationId,
        index: cmd.deletedIndex,
      };

    case 'delete-multiple-operations':
      if (!cmd.deletedOperations) {
        throw new Error('Cannot invert delete-multiple-operations without deletedOperations data');
      }
      // For multiple deletes, we need to restore each operation individually
      // in reverse order to maintain correct indices
      const restoreCommands: Command[] = cmd.deletedOperations
        .sort((a: any, b: any) => b.index - a.index) // Restore in reverse order
        .map(({ operationId, operation, index }: any) => ({
          type: 'add-operation' as const,
          operation,
          operationId,
          index,
        }));

      return {
        type: 'batch',
        label: 'Restore Deleted Operations',
        commands: restoreCommands,
      };

    case 'add-operation':
      return {
        type: 'delete-operation',
        operationId: cmd.operationId!,
      };

    case 'update-operation':
      if (!cmd.originalOperation) {
        throw new Error('Cannot invert update-operation without originalOperation data');
      }
      return {
        type: 'update-operation',
        operationId: cmd.operationId,
        patch: cmd.originalOperation,
      };

    case 'reorder-operations':
      if (!cmd.originalOrder) {
        throw new Error('Cannot invert reorder-operations without originalOrder data');
      }
      return {
        type: 'reorder-operations',
        from: cmd.to,
        to: cmd.from,
        originalOrder: cmd.originalOrder,
      };

    case 'add-operation-source':
      const addIndex = cmd.index ?? -1; // If no index specified, it was added at the end
      return {
        type: 'remove-operation-source',
        targetOperationId: cmd.targetOperationId,
        index: addIndex,
      };

    case 'remove-operation-source':
      if (!cmd.removedSource) {
        throw new Error('Cannot invert remove-operation-source without removedSource data');
      }
      return {
        type: 'add-operation-source',
        targetOperationId: cmd.targetOperationId,
        source: cmd.removedSource,
        index: cmd.index,
      };

    case 'reorder-operation-sources':
      if (!cmd.originalSources) {
        throw new Error('Cannot invert reorder-operation-sources without originalSources data');
      }
      return {
        type: 'reorder-operation-sources',
        targetOperationId: cmd.targetOperationId,
        from: cmd.to,
        to: cmd.from,
        originalSources: cmd.originalSources,
      };

    case 'remove-operation-sources-from-current-op':
      if (!cmd.removedSources || !cmd.targetOperationId) {
        throw new Error(
          'Cannot invert remove-operation-sources-from-current-op without removedSources data'
        );
      }
      // To undo removing sources, we need to add them back
      const addBackCommands: Command[] = cmd.removedSources
        .sort((a, b) => a.index - b.index) // Restore in original order
        .map(({ index, source }) => ({
          type: 'add-operation-source' as const,
          targetOperationId: cmd.targetOperationId!,
          source,
          index,
        }));

      if (addBackCommands.length === 1) {
        return addBackCommands[0]!;
      } else {
        return {
          type: 'batch',
          label: 'Restore Removed Sources',
          commands: addBackCommands,
        };
      }

    case 'batch':
      // Invert batch by inverting each command in reverse order
      const invertedCommands = cmd.commands.map(invertCommand).reverse();
      return {
        type: 'batch',
        label: `Undo ${cmd.label || 'Batch'}`,
        commands: invertedCommands,
      };

    default:
      throw new Error(`Cannot invert unknown command type: ${(cmd as any).type}`);
  }
}

// ============================================================================
// INDIVIDUAL COMMAND IMPLEMENTATIONS
// ============================================================================

function applyDeleteOperation(
  state: AppState,
  cmd: DeleteOperationCommand
): DeleteOperationCommand {
  if (!state.operations) {
    state.operations = { defs: {}, order: [], _version: 1 };
  }

  const operation = state.operations.defs[cmd.operationId];
  if (!operation) {
    return cmd; // Operation doesn't exist, nothing to do
  }

  // Capture data for undo
  const deletedIndex = state.operations.order?.indexOf(cmd.operationId) ?? -1;
  const modifiedOperations: Array<{
    operationId: OperationId;
    originalSources: OperationSource[];
    originalOperations?: OperationId[];
  }> = [];
  const deletedPipelineReferences: Array<{
    pipelineName: string;
    originalPipeline: OperationId[];
  }> = [];

  // Remove from definitions
  delete state.operations.defs[cmd.operationId];

  // Remove from order
  if (state.operations.order) {
    state.operations.order = state.operations.order.filter(id => id !== cmd.operationId);
  }

  // Remove references from other operations
  for (const [opId, op] of Object.entries(state.operations.defs)) {
    let modified = false;
    const originalSources = op.sources ? [...op.sources] : [];
    let originalOperations: OperationId[] | undefined;

    if (op.sources) {
      const newSources = op.sources.filter(source => {
        if (source.type === 'operation' || source.type === 'previousOperation') {
          const shouldRemove = source.operationId === cmd.operationId;
          if (shouldRemove) modified = true;
          return !shouldRemove;
        }
        return true;
      });

      if (modified) {
        op.sources = newSources;
      }
    }

    if (op.kind === 'pipeline') {
      originalOperations = [...op.operations];
      const newOperations = op.operations.filter(opId => opId !== cmd.operationId);
      if (newOperations.length !== op.operations.length) {
        op.operations = newOperations;
        modified = true;
      }
    }

    if (modified) {
      modifiedOperations.push({
        operationId: opId,
        originalSources,
        originalOperations,
      });
    }
  }

  // Remove from pipelines
  if (state.operations.pipelines) {
    for (const [pipelineName, pipeline] of Object.entries(state.operations.pipelines)) {
      if (pipeline && pipeline.includes(cmd.operationId)) {
        deletedPipelineReferences.push({
          pipelineName,
          originalPipeline: [...pipeline],
        });
        state.operations.pipelines[pipelineName] = pipeline.filter(id => id !== cmd.operationId);
      }
    }
  }

  // Update versions
  state.operations._version = (state.operations._version ?? 0) + 1;
  state._rev = (state._rev ?? 0) + 1;

  return {
    ...cmd,
    deletedOperation: operation,
    deletedIndex,
    modifiedOperations,
    deletedPipelineReferences,
  };
}

function applyDeleteMultipleOperations(
  state: AppState,
  cmd: DeleteMultipleOperationsCommand
): DeleteMultipleOperationsCommand {
  // For multiple operations, we'll apply individual delete commands
  const deletedOperations: Array<{
    operationId: OperationId;
    operation: OperationDef;
    index: number;
  }> = [];

  for (const operationId of cmd.operationIds) {
    const deleteCmd: DeleteOperationCommand = {
      type: 'delete-operation',
      operationId,
    };

    const result = applyDeleteOperation(state, deleteCmd);
    if (result.deletedOperation) {
      deletedOperations.push({
        operationId,
        operation: result.deletedOperation,
        index: result.deletedIndex ?? -1,
      });
    }
  }

  return {
    ...cmd,
    deletedOperations,
  };
}

function applyAddOperation(state: AppState, cmd: AddOperationCommand): AddOperationCommand {
  if (!state.operations) {
    state.operations = { defs: {}, order: [], _version: 1 };
  }

  // Track if this is the first operation being added
  const isFirstOperation = Object.keys(state.operations.defs).length === 0;

  // Generate ID if not provided
  const operationId = cmd.operationId ?? generateOperationId();

  // Create the full operation
  const operation: OperationDef = {
    ...cmd.operation,
    id: operationId,
  } as OperationDef;

  // Add to definitions
  state.operations.defs[operationId] = operation;

  // Add to order
  if (!state.operations.order) {
    state.operations.order = [];
  }

  if (typeof cmd.index === 'number' && cmd.index >= 0) {
    state.operations.order.splice(cmd.index, 0, operationId);
  } else {
    state.operations.order.push(operationId);
  }

  // If this is the first operation, automatically select it
  if (isFirstOperation && state.uiSettings?.selectedOperationId) {
    state.uiSettings.selectedOperationId = operationId;
  }

  // Update versions
  state.operations._version = (state.operations._version ?? 0) + 1;
  state._rev = (state._rev ?? 0) + 1;

  return {
    ...cmd,
    operationId,
  };
}

function applyUpdateOperation(
  state: AppState,
  cmd: UpdateOperationCommand
): UpdateOperationCommand {
  if (!state.operations?.defs[cmd.operationId]) {
    throw new Error(`Operation ${cmd.operationId} not found`);
  }

  const originalOperation = { ...state.operations.defs[cmd.operationId] };

  // Apply patch
  state.operations.defs[cmd.operationId] = {
    ...state.operations.defs[cmd.operationId],
    ...cmd.patch,
    id: cmd.operationId, // Preserve ID
  } as OperationDef;

  // Update versions
  state.operations._version = (state.operations._version ?? 0) + 1;
  state._rev = (state._rev ?? 0) + 1;

  return {
    ...cmd,
    originalOperation: originalOperation as OperationDef,
  };
}

function applyReorderOperations(
  state: AppState,
  cmd: ReorderOperationsCommand
): ReorderOperationsCommand {
  if (!state.operations?.order) {
    throw new Error('Cannot reorder operations: no order array');
  }

  const originalOrder = [...state.operations.order];

  // Perform the reorder
  const [movedItem] = state.operations.order.splice(cmd.from, 1);
  if (movedItem) {
    state.operations.order.splice(cmd.to, 0, movedItem);
  }

  // Update versions
  state.operations._version = (state.operations._version ?? 0) + 1;
  state._rev = (state._rev ?? 0) + 1;

  return {
    ...cmd,
    originalOrder,
  };
}

function applyAddOperationSource(
  state: AppState,
  cmd: AddOperationSourceCommand
): AddOperationSourceCommand {
  const operation = state.operations?.defs[cmd.targetOperationId];
  if (!operation) {
    throw new Error(`Operation ${cmd.targetOperationId} not found`);
  }

  if (!operation.sources) {
    operation.sources = [];
  }

  // Add source at specified index or at the end
  if (typeof cmd.index === 'number' && cmd.index >= 0) {
    operation.sources.splice(cmd.index, 0, cmd.source);
  } else {
    operation.sources.push(cmd.source);
  }

  // Update versions
  if (state.operations) {
    state.operations._version = (state.operations._version ?? 0) + 1;
  }
  state._rev = (state._rev ?? 0) + 1;

  return cmd;
}

function applyRemoveOperationSource(
  state: AppState,
  cmd: RemoveOperationSourceCommand
): RemoveOperationSourceCommand {
  const operation = state.operations?.defs[cmd.targetOperationId];
  if (!operation?.sources) {
    throw new Error(`Operation ${cmd.targetOperationId} not found or has no sources`);
  }

  if (cmd.index < 0 || cmd.index >= operation.sources.length) {
    throw new Error(`Invalid source index ${cmd.index}`);
  }

  // Capture the removed source
  const removedSource = operation.sources[cmd.index];

  // Remove the source
  operation.sources.splice(cmd.index, 1);

  // Update versions
  if (state.operations) {
    state.operations._version = (state.operations._version ?? 0) + 1;
  }
  state._rev = (state._rev ?? 0) + 1;

  return {
    ...cmd,
    removedSource,
  };
}

function applyReorderOperationSources(
  state: AppState,
  cmd: ReorderOperationSourcesCommand
): ReorderOperationSourcesCommand {
  const operation = state.operations?.defs[cmd.targetOperationId];
  if (!operation?.sources) {
    throw new Error(`Operation ${cmd.targetOperationId} not found or has no sources`);
  }

  const originalSources = [...operation.sources];

  // Perform the reorder
  const [movedSource] = operation.sources.splice(cmd.from, 1);
  if (movedSource) {
    operation.sources.splice(cmd.to, 0, movedSource);
  }

  // Update versions
  if (state.operations) {
    state.operations._version = (state.operations._version ?? 0) + 1;
  }
  state._rev = (state._rev ?? 0) + 1;

  return {
    ...cmd,
    originalSources,
  };
}

function applyRemoveOperationSourcesFromCurrentOp(
  state: AppState,
  cmd: RemoveOperationSourcesFromCurrentOpCommand
): RemoveOperationSourcesFromCurrentOpCommand {
  // Get the currently selected operation
  const selectedOpId = state.uiSettings?.selectedOperationId;

  if (!selectedOpId) {
    // No operation selected, nothing to do
    return cmd;
  }

  const currentOp = state.operations?.defs?.[selectedOpId];
  if (!currentOp) {
    throw new Error(`Current operation "${selectedOpId}" not found`);
  }

  if (currentOp.kind !== 'merge') {
    throw new Error(`Current operation "${selectedOpId}" is not a MergeOp, cannot remove sources`);
  }

  // Track which sources were removed for undo
  const removedSources: Array<{ index: number; source: OperationSource }> = [];

  // Filter out sources that match operation IDs to remove, capturing removed ones
  const newSources: OperationSource[] = [];

  currentOp.sources.forEach((source, index) => {
    if (source.type === 'operation' && cmd.operationIdsToRemove.includes(source.operationId)) {
      // This source should be removed - capture it for undo
      removedSources.push({ index, source });
    } else {
      // Keep this source
      newSources.push(source);
    }
  });

  // Update the operation with the filtered sources
  currentOp.sources = newSources;

  // Update versions
  if (state.operations) {
    state.operations._version = (state.operations._version ?? 0) + 1;
  }
  state._rev = (state._rev ?? 0) + 1;

  return {
    ...cmd,
    targetOperationId: selectedOpId,
    removedSources,
  };
}

function applyCommandBatch(state: AppState, cmd: CommandBatch): CommandBatch {
  const executedCommands: Command[] = [];

  for (const subCmd of cmd.commands) {
    const result = applyCommand(state, subCmd);
    executedCommands.push(result);
  }

  return {
    ...cmd,
    commands: executedCommands,
  };
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

      // Apply the command and capture any side effects
      let finalCommand: Command = command;
      appState.update(state => {
        finalCommand = applyCommand(state, command);
        return state;
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

      // Apply the inverse command
      appState.update(state => {
        applyCommand(state, historyEntry.inverse);
        return state;
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

      // Apply the forward command
      appState.update(state => {
        applyCommand(state, historyEntry.forward);
        return state;
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
    case 'batch':
      return command.label || `Batch (${command.commands.length} commands)`;
    default:
      return 'Unknown Command';
  }
}

function generateOperationId(): OperationId {
  const timestamp = Date.now().toString(36);
  const random = Math.random().toString(36).substring(2, 8);
  return `op_${timestamp}_${random}`;
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

// Legacy compatibility - will be phased out
export function executeCommand(command: any): void {
  console.warn('executeCommand is deprecated, use dispatch() instead');
  // This is a compatibility shim for the old object-based commands
  if (typeof command === 'object' && command.do && command.undo) {
    throw new Error(
      'Object-based commands are no longer supported. Use serializable command data instead.'
    );
  }
}
