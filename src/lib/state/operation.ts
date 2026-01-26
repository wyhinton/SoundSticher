import { get } from 'svelte/store';
import { appState, AppState, AudioFileItem, Section } from './state.svelte';
import { loggingState, logger } from './logging';
import { groupRegistry, GroupResult } from './groups';
import {
  dispatch,
  type DeleteOperationCommand,
  type DeleteMultipleOperationsCommand,
  type UpdateOperationCommand,
} from './undo';

// ============================================================================
// TYPES & INTERFACES
// ============================================================================

/** Unique, immutable identifier for operations (UUID/nanoid/ulid format) */
export type OperationId = string;

/**
 * Controls how an operation reacts to upstream changes
 * - 'auto': Re-render when any input changes (default)
 * - 'manual': Never re-render unless explicitly triggered
 * - 'frozen': Treat last output as immutable, cut invalidation chain
 */
export type RenderPolicy = 'auto' | 'manual' | 'frozen';

export interface OperationsState {
  defs: Record<OperationId, OperationDef>; // serialized operation definitions keyed by ID
  order?: OperationId[]; // optional ordering of operations for UI display
  pipelines?: Record<string, OperationId[]>; // optional grouping of operations into pipelines
  _version?: number;
}

/**
 * An operation can be either a file rendering operation (produces output files)
 * or a sample editing operation (modifies audio in-place).
 *
 * IMPORTANT: Operations have:
 * - `id`: Immutable, unique identifier (used for lookup and references)
 * - `name`: Mutable, user-visible display label
 */
export type OperationDef = MergeOp | PipelineOp | SampleOp;

/** Base interface for all operations with common fields */
export interface BaseOperation {
  id: OperationId; // stable, immutable identity (UUID/nanoid)
  name: string; // user-visible, editable display label

  /**
   * Controls how this operation reacts to upstream changes.
   * - 'auto': Re-render when any input changes (default)
   * - 'manual': Never re-render unless explicitly triggered
   * - 'frozen': Treat last output as immutable, don't propagate invalidations
   *
   * Note: Frozen ops still resolve sources and cache outputs, they just don't
   * automatically re-render when upstream inputs change. This cuts the invalidation
   * chain in the dependency graph.
   */
  renderPolicy?: RenderPolicy;
}

// ============================================================================
// FILE RENDERING OPERATIONS (produce new output files)
// ============================================================================

export interface SampleOp extends BaseOperation {
  kind: 'sample';
  sources: OperationSource[];
  /** Schema-driven operation parameters */
  params?: Record<string, unknown>;
}

export interface MergeOp extends BaseOperation {
  kind: 'merge';
  sources: OperationSource[];
  outputPath: string;
  format: string;
  /** Schema-driven operation parameters */
  params?: Record<string, unknown>;
}

export interface PipelineOp extends BaseOperation {
  kind: 'pipeline';
  /** Ordered list of operation IDs to execute in sequence */
  operations: OperationId[];
  /** Sources for the first operation in the pipeline */
  sources: OperationSource[];
  /** Schema-driven operation parameters */
  params?: Record<string, unknown>;
}

export type OperationSource =
  | { type: 'group'; groupRef: string }
  | { type: 'file'; fileId: string }
  | { type: 'files'; fileIds: string[] }
  | { type: 'all' }
  | { type: 'active' }
  | { type: 'section'; sectionIndex: number }
  | { type: 'operation'; operationId: OperationId }
  | { type: 'previousOperation'; operationId: OperationId };

export type AudioFormat = 'wav' | 'mp3' | 'flac' | 'ogg' | 'aiff';

export type OperationStatus = 'pending' | 'running' | 'completed' | 'failed' | 'cancelled';

export interface OperationResult {
  operationId: OperationId;
  status: OperationStatus;
  progress?: number; // 0-100
  message?: string;
  error?: string;
  outputFiles?: string[];
  startTime?: number;
  endTime?: number;
}

// ============================================================================
// ID GENERATION
// ============================================================================

