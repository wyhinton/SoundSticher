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
 */

import { get, derived, writable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { appState, type AppState } from './state.svelte';
import { loggingState } from './logging';

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
});

// Derived stores for convenience
export const autoRenderEnabled = derived(autoRenderState, $state => $state.enabled);
export const autoRenderStatus = derived(autoRenderState, $state => $state.status);
export const autoRenderLastResult = derived(autoRenderState, $state => $state.lastResult);

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

    if (currentRev !== lastKnownRev && currentState.enabled) {
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

    // Invoke the backend command
    const result = await invoke<BatchRenderResult>('render_all_auto_operations', { params });

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
