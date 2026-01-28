/**
 * Auto-Render Service
 *
 * This module provides automatic rendering of operations when the app state revision (_rev) changes.
 * It monitors _rev changes and triggers backend rendering for operations with renderPolicy: 'auto'.
 *
 * Usage:
 * - Call initializeAutoRenderSubscription() once during app initialization
 * - The service will automatically detect _rev changes and trigger renders
 * - Use setAutoRenderEnabled() to enable/disable the feature
 * - Use triggerManualRender() to force a render cycle
 *
 * ## Tracking Individual Operation Render Status
 *
 * The service maintains detailed per-operation render state that can be used in UI components:
 *
 * ### Example: Display render status in an operation component
 *
 * ```typescript
 * import { createOperationRenderStore, type OperationRenderState } from '$lib/state/autoRender';
 *
 * export let operationId: string;
 *
 * // Create a reactive store for this operation's render state
 * const renderState = createOperationRenderStore(operationId);
 *
 * // Use it in your template:
 * {#if $renderState}
 *   {#if $renderState.status === 'rendering'}
 *     <div>⏳ Rendering... ({$renderState.index}/{$renderState.totalOperations})</div>
 *   {:else if $renderState.status === 'success'}
 *     <div>✅ Rendered in {$renderState.duration_ms}ms</div>
 *   {:else if $renderState.status === 'error'}
 *     <div>❌ Error: {$renderState.error}</div>
 *   {:else if $renderState.status === 'skipped'}
 *     <div>⏭️ Skipped</div>
 *   {/if}
 * {/if}
 * ```
 *
 * ### Example: Access global render progress
 *
 * ```typescript
 * import { autoRenderProgress, autoRenderStatus } from '$lib/state/autoRender';
 *
 * // Display overall progress
 * {#if $autoRenderStatus === 'rendering'}
 *   <div>
 *     Rendering {$autoRenderProgress.completed}/{$autoRenderProgress.total}
 *     ({$autoRenderProgress.percentage.toFixed(0)}%)
 *   </div>
 * {/if}
 * ```
 *
 * ### Available Operation States:
 * - `pending`: Operation is queued but hasn't started rendering yet
 * - `rendering`: Operation is currently being rendered (set when first progress event received)
 * - `success`: Operation rendered successfully
 * - `error`: Operation failed to render
 * - `skipped`: Operation was skipped (e.g., due to renderPolicy: 'manual')
 */