/**
 * Generate a unique operation ID using timestamp + random component
 * Format: op_<timestamp>_<random> for debugging friendliness
 */
export function generateOperationId(): OperationId {
  const timestamp = Date.now().toString(36);
  const random = Math.random().toString(36).substring(2, 8);
  return `op_${timestamp}_${random}`;
}

// ============================================================================
// OPERATION INFO DICTIONARY (for UI)
// ============================================================================

export interface OperationInfo {
  icon: string;
  label: string;
  description: string;
  category: 'render' | 'edit' | 'meta';
  params: string[];
}

export const OperationInfoDictionary: Record<OperationDef['kind'], OperationInfo> = {
  merge: {
    icon: '➕',
    label: 'Merge',
    description: 'Concatenate multiple audio files into a single output file',
    category: 'render',
    params: ['source', 'outputPath', 'format'],
  },
  pipeline: {
    icon: '🔀',
    label: 'Pipeline',
    description: 'Chain multiple operations together in sequence',
    category: 'meta',
    params: ['sources', 'operations'],
  },
  sample: {
    icon: '🔊',
    label: 'Sample',
    description: 'Simple audio file playback',
    category: 'render',
    params: ['source'],
  },
};

// ============================================================================
// OPERATION REGISTRY (similar to GroupRegistry)
// ============================================================================

export type OperationExecutor = (
  state: AppState,
  operation: OperationDef,
  fileIds: Set<string>
) => Promise<OperationResult>;

export class OperationRegistry {
  private cache = new Map<OperationId, { version: number; fileIds: Set<string> }>();

  constructor(private getDefs: () => Record<OperationId, OperationDef> | undefined) {}

  /**
   * Resolve the source files for an operation by ID
   */
  resolveSource(id: OperationId, state: AppState): Set<string> {
    const version = state._version ?? 0;
    const isLogging = get(loggingState).operationsLog;

    // Check cache
    const cached = this.cache.get(id);
    if (cached && cached.version === version) {
      if (isLogging) {
        console.log(`💾 Operations: Using cached source for id="${id}" (version ${version})`);
      }
      return cached.fileIds;
    }

    const defs = this.getDefs();
    const def = defs?.[id];

    if (!def) {
      if (isLogging) {
        console.error(`❌ Operations: Unknown operation id="${id}"`);
      }
      throw new Error(`Unknown operation id="${id}"`);
    }

    if (isLogging) {
      console.log(`🔍 Operations: Resolving source for id="${id}" name="${def.name}"`, def);
    }

    let fileIds: Set<string>;

    if (def.kind === 'merge' || def.kind === 'sample' || def.kind === 'pipeline') {
      // All operations now have sources array
      const allFileIds = new Set<string>();
      for (const source of def.sources) {
        const sourceFileIds = this.resolveOperationSource(source, state, id);
        sourceFileIds.forEach(id => allFileIds.add(id));
      }
      fileIds = allFileIds;
    } else {
      fileIds = new Set();
    }

    this.cache.set(id, { version, fileIds });

    if (isLogging) {
      console.log(
        `✅ Operations: Resolved id="${id}" -> ${fileIds.size} files`,
        Array.from(fileIds)
      );
    }

    return fileIds;
  }

  /**
   * Resolve an OperationSource to a set of file IDs
   */
  private resolveOperationSource(
    source: OperationSource,
    state: AppState,
    operationId?: OperationId
  ): Set<string> {
    switch (source.type) {
      case 'group':
        return groupRegistry.eval(source.groupRef, state);

      case 'file':
        return new Set([source.fileId]);

      case 'files':
        return new Set(source.fileIds);

      case 'operation':
        // Reference to another operation's output by ID
        console.warn('operation source type requires execution context');
        return new Set();

      case 'all':
      case 'active':
      case 'section':
        // These cases need different implementation
        console.warn(`Operation source type "${source.type}" needs updated implementation`);
        return new Set();

      case 'previousOperation':
        // For pipeline operations, this would reference output from previous op
        console.warn('previousOperation source type requires execution context');
        return new Set();

      default:
        return new Set();
    }
  }

