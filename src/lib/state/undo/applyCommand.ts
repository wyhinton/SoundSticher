import { get } from 'svelte/store';
import { appState, type AppState } from '../state.svelte';
import { loggingState } from '../logging';
import type { OperationId, OperationDef, OperationSource, RenderPolicy } from '../operation';
import { generateOperationId } from '../operation';
import type {
  Command,
  DeleteOperationCommand,
  DeleteMultipleOperationsCommand,
  AddOperationCommand,
  UpdateOperationCommand,
  ReorderOperationsCommand,
  AddOperationSourceCommand,
  RemoveOperationSourceCommand,
  ReorderOperationSourcesCommand,
  RemoveOperationSourcesFromCurrentOpCommand,
  SetRenderPolicyCommand,
  CommandBatch,
} from './undo';

// ============================================================================
// COMMAND APPLICATION DISPATCHER
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

    case 'set-render-policy':
      return applySetRenderPolicy(state, cmd);

    case 'batch':
      return applyCommandBatch(state, cmd);

    default:
      throw new Error(`Unknown command type: ${(cmd as any).type}`);
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
  const cascadeDeletedOperations: Array<{
    operationId: OperationId;
    operation: OperationDef;
    index: number;
  }> = [];

  // Track operations that should be cascade deleted
  const cascadeDeleteOperationIds: Set<OperationId> = new Set();

  // Before deleting, find operations in the deleted operation's sources
  // that are ONLY used by this operation
  if (operation.sources) {
    for (const source of operation.sources) {
      if (source.type === 'operation' || source.type === 'previousOperation') {
        const sourceOpId = source.operationId;

        // Count how many operations reference this source operation
        let referenceCount = 0;
        for (const [opId, op] of Object.entries(state.operations.defs)) {
          if (opId === cmd.operationId) continue; // Skip the operation being deleted

          if (op.sources) {
            const hasReference = op.sources.some(
              s =>
                (s.type === 'operation' || s.type === 'previousOperation') &&
                s.operationId === sourceOpId
            );
            if (hasReference) referenceCount++;
          }

          if (op.kind === 'pipeline' && op.operations.includes(sourceOpId)) {
            referenceCount++;
          }
        }

        // If no other operations reference this source, mark it for cascade deletion
        if (referenceCount === 0) {
          cascadeDeleteOperationIds.add(sourceOpId);
        }
      }
    }
  }

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

  // Cascade delete orphaned operations and capture them for undo
  for (const cascadeOpId of cascadeDeleteOperationIds) {
    const cascadeOp = state.operations.defs[cascadeOpId];
    if (cascadeOp) {
      const cascadeIndex = state.operations.order?.indexOf(cascadeOpId) ?? -1;

      // Capture the operation before deleting it
      cascadeDeletedOperations.push({
        operationId: cascadeOpId,
        operation: { ...cascadeOp }, // Deep copy to preserve the operation
        index: cascadeIndex,
      });

      // Recursively delete this operation (which may cascade further)
      const cascadeCmd: DeleteOperationCommand = {
        type: 'delete-operation',
        operationId: cascadeOpId,
      };
      const cascadeResult = applyDeleteOperation(state, cascadeCmd);

      // Also capture any nested cascade deletions
      if (cascadeResult.cascadeDeletedOperations) {
        cascadeDeletedOperations.push(...cascadeResult.cascadeDeletedOperations);
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
    cascadeDeletedOperations:
      cascadeDeletedOperations.length > 0 ? cascadeDeletedOperations : undefined,
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
  const isOnlyOperation = Object.keys(state.operations.defs).length === 0;

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
  if (isOnlyOperation && state.uiSettings?.selectedOperationId !== undefined) {
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

function applySetRenderPolicy(
  state: AppState,
  cmd: SetRenderPolicyCommand
): SetRenderPolicyCommand {
  const operation = state.operations?.defs[cmd.operationId];
  if (!operation) {
    throw new Error(`Operation ${cmd.operationId} not found`);
  }

  // Capture previous policy for undo
  const previousPolicy = operation.renderPolicy || 'auto';

  // Update the render policy
  operation.renderPolicy = cmd.policy;

  // Update versions
  if (state.operations) {
    state.operations._version = (state.operations._version ?? 0) + 1;
  }
  state._rev = (state._rev ?? 0) + 1;

  return {
    ...cmd,
    previousPolicy,
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