import { get, derived, writable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { appState, type AppState } from './state.svelte';
import { loggingState } from './logging';
import { invokeWithPerf } from './performance';
import { createTypedEventChannelWithLoggingAndStatusMessages } from '../utils/channelMaker';

// ============================================================================
// TYPES
// ============================================================================

/** Result for a single operation render */
export interface OperationRenderResult {
  operation_id: string;
  operation_name: string;
  success: boolean;
  error: string | null;
  duration_ms: number;
}

/** Result for the batch render operation */
export interface BatchRenderResult {
  total_operations: number;
  successful_renders: number;
  failed_renders: number;
  skipped_operations: number;
  results: OperationRenderResult[];
  total_duration_ms: number;
  triggered_by_rev: number;
}

/** Status of the auto-render service */
export type AutoRenderStatus = 'idle' | 'pending' | 'rendering' | 'error';

/** Status of a single operation during rendering */
export type OperationRenderStatus = 'pending' | 'rendering' | 'success' | 'error' | 'skipped';

/** Detailed status information for a single operation */
export interface OperationRenderState {
  /** Current status of this operation */
  status: OperationRenderStatus;
  /** Operation name */
  name: string;
  /** Index in the render queue */
  index: number;
  /** Total operations in the batch */
  totalOperations: number;
  /** Error message if failed */
  error?: string;
  /** Duration in milliseconds (only set when complete) */
  duration_ms?: number;
  /** Timestamp when rendering started */
  startedAt?: number;
  /** Timestamp when rendering completed */
  completedAt?: number;
}

/** Auto-render state exposed to UI */
export interface AutoRenderState {
  /** Whether auto-render is enabled */
  enabled: boolean;
  /** Current status of the service */
  status: AutoRenderStatus;
  /** Last render result (if any) */
  lastResult: BatchRenderResult | null;
  /** Last error (if any) */
  lastError: string | null;
  /** Number of pending render requests (debounced) */
  pendingRequests: number;
  /** Last processed revision */
  lastProcessedRev: number;
  /** Map of operation ID to its current render state */
  operationStates: Record<string, OperationRenderState>;
  /** Current operation being rendered (if any) */
  currentOperationId: string | null;
  /** Total operations in current batch */
  totalOperations: number;
  /** Number of operations completed in current batch */
  completedOperations: number;
}

// ============================================================================
// STATE
// ============================================================================

/** Writable store for auto-render state */
export const autoRenderState = writable<AutoRenderState>({
  enabled: true,
  status: 'idle',
  lastResult: null,
  lastError: null,
  pendingRequests: 0,
  lastProcessedRev: 0,
  operationStates: {},
  currentOperationId: null,
  totalOperations: 0,
  completedOperations: 0,
});

// Derived stores for convenience
export const autoRenderEnabled = derived(autoRenderState, $state => $state.enabled);
export const autoRenderStatus = derived(autoRenderState, $state => $state.status);
export const autoRenderLastResult = derived(autoRenderState, $state => $state.lastResult);
export const autoRenderOperationStates = derived(autoRenderState, $state => $state.operationStates);
export const autoRenderCurrentOperation = derived(
  autoRenderState,
  $state => $state.currentOperationId
);
export const autoRenderProgress = derived(autoRenderState, $state => ({
  total: $state.totalOperations,
  completed: $state.completedOperations,
  percentage:
    $state.totalOperations > 0 ? ($state.completedOperations / $state.totalOperations) * 100 : 0,
}));

/**
 * Get the render state for a specific operation
 * @param operationId The operation ID to check
 * @returns The operation's render state, or null if not found
 */
export function getOperationRenderState(operationId: string): OperationRenderState | null {
  const state = get(autoRenderState);
  return state.operationStates[operationId] ?? null;
}

/**
 * Create a derived store for a specific operation's render state
 * @param operationId The operation ID to track
 * @returns A derived store that updates when the operation's render state changes
 */
export function createOperationRenderStore(operationId: string) {
  return derived(autoRenderState, $state => $state.operationStates[operationId] ?? null);
}

// ============================================================================
// CONFIGURATION
// ============================================================================

/** Debounce delay in milliseconds */
const DEBOUNCE_DELAY_MS = 300;

/** Maximum number of retries on failure */
const MAX_RETRIES = 3;

/** Retry delay in milliseconds */
const RETRY_DELAY_MS = 1000;

// ============================================================================
// INTERNAL STATE
// ============================================================================

let subscriptionInitialized = false;
let lastKnownRev: number = 0;
let debounceTimer: ReturnType<typeof setTimeout> | null = null;
let isRendering = false;
let retryCount = 0;

// ============================================================================
// PUBLIC API
// ============================================================================

/**
 * Initialize the auto-render subscription.
 * Call this once during app initialization (e.g., in +layout.svelte).
 */
export function initializeAutoRenderSubscription(): void {
  if (subscriptionInitialized) {
    console.warn('Auto-render subscription already initialized');
    return;
  }

  subscriptionInitialized = true;
  lastKnownRev = get(appState)._rev ?? 0;

  // Update the state
  autoRenderState.update(s => ({
    ...s,
    lastProcessedRev: lastKnownRev,
  }));

  const isLogging = get(loggingState).operationsLog;
  if (isLogging) {
    console.log(`🔄 Auto-render: Initialized with rev ${lastKnownRev}`);
  }

  // Subscribe to appState changes
  appState.subscribe(state => {
    const currentRev = state._rev ?? 0;
    const currentState = get(autoRenderState);
    console.log(`%cHERE LINE :136 %c`, 'color: yellow; font-weight: bold', '');

    if (currentRev !== lastKnownRev && currentState.enabled) {
      console.log(isLogging);
      if (isLogging) {
        console.log(`🔄 Auto-render: Rev changed from ${lastKnownRev} to ${currentRev}`);
      }

      lastKnownRev = currentRev;
      scheduleRender(currentRev, state);
    }
  });

  if (isLogging) {
    console.log('✅ Auto-render: Subscription initialized');
  }
}

/**
 * Enable or disable auto-rendering
 */
export function setAutoRenderEnabled(enabled: boolean): void {
  const isLogging = get(loggingState).operationsLog;

  autoRenderState.update(s => ({
    ...s,
    enabled,
  }));

  if (isLogging) {
    console.log(`🔄 Auto-render: ${enabled ? 'Enabled' : 'Disabled'}`);
  }

  // If re-enabling, check if we need to render
  if (enabled) {
    const currentRev = get(appState)._rev ?? 0;
    const currentState = get(autoRenderState);

    if (currentRev > currentState.lastProcessedRev) {
      scheduleRender(currentRev, get(appState));
    }
  }
}

/**
 * Toggle auto-rendering on/off
 */
export function toggleAutoRender(): void {
  const currentState = get(autoRenderState);
  setAutoRenderEnabled(!currentState.enabled);
}

/**
 * Manually trigger a render cycle for all auto operations
 */
export async function triggerManualRender(forceAll: boolean = false): Promise<BatchRenderResult> {
  const isLogging = get(loggingState).operationsLog;

  if (isLogging) {
    console.log(`🔄 Auto-render: Manual render triggered (forceAll: ${forceAll})`);
  }

  const currentRev = get(appState)._rev ?? 0;
  return executeRender(currentRev, get(appState), forceAll);
}

/**
 * Render specific operations by ID
 */
export async function renderOperations(
  operationIds: string[],
  force: boolean = false
): Promise<BatchRenderResult> {
  const isLogging = get(loggingState).operationsLog;

  if (isLogging) {
    console.log(`🔄 Auto-render: Rendering ${operationIds.length} specific operations`);
  }

  const state = get(appState);
  const currentRev = state._rev ?? 0;

  return executeRenderWithOptions(currentRev, state, {
    specificOperationIds: operationIds,
    forceRender: force,
  });
}

/**
 * Get the current auto-render state
 */
export function getAutoRenderState(): AutoRenderState {
  return get(autoRenderState);
}

// ============================================================================
// INTERNAL FUNCTIONS
// ============================================================================

/**
 * Schedule a render with debouncing
 */
function scheduleRender(rev: number, state: AppState): void {
  const isLogging = get(loggingState).operationsLog;

  // Cancel any pending debounce
  if (debounceTimer !== null) {
    clearTimeout(debounceTimer);
  }

  // Update pending count
  autoRenderState.update(s => ({
    ...s,
    pendingRequests: s.pendingRequests + 1,
    status: 'pending',
  }));

  // Schedule the render
  debounceTimer = setTimeout(async () => {
    debounceTimer = null;

    // Reset pending count
    autoRenderState.update(s => ({
      ...s,
      pendingRequests: 0,
    }));

    if (isLogging) {
      console.log(`🔄 Auto-render: Executing debounced render for rev ${rev}`);
    }

    await executeRender(rev, state, false);
  }, DEBOUNCE_DELAY_MS);
}

/**
 * Execute the actual render
 */
async function executeRender(
  rev: number,
  state: AppState,
  forceAll: boolean
): Promise<BatchRenderResult> {
  return executeRenderWithOptions(rev, state, { forceRender: forceAll });
}

interface RenderOptions {
  specificOperationIds?: string[];
  forceRender?: boolean;
}

async function executeRenderWithOptions(
  rev: number,
  state: AppState,
  options: RenderOptions
): Promise<BatchRenderResult> {
  const isLogging = get(loggingState).operationsLog;

  // Prevent concurrent renders
  if (isRendering) {
    if (isLogging) {
      console.log('⏳ Auto-render: Render already in progress, skipping');
    }
    return {
      total_operations: 0,
      successful_renders: 0,
      failed_renders: 0,
      skipped_operations: 0,
      results: [],
      total_duration_ms: 0,
      triggered_by_rev: rev,
    };
  }

  isRendering = true;
  autoRenderState.update(s => ({
    ...s,
    status: 'rendering',
    lastError: null,
  }));

  try {
    // Get the current operations state
    const operationsState = state.operations ?? { defs: {}, order: [] };

    // Prepare the parameters for the backend command
    const params = {
      operations_state: operationsState,
      current_rev: rev,
      specific_operation_ids: options.specificOperationIds ?? null,
      force_render: options.forceRender ?? false,
    };

    if (isLogging) {
      console.log('🔄 Auto-render: Calling backend render_all_auto_operations', {
        operationsCount: Object.keys(operationsState.defs).length,
        rev,
        forceRender: options.forceRender,
        specificIds: options.specificOperationIds,
      });
    }

    // Create a typed event channel to receive progress updates from the backend
    type AutoRenderStartedEvent = { event: 'started'; data: { total_operations: number } };
    type AutoRenderProgressEvent = {
      event: 'progress';
      data: {
        operation_index: number;
        total_operations: number;
        operation_id: string;
        operation_name: string;
        success: boolean;
      };
    };
    type AutoRenderFinishedEvent = { event: 'finished'; data: { result: BatchRenderResult } };
    type AutoRenderEvent =
      | AutoRenderStartedEvent
      | AutoRenderProgressEvent
      | AutoRenderFinishedEvent;

    const onEvent = createTypedEventChannelWithLoggingAndStatusMessages<AutoRenderEvent>(
      'AutoRender',
      {
        source: 'auto-render',
        startedMessage: data => `Rendering ${data.total_operations} operations...`,
        progressMessage: data =>
          `Rendering: ${data.operation_name} (${data.operation_index}/${data.total_operations})`,
        finishedMessage: data =>
          `Rendered ${data.result.successful_renders}/${data.result.total_operations} ops in ${data.result.total_duration_ms}ms`,
        getProgress: (data: any) => {
          if (typeof data.operation_index === 'number' && data.total_operations) {
            return data.operation_index / data.total_operations;
          }
          return undefined;
        },
        autoClearSuccess: 2000,
      },
      {
        onStarted: data => {
          if (isLogging) console.log('📡 AutoRender Started', data);

          // Reset operation states and set total
          autoRenderState.update(s => ({
            ...s,
            status: 'rendering',
            totalOperations: data.total_operations,
            completedOperations: 0,
            operationStates: {},
            currentOperationId: null,
          }));
        },
        onProgress: data => {
          if (isLogging) console.log('📡 AutoRender Progress', data);

          // Update operation-specific state
          autoRenderState.update(s => {
            // Check if this is the first time we're seeing this operation (start rendering)
            const previousState = s.operationStates[data.operation_id];
            const isNewOperation = !previousState;

            const operationState: OperationRenderState = {
              status: data.success ? 'success' : 'error',
              name: data.operation_name,
              index: data.operation_index,
              totalOperations: data.total_operations,
              error: data.success ? undefined : 'Render failed',
              startedAt: previousState?.startedAt ?? Date.now(),
              completedAt: Date.now(),
            };

            // Calculate duration if we have both timestamps
            if (operationState.startedAt && operationState.completedAt) {
              operationState.duration_ms = operationState.completedAt - operationState.startedAt;
            }

            return {
              ...s,
              currentOperationId: data.operation_id,
              completedOperations: data.operation_index,
              operationStates: {
                ...s.operationStates,
                [data.operation_id]: operationState,
              },
            };
          });
        },
        onFinished: data => {
          if (isLogging) console.log('📡 AutoRender Finished', data);

          // Update all operation states with final results
          autoRenderState.update(s => {
            const updatedOperationStates = { ...s.operationStates };

            // Update states based on the final result
            data.result.results.forEach(result => {
              const existingState = updatedOperationStates[result.operation_id];
              updatedOperationStates[result.operation_id] = {
                ...existingState,
                status: result.success
                  ? 'success'
                  : result.error?.includes('Skipped')
                    ? 'skipped'
                    : 'error',
                name: result.operation_name,
                error: result.error ?? undefined,
                duration_ms: result.duration_ms,
                completedAt: existingState?.completedAt ?? Date.now(),
              } as OperationRenderState;
            });

            return {
              ...s,
              status: 'idle',
              currentOperationId: null,
              completedOperations: data.result.total_operations,
              operationStates: updatedOperationStates,
            };
          });
        },
      }
    );

    // Invoke the backend command and pass the onEvent channel so the backend can send progress
    const invokeResult = await invokeWithPerf<BatchRenderResult>('render_all_auto_operations', {
      params,
      onEvent,
    });

    if (!invokeResult.ok) {
      throw new Error(invokeResult.error?.message ?? 'render_all_auto_operations failed');
    }

    const result = invokeResult.value;

    if (isLogging) {
      console.log('✅ Auto-render: Render completed', {
        total: result.total_operations,
        successful: result.successful_renders,
        failed: result.failed_renders,
        skipped: result.skipped_operations,
        duration: `${result.total_duration_ms}ms`,
      });
    }

    // Update state with success
    autoRenderState.update(s => ({
      ...s,
      status: 'idle',
      lastResult: result,
      lastProcessedRev: rev,
    }));

    // Reset retry count on success
    retryCount = 0;

    return result;
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error);

    if (isLogging) {
      console.error('❌ Auto-render: Render failed', errorMessage);
    }

    autoRenderState.update(s => ({
      ...s,
      status: 'error',
      lastError: errorMessage,
    }));

    // Retry logic
    if (retryCount < MAX_RETRIES) {
      retryCount++;
      if (isLogging) {
        console.log(
          `🔄 Auto-render: Retrying in ${RETRY_DELAY_MS}ms (attempt ${retryCount}/${MAX_RETRIES})`
        );
      }

      await new Promise(resolve => setTimeout(resolve, RETRY_DELAY_MS));
      return executeRenderWithOptions(rev, state, options);
    }

    // Reset retry count after max retries
    retryCount = 0;

    return {
      total_operations: 0,
      successful_renders: 0,
      failed_renders: 1,
      skipped_operations: 0,
      results: [
        {
          operation_id: 'batch',
          operation_name: 'Batch Render',
          success: false,
          error: errorMessage,
          duration_ms: 0,
        },
      ],
      total_duration_ms: 0,
      triggered_by_rev: rev,
    };
  } finally {
    isRendering = false;
  }
}