  /**
   * Get the definition for an operation by ID
   */
  getDefinition(id: OperationId): OperationDef | undefined {
    return this.getDefs()?.[id];
  }

  /**
   * Get all operation IDs
   */
  getOperationIds(): OperationId[] {
    const defs = this.getDefs();
    return defs ? Object.keys(defs) : [];
  }

  /**
   * @deprecated Use getOperationIds() instead
   */
  getOperationNames(): string[] {
    return this.getOperationIds();
  }

  /**
   * Invalidate all cached source resolutions
   */
  invalidateAll() {
    const isLogging = get(loggingState).operationsLog;
    const cacheSize = this.cache.size;

    this.cache.clear();

    if (isLogging) {
      console.log(`🗑️ Operations: Invalidated ${cacheSize} cached entries`);
    }
  }
}

// ============================================================================
// SINGLETON REGISTRY INSTANCE
// ============================================================================

export const operationRegistry = new OperationRegistry(() => {
  return get(appState).operations?.defs;
});

let lastRev: number | undefined;
let subscriptionInitialized = false;

/**
 * Initialize the appState subscription for cache invalidation.
 * Call this after all modules are loaded to avoid circular dependencies.
 */
export function initializeOperationsSubscription() {
  if (subscriptionInitialized) return;

  lastRev = get(appState)._rev ?? 0;
  subscriptionInitialized = true;

  appState.subscribe(state => {
    const currentRev = state._rev ?? 0;
    const isLogging = get(loggingState).operationsLog;

    if (currentRev !== lastRev) {
      if (isLogging) {
        console.log(
          `🔄 Operations: Content revision changed from ${lastRev} to ${currentRev} - invalidating cache`
        );
      }

      operationRegistry.invalidateAll();
      lastRev = currentRev;
    }
  });
}

// ============================================================================
// CRUD OPERATIONS FOR OPERATIONS STATE
// ============================================================================

/** Type-safe definitions for creating new operations without id/name */
export type NewMergeOp = Omit<MergeOp, 'id' | 'name'>;
export type NewPipelineOp = Omit<PipelineOp, 'id' | 'name'>;
export type NewSampleOp = Omit<SampleOp, 'id' | 'name'>;
export type NewOperationDef = NewMergeOp | NewPipelineOp | NewSampleOp;

/**
 * Create and add a new operation definition.
 * Generates a new unique ID for the operation.
 *
 * @param name - Display name for the operation (user-visible, editable)
 * @param defWithoutId - Operation definition without id/name (these will be set automatically)
 * @param renderPolicy - Optional render policy override (defaults to 'auto')
 * @returns The generated operation ID
 */
export function createOperation(
  name: string,
  defWithoutId: NewOperationDef,
  renderPolicy: RenderPolicy = 'auto'
): OperationId {
  const isLogging = get(loggingState).operationsLog;
  const id = generateOperationId();

  const def = {
    renderPolicy,
    ...defWithoutId,
    id,
    name,
  } as OperationDef;

  if (isLogging) {
    console.log(
      `➕ Operations: Creating operation id="${id}" name="${name}" renderPolicy="${renderPolicy}"`,
      def
    );
  }

  appState.update(s => {
    if (!s.operations) {
      s.operations = { defs: {}, order: [], _version: 1 };
    }
    if (!s.operations.order) {
      s.operations.order = Object.keys(s.operations.defs);
    }

    // Store by ID, ensure id matches the key
    s.operations.defs[id] = def;
    s.operations.order.push(id);
    s.operations._version = (s.operations._version ?? 0) + 1;
    s._rev = (s._rev ?? 0) + 1;

    return s;
  });

  return id;
}

/**
 * Update an existing operation's parameters by ID using the undo/redo system
 */
