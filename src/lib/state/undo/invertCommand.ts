import { OperationDef } from '../operation';
import { Command } from './undo';

export function invertCommand(cmd: Command): Command {
  switch (cmd.type) {
    case 'delete-operation':
      if (!cmd.deletedOperation) {
        throw new Error('Cannot invert delete-operation without deletedOperation data');
      }

      // If there were cascade deletions, we need to restore them all
      if (cmd.cascadeDeletedOperations && cmd.cascadeDeletedOperations.length > 0) {
        const restoreCommands: Command[] = [];

        // First restore cascade deleted operations (in reverse order to maintain dependencies)
        // Sort by index descending so deeper dependencies are restored first
        cmd.cascadeDeletedOperations
          .sort((a, b) => b.index - a.index)
          .forEach(({ operationId, operation, index }) => {
            restoreCommands.push({
              type: 'add-operation' as const,
              operation,
              operationId,
              index,
            });
          });

        // Then restore the main operation
        restoreCommands.push({
          type: 'add-operation' as const,
          operation: cmd.deletedOperation,
          operationId: cmd.operationId,
          index: cmd.deletedIndex,
        });

        return {
          type: 'batch',
          label: 'Restore Deleted Operation(s)',
          commands: restoreCommands,
        };
      }

      // Simple case: no cascade deletions
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
      // Create a patch that contains only the fields that were changed
      const inversePatch: Partial<Omit<OperationDef, 'id'>> = {};
      for (const key in cmd.patch) {
        if (key !== 'id' && key in cmd.originalOperation) {
          inversePatch[key as keyof typeof inversePatch] = cmd.originalOperation[
            key as keyof OperationDef
          ] as any;
        }
      }
      return {
        type: 'update-operation',
        operationId: cmd.operationId,
        patch: inversePatch,
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

    case 'set-render-policy':
      return {
        type: 'set-render-policy',
        operationId: cmd.operationId,
        policy: cmd.previousPolicy!,
      };

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
