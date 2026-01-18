import { get } from 'svelte/store';
import { appState, type AppState } from './state.svelte';
import { loggingState } from './logging';
import type { OperationId, OperationDef, OperationSource } from './operation';

// ============================================================================
// SERIALIZABLE COMMAND TYPES
// ============================================================================

/**
 * Serializable commands representing user intentions.
 * These are plain data objects that can be JSON-serialized, logged, and persisted.
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

export interface CommandBatch {
  type: 'batch';
  label?: string;
  commands: Command[];
}

// ============================================================================
// CONCRETE COMMAND IMPLEMENTATIONS
// ============================================================================

/**
 * Command to delete operations with full undo support.
 * Stores all necessary data to restore deleted operations and their relationships.
 */
export class DeleteOperationCommand implements Command {
  readonly label: string;
  readonly id: string;

  // Data needed for undo - captured during execution
  private deletedOperations: Map<OperationId, OperationDef> = new Map();
  private deletedOrder: OperationId[] = [];
  private deletedPipelines: Record<string, OperationId[]> = {};
  private modifiedOperations: Map<
    OperationId,
    {
      originalSources: any[];
      originalOperations?: OperationId[];
    }
  > = new Map();
  private originalVersion = 0;
  private originalRev = 0;

  constructor(
    private readonly operationIds: OperationId[],
    customLabel?: string
  ) {
    const idList = operationIds.join(', ');
    this.label =
      customLabel ||
      (operationIds.length === 1 ? `Delete Operation` : `Delete ${operationIds.length} Operations`);
    this.id = `delete-ops-${Date.now()}-${Math.random().toString(36).slice(2)}`;
  }

  do(state: AppState): void {
    if (!state.operations) {
      state.operations = { defs: {}, order: [], _version: 1 };
    }

    const isLogging = get(loggingState).operationsLog;
    const idsToDelete = new Set(this.operationIds);

    // Clear previous undo data in case command is re-executed
    this.deletedOperations.clear();
    this.deletedOrder = [];
    this.deletedPipelines = {};
    this.modifiedOperations.clear();

    // Store original state versions
    this.originalVersion = state.operations._version ?? 0;
    this.originalRev = state._rev ?? 0;

    if (isLogging) {
      console.log(`🗑️ DeleteCommand: Executing deletion of operations`, this.operationIds);
    }

    let deletedAny = false;

    // 1. Store and remove operation definitions
    for (const id of this.operationIds) {
      const operation = state.operations.defs[id];
      if (!operation) {
        if (isLogging) {
          console.warn(`⚠️ DeleteCommand: Cannot delete id="${id}" - not found`);
        }
        continue;
      }

      // Store for undo
      this.deletedOperations.set(id, { ...operation });

      // Remove from state
      delete state.operations.defs[id];
      deletedAny = true;

      if (isLogging) {
        console.log(`✅ DeleteCommand: Stored and deleted operation id="${id}"`);
      }
    }

    // 2. Store and update order array
    if (deletedAny && state.operations.order) {
      this.deletedOrder = state.operations.order.filter(id => idsToDelete.has(id));
      state.operations.order = state.operations.order.filter(id => !idsToDelete.has(id));
    }

    // 3. Store and remove references from remaining operations' sources
    if (deletedAny) {
      for (const [opId, op] of Object.entries(state.operations.defs)) {
        let modified = false;
        const originalSources = op.sources ? [...op.sources] : [];
        let originalOperations: OperationId[] | undefined;

        if (op.sources) {
          const newSources = op.sources.filter(source => {
            if (source.type === 'operation' || source.type === 'previousOperation') {
              const shouldRemove = idsToDelete.has(source.operationId);
              if (shouldRemove) modified = true;
              return !shouldRemove;
            }
            return true;
          });

          if (modified) {
            op.sources = newSources;
          }
        }

        // Also clean up pipeline operation references
        if (op.kind === 'pipeline') {
          originalOperations = [...op.operations];
          const newOperations = op.operations.filter(opId => !idsToDelete.has(opId));
          if (newOperations.length !== op.operations.length) {
            op.operations = newOperations;
            modified = true;
          }
        }

        // Store modification data for undo
        if (modified) {
          this.modifiedOperations.set(opId, {
            originalSources,
            originalOperations,
          });
        }
      }
    }

    // 4. Store and remove from pipelines
    if (deletedAny && state.operations.pipelines) {
      for (const [pipelineName, pipeline] of Object.entries(state.operations.pipelines)) {
        if (pipeline) {
          const originalPipeline = [...pipeline];
          const newPipeline = pipeline.filter(id => !idsToDelete.has(id));

          if (newPipeline.length !== originalPipeline.length) {
            this.deletedPipelines[pipelineName] = originalPipeline;
            state.operations.pipelines[pipelineName] = newPipeline;
          }
        }
      }
    }

    // 5. Update versions
    if (deletedAny) {
      state.operations._version = (state.operations._version ?? 0) + 1;
      state._rev = (state._rev ?? 0) + 1;
    }

    if (isLogging) {
      console.log(
        `✅ DeleteCommand: Successfully deleted ${this.deletedOperations.size} operations`
      );
    }
  }