export function updateOperationById(
  id: OperationId,
  patch: Partial<Omit<OperationDef, 'id'>>,
  expectedKind?: OperationDef['kind'],
  label?: string
): void {
  const isLogging = get(loggingState).operationsLog;

  if (isLogging) {
    console.log(`📝 Operations: Updating operation id="${id}"`, { patch, expectedKind });
  }

  // Validate operation exists and kind matches before dispatching
  const def = get(appState).operations?.defs?.[id];
  if (!def) {
    if (isLogging) {
      console.warn(`⚠️ Operations: Cannot update id="${id}" - not found`);
    }
    return;
  }

  if (expectedKind && def.kind !== expectedKind) {
    if (isLogging) {
      console.warn(
        `⚠️ Operations: Cannot update id="${id}" - expected kind "${expectedKind}" but got "${def.kind}"`
      );
    }
    return;
  }

  // Dispatch undoable command
  const command: UpdateOperationCommand = {
    type: 'update-operation',
    operationId: id,
    patch,
  };

  // Generate label if not provided
  const commandLabel =
    label ||
    (patch.name
      ? `Rename Operation to "${patch.name}"`
      : Object.keys(patch).length === 1
        ? `Update ${Object.keys(patch)[0]}`
        : 'Update Operation');

  dispatch(command, commandLabel);
}

/**
 * Rename an operation (change its display name, not its ID)
 */
export function renameOperation(id: OperationId, newName: string): void {
  updateOperationById(id, { name: newName });
}

/**
 * Delete multiple operations by their IDs using the command pattern for undo/redo support
 */
export function deleteOperationsById(ids: OperationId[]): void {
  const command: DeleteMultipleOperationsCommand = {
    type: 'delete-multiple-operations',
    operationIds: ids,
  };
  dispatch(command);
}

/**
 * Delete a single operation by ID using the command pattern for undo/redo support
 */
export function deleteOperationById(id: OperationId): void {
  const command: DeleteOperationCommand = {
    type: 'delete-operation',
    operationId: id,
  };
  dispatch(command);
}

/**
 * @deprecated Use deleteOperationById() instead
 */
export function deleteOperation(idOrName: string): void {
  deleteOperations([idOrName]);
}

/**
 * @deprecated Use deleteOperationsById() instead
 * Delete multiple operations - accepts either IDs or names for backward compatibility
 */
export function deleteOperations(idsOrNames: string[]): void {
  const state = get(appState);
  const defs = state.operations?.defs;

  // Resolve to IDs
  const ids: OperationId[] = idsOrNames.map(idOrName => {
    if (defs?.[idOrName]) {
      return idOrName; // Already an ID
    }
    // Try to find by name
    const entry = Object.entries(defs || {}).find(([, def]) => def.name === idOrName);
    return entry ? entry[0] : idOrName; // Return the found ID or the original string
  });

  deleteOperationsById(ids);
}

/**
 * Delete all operations
 */
export function deleteAllOperations(): void {
  const isLogging = get(loggingState).operationsLog;

  if (isLogging) {
    console.log(`🗑️ Operations: Deleting all operations`);
  }

  appState.update(s => {
    if (!s.operations) {
      s.operations = { defs: {}, order: [], _version: 1 };
    }

    s.operations.defs = {};
    s.operations.order = [];
    s.operations.pipelines = {};
    s.operations._version = (s.operations._version ?? 0) + 1;
    s._rev = (s._rev ?? 0) + 1;

    return s;
  });
}

/**
 * Add operation ID to a pipeline
 */
export function addToPipeline(pipelineName: string, operationId: OperationId): void {
  appState.update(s => {
    if (!s.operations) {
      s.operations = { defs: {}, pipelines: {}, order: [], _version: 1 };
    }
    if (!s.operations.pipelines) {
      s.operations.pipelines = {};
    }
    if (!s.operations.pipelines[pipelineName]) {
      s.operations.pipelines[pipelineName] = [];
    }

    if (!s.operations.pipelines[pipelineName].includes(operationId)) {
      s.operations.pipelines[pipelineName].push(operationId);
      s.operations._version = (s.operations._version ?? 0) + 1;
      s._rev = (s._rev ?? 0) + 1;
    }

    return s;
  });
}

// ============================================================================
// RENDER POLICY MANAGEMENT
// ============================================================================

/**
 * Update the render policy for an operation
 */
export function setRenderPolicy(id: OperationId, policy: RenderPolicy): void {
  updateOperationById(id, { renderPolicy: policy });
}

/**
 * Toggle an operation between 'auto' and 'frozen' render policies
 */
export function toggleFreezeOperation(id: OperationId): void {
  const op = getOperationById(id);
  if (!op) return;

  const currentPolicy = op.renderPolicy || 'auto';
  const newPolicy: RenderPolicy = currentPolicy === 'frozen' ? 'auto' : 'frozen';

  setRenderPolicy(id, newPolicy);

  const isLogging = get(loggingState).operationsLog;
  if (isLogging) {
    console.log(
      `🧊 Operations: Toggled freeze for id="${id}" from "${currentPolicy}" to "${newPolicy}"`
    );
  }
}

/**
 * Check if an operation should re-render based on its render policy
 *
 * @param op - The operation to check
 * @param upstreamChanged - Whether any upstream dependencies have changed
 * @returns true if the operation should re-render
 */
export function shouldRerender(op: OperationDef, upstreamChanged: boolean): boolean {
  const policy = op.renderPolicy || 'auto';

  switch (policy) {
    case 'manual':
      return false; // Never auto-rerender
    case 'frozen':
      return false; // Treat output as immutable
    case 'auto':
    default:
      return upstreamChanged; // Re-render when inputs change
  }
}

/**
 * Get all upstream operation IDs that this operation depends on
 * Used for dependency graph traversal and invalidation
 */
export function getUpstreamOps(opId: OperationId): OperationId[] {
  const op = getOperationById(opId);
  if (!op) return [];

  const upstreamIds: OperationId[] = [];

  // Collect from sources
  if ('sources' in op && Array.isArray(op.sources)) {
    for (const source of op.sources) {
      if (source.type === 'operation' || source.type === 'previousOperation') {
        upstreamIds.push(source.operationId);
      }
    }
  }

  // Collect from pipeline operations
  if (op.kind === 'pipeline') {
    upstreamIds.push(...op.operations);
  }

  return upstreamIds;
}

/**
 * Get all downstream operation IDs that depend on this operation
 * Used for invalidation propagation
 */
export function getDownstreamOps(opId: OperationId): OperationId[] {
  const allOps = getAllOperations();
  const downstreamIds: OperationId[] = [];

  for (const op of allOps) {
    const upstreamOfThis = getUpstreamOps(op.id);
    if (upstreamOfThis.includes(opId)) {
      downstreamIds.push(op.id);
    }
  }

  return downstreamIds;
}

/**
 * Compute which operations need to be re-rendered after a change to a specific operation
 * Respects frozen operations as invalidation chain barriers
 *
 * @param changedOpId - The ID of the operation that changed
 * @returns Set of operation IDs that need re-rendering
 */
export function computeInvalidatedOps(changedOpId: OperationId): Set<OperationId> {
  const invalidated = new Set<OperationId>();
  const visited = new Set<OperationId>();
  const queue: OperationId[] = [changedOpId];

  while (queue.length > 0) {
    const currentId = queue.shift()!;

    if (visited.has(currentId)) continue;
    visited.add(currentId);

    const currentOp = getOperationById(currentId);
    if (!currentOp) continue;

    // Mark as invalidated (unless it's the root change and it's frozen)
    if (currentId !== changedOpId) {
      invalidated.add(currentId);
    }

    // Check if we should propagate further downstream
    const policy = currentOp.renderPolicy || 'auto';

    // Frozen operations cut the invalidation chain
    // Don't propagate invalidation past frozen nodes
    if (policy === 'frozen' && currentId !== changedOpId) {
      const isLogging = get(loggingState).operationsLog;
      if (isLogging) {
        console.log(`🧊 Operations: Invalidation chain stopped at frozen op id="${currentId}"`);
      }
      continue; // Don't propagate past this node
    }

    // Get downstream operations and add them to the queue
    const downstream = getDownstreamOps(currentId);
    for (const downstreamId of downstream) {
      if (!visited.has(downstreamId)) {
        queue.push(downstreamId);
      }
    }
  }

  const isLogging = get(loggingState).operationsLog;
  if (isLogging && invalidated.size > 0) {
    console.log(
      `🔄 Operations: Change to id="${changedOpId}" invalidated ${invalidated.size} operations:`,
      Array.from(invalidated)
    );
  }

  return invalidated;
}