  undo(state: AppState): void {
    const isLogging = get(loggingState).operationsLog;

    if (isLogging) {
      console.log(
        `🔄 DeleteCommand: Undoing deletion of ${this.deletedOperations.size} operations`
      );
    }

    if (!state.operations) {
      state.operations = { defs: {}, order: [], _version: 1 };
    }

    // 1. Restore operation definitions
    for (const [id, operation] of this.deletedOperations) {
      state.operations.defs[id] = { ...operation };
      if (isLogging) {
        console.log(`↶ DeleteCommand: Restored operation id="${id}"`);
      }
    }

    // 2. Restore order
    if (this.deletedOrder.length > 0 && state.operations.order) {
      // Find insertion points and restore order
      const currentOrder = [...state.operations.order];
      for (const deletedId of this.deletedOrder) {
        // Insert in a reasonable position (at the end for simplicity)
        currentOrder.push(deletedId);
      }
      state.operations.order = currentOrder;
    }

    // 3. Restore modified operations' sources and pipeline references
    for (const [opId, { originalSources, originalOperations }] of this.modifiedOperations) {
      const operation = state.operations.defs[opId];
      if (operation) {
        operation.sources = [...originalSources];

        if (operation.kind === 'pipeline' && originalOperations) {
          operation.operations = [...originalOperations];
        }

        if (isLogging) {
          console.log(`↶ DeleteCommand: Restored references in operation id="${opId}"`);
        }
      }
    }

    // 4. Restore pipelines
    for (const [pipelineName, originalPipeline] of Object.entries(this.deletedPipelines)) {
      if (state.operations.pipelines) {
        state.operations.pipelines[pipelineName] = [...originalPipeline];
        if (isLogging) {
          console.log(`↶ DeleteCommand: Restored pipeline "${pipelineName}"`);
        }
      }
    }

    // 5. Restore versions (optional - could increment instead)
    state.operations._version = this.originalVersion;
    state._rev = this.originalRev;

    if (isLogging) {
      console.log(
        `✅ DeleteCommand: Successfully undid deletion of ${this.deletedOperations.size} operations`
      );
    }
  }
}

/**
 * Undo/Redo system state
 */
export interface UndoRedoState {
  undoStack: Command[];
  redoStack: Command[];
  maxStackSize: number;
  isExecuting: boolean; // Prevent undo/redo during command execution
}

// ============================================================================
// UNDO/REDO SYSTEM
// ============================================================================

class UndoRedoManager {
  private state: UndoRedoState = {
    undoStack: [],
    redoStack: [],
    maxStackSize: 50, // Configurable limit to prevent memory issues
    isExecuting: false,
  };