// ============================================================================
// LOOKUP HELPERS
// ============================================================================

/**
 * Get an operation by its ID
 */
export function getOperationById(id: OperationId): OperationDef | undefined {
  return get(appState).operations?.defs?.[id];
}

/**
 * Get an operation by its display name (for backward compatibility)
 * Note: Names are not guaranteed to be unique!
 */
export function getOperationByName(name: string): OperationDef | undefined {
  const defs = get(appState).operations?.defs;
  if (!defs) return undefined;

  const entry = Object.entries(defs).find(([, def]) => def.name === name);
  return entry?.[1];
}

/**
 * Get all operations as an array, optionally in order
 */
export function getAllOperations(): OperationDef[] {
  const state = get(appState);
  const defs = state.operations?.defs;
  const order = state.operations?.order;

  if (!defs) return [];

  if (order) {
    return order.map(id => defs[id]).filter((def): def is OperationDef => def !== undefined);
  }

  return Object.values(defs);
}

/**
 * Get the operation IDs from timeline selection
 * Use this to safely delete operations from timeline selection
 */
export function getOperationIdsFromTimelineItems(
  timelineItems: { operationId: OperationId }[],
  selectedItemIds: Set<string>
): Set<OperationId> {
  const opIds = new Set<OperationId>();

  for (const item of timelineItems) {
    if (selectedItemIds.has(item.operationId)) {
      opIds.add(item.operationId);
    }
  }

  return opIds;
}

/** @deprecated Use OperationDef directly - operations now have id and name properties */
export interface NamedOperationDef {
  name: string;
  def: OperationDef;
}

export function removeOperationsFromCurrentOp(operationIdsToRemove: OperationId[]): void {
  const isLogging = get(loggingState).operationsLog;

  if (isLogging) {
    console.log(`🗑️ Operations: Removing operations from current op:`, operationIdsToRemove);
  }

  appState.update(s => {
    const selectedOpId = s.uiSettings?.selectedOperationId;

    if (!selectedOpId) {
      if (isLogging) {
        console.warn(`⚠️ Operations: No operation selected, cannot remove sources`);
      }
      return s;
    }

    const currentOp = s.operations?.defs?.[selectedOpId];
    if (!currentOp) {
      if (isLogging) {
        console.warn(`⚠️ Operations: Current operation "${selectedOpId}" not found`);
      }
      return s;
    }

    if (currentOp.kind !== 'merge') {
      if (isLogging) {
        console.warn(
          `⚠️ Operations: Current operation "${selectedOpId}" is not a MergeOp, cannot remove sources`
        );
      }
      return s;
    }

    // Filter out the sources that match the operation IDs to remove
    const originalSourceCount = currentOp.sources.length;
    const newSources = currentOp.sources.filter(source => {
      if (source.type === 'operation') {
        return !operationIdsToRemove.includes(source.operationId);
      }
      // Keep non-operation sources (like file sources)
      return true;
    });

    const removedCount = originalSourceCount - newSources.length;

    if (removedCount === 0) {
      if (isLogging) {
        console.log(`📝 Operations: No matching sources found to remove from "${selectedOpId}"`);
      }
      return s;
    }

    // Update the current operation with the filtered sources
    s.operations!.defs[selectedOpId] = {
      ...currentOp,
      sources: newSources,
    } as MergeOp;

    s.operations!._version = (s.operations!._version ?? 0) + 1;
    s._rev = (s._rev ?? 0) + 1;

    if (isLogging) {
      console.log(`✅ Operations: Removed ${removedCount} source(s) from "${selectedOpId}"`);
    }

    return s;
  });
}