  /**
   * Execute a command and add it to the undo stack
   */
  executeCommand(command: Command): void {
    const isLogging = get(loggingState).operationsLog;

    if (this.state.isExecuting) {
      console.warn('⚠️ UndoRedo: Cannot execute command while another command is executing');
      return;
    }

    try {
      this.state.isExecuting = true;

      if (isLogging) {
        console.log(`🔄 UndoRedo: Executing command "${command.label}"`);
      }

      // Execute the command
      appState.update(state => {
        command.do(state);
        return state;
      });

      // Add to undo stack
      this.state.undoStack.push(command);

      // Clear redo stack (executing new command invalidates redo history)
      this.state.redoStack = [];

      // Enforce stack size limit
      if (this.state.undoStack.length > this.state.maxStackSize) {
        this.state.undoStack.shift(); // Remove oldest command
        if (isLogging) {
          console.log(`📚 UndoRedo: Trimmed undo stack to ${this.state.maxStackSize} commands`);
        }
      }

      if (isLogging) {
        console.log(`✅ UndoRedo: Command "${command.label}" executed successfully`);
        console.log(
          `📊 UndoRedo: Stack sizes - Undo: ${this.state.undoStack.length}, Redo: ${this.state.redoStack.length}`
        );
      }
    } catch (error) {
      console.error(`❌ UndoRedo: Error executing command "${command.label}":`, error);
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

    const command = this.state.undoStack.pop();
    if (!command) {
      if (isLogging) {
        console.log('📭 UndoRedo: Nothing to undo');
      }
      return false;
    }

    try {
      this.state.isExecuting = true;

      if (isLogging) {
        console.log(`↶ UndoRedo: Undoing command "${command.label}"`);
      }

      // Undo the command
      appState.update(state => {
        command.undo(state);
        return state;
      });

      // Move to redo stack
      this.state.redoStack.push(command);

      if (isLogging) {
        console.log(`✅ UndoRedo: Command "${command.label}" undone successfully`);
        console.log(
          `📊 UndoRedo: Stack sizes - Undo: ${this.state.undoStack.length}, Redo: ${this.state.redoStack.length}`
        );
      }

      return true;
    } catch (error) {
      console.error(`❌ UndoRedo: Error undoing command "${command.label}":`, error);
      // Put command back on undo stack if undo failed
      this.state.undoStack.push(command);
      throw error;
    } finally {
      this.state.isExecuting = false;
    }
  }

  /**
   * Redo the last undone command
   */
  redo(): boolean {
    const isLogging = get(loggingState).operationsLog;

    if (this.state.isExecuting) {
      console.warn('⚠️ UndoRedo: Cannot redo while command is executing');
      return false;
    }

    const command = this.state.redoStack.pop();
    if (!command) {
      if (isLogging) {
        console.log('📭 UndoRedo: Nothing to redo');
      }
      return false;
    }

    try {
      this.state.isExecuting = true;

      if (isLogging) {
        console.log(`↷ UndoRedo: Redoing command "${command.label}"`);
      }

      // Re-execute the command
      appState.update(state => {
        command.do(state);
        return state;
      });

      // Move back to undo stack
      this.state.undoStack.push(command);

      if (isLogging) {
        console.log(`✅ UndoRedo: Command "${command.label}" redone successfully`);
        console.log(
          `📊 UndoRedo: Stack sizes - Undo: ${this.state.undoStack.length}, Redo: ${this.state.redoStack.length}`
        );
      }

      return true;
    } catch (error) {
      console.error(`❌ UndoRedo: Error redoing command "${command.label}":`, error);
      // Put command back on redo stack if redo failed
      this.state.redoStack.push(command);
      throw error;
    } finally {
      this.state.isExecuting = false;
    }
  }

  /**
   * Check if undo is available
   */
  canUndo(): boolean {
    return this.state.undoStack.length > 0 && !this.state.isExecuting;
  }

  /**
   * Check if redo is available
   */
  canRedo(): boolean {
    return this.state.redoStack.length > 0 && !this.state.isExecuting;
  }

  /**
   * Get the label of the next command that would be undone
   */
  getUndoLabel(): string | null {
    const command = this.state.undoStack[this.state.undoStack.length - 1];
    return command ? command.label : null;
  }

  /**
   * Get the label of the next command that would be redone
   */
  getRedoLabel(): string | null {
    const command = this.state.redoStack[this.state.redoStack.length - 1];
    return command ? command.label : null;
  }

  /**
   * Clear all undo/redo history
   */
  clearHistory(): void {
    const isLogging = get(loggingState).operationsLog;

    if (isLogging) {
      console.log(
        `🗑️ UndoRedo: Clearing history (${this.state.undoStack.length} undo, ${this.state.redoStack.length} redo)`
      );
    }

    this.state.undoStack = [];
    this.state.redoStack = [];
  }

  /**
   * Get current stack sizes for debugging
   */
  getStackSizes(): { undoCount: number; redoCount: number } {
    return {
      undoCount: this.state.undoStack.length,
      redoCount: this.state.redoStack.length,
    };
  }

  /**
   * Set maximum stack size (for memory management)
   */
  setMaxStackSize(size: number): void {
    this.state.maxStackSize = Math.max(1, size);

    // Trim existing stacks if necessary
    while (this.state.undoStack.length > this.state.maxStackSize) {
      this.state.undoStack.shift();
    }
    while (this.state.redoStack.length > this.state.maxStackSize) {
      this.state.redoStack.shift();
    }
  }

  /**
   * Get undo stack information for debugging (without exposing actual commands)
   */
  getUndoStackInfo(): ReadonlyArray<{ label: string; id?: string }> {
    return this.state.undoStack.map(cmd => ({
      label: cmd.label,
      id: cmd.id,
    }));
  }

  /**
   * Get redo stack information for debugging (without exposing actual commands)
   */
  getRedoStackInfo(): ReadonlyArray<{ label: string; id?: string }> {
    return this.state.redoStack.map(cmd => ({
      label: cmd.label,
      id: cmd.id,
    }));
  }
}

// ============================================================================
// SINGLETON INSTANCE
// ============================================================================

export const undoRedoManager = new UndoRedoManager();

// ============================================================================
// CONVENIENCE FUNCTIONS FOR UI
// ============================================================================

/**
 * Execute an undoable command
 */
export function executeCommand(command: Command): void {
  undoRedoManager.executeCommand(command);
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
  return undoRedoManager.getStackSizes();
}

/**
 * Get detailed undo stack information for debugging
 */
export function getUndoStack(): ReadonlyArray<{ label: string; id?: string }> {
  return undoRedoManager.getUndoStackInfo();
}

/**
 * Get detailed redo stack information for debugging
 */
export function getRedoStack(): ReadonlyArray<{ label: string; id?: string }> {
  return undoRedoManager.getRedoStackInfo();
}

/**
 * Create and execute a command to delete operations by ID
 */
export function deleteOperationByIdCommand(id: OperationId): void {
  const command = new DeleteOperationCommand([id]);
  executeCommand(command);
}

/**
 * Create and execute a command to delete multiple operations by ID
 */
export function deleteOperationsByIdCommand(ids: OperationId[]): void {
  const command = new DeleteOperationCommand(ids);
  executeCommand(command);
}
