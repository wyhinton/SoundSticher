// Waveform cache service for frontend
//
// This mirrors the Rust-side waveform cache and provides:
// - Local caching of waveforms by audio key
// - Batch requests when switching operations
// - Reactive stores for timeline items derived from operations

import { invoke } from '@tauri-apps/api/core';
import { writable, derived, get, type Writable, type Readable } from 'svelte/store';
import { appState, type TimelineItem, type AudioFileTimelineItem } from './state.svelte';
import type { OperationDef, CombineOperation } from './operation';
import { logger } from './logging';

// ============================================================================
// TYPES
// ============================================================================

export interface AudioKey {
  sourceId: string;
  contentHash: number;
}

export interface Waveform {
  svgPath: string;
  peaks: [number, number][];
  sampleRate: number;
  duration: number;
  sampleCount: number;
  width: number;
  height: number;
}

export interface WaveformSpec {
  width: number;
  height: number;
  normalize: boolean;
}

export interface WaveformResponse {
  audioKey: AudioKey;
  waveform: Waveform;
  cacheHit: boolean;
}

export interface BatchWaveformItem {
  filePath: string;
  audioKey: AudioKey;
  waveform: Waveform | null;
  error: string | null;
  cacheHit: boolean;
}

export interface BatchWaveformResponse {
  items: BatchWaveformItem[];
  totalCacheHits: number;
  totalComputed: number;
  totalErrors: number;
}

export interface CacheStats {
  hits: number;
  misses: number;
  evictions: number;
  totalComputeTimeMs: number;
}

// Local cache key generation
function createCacheKey(filePath: string, width: number, height: number): string {
  return `${filePath}:${width}:${height}`;
}

// ============================================================================
// WAVEFORM CACHE CLASS
// ============================================================================

export class WaveformCache {
  private cache = new Map<string, Waveform>();
  private inFlight = new Map<string, Promise<Waveform>>();
  private maxEntries: number;

  constructor(maxEntries: number = 500) {
    this.maxEntries = maxEntries;
  }

  /**
   * Get a waveform from cache or fetch from backend
   */
  async getOrFetch(
    filePath: string,
    spec: WaveformSpec = { width: 1000, height: 70, normalize: false }
  ): Promise<Waveform> {
    const key = createCacheKey(filePath, spec.width, spec.height);
    console.log(`%cHERE LINE :93 %c`, 'color: yellow; font-weight: bold', '');

    // Check local cache first
    const cached = this.cache.get(key);
    if (cached) {
      logger.waveform.cache(`Cache hit for ${filePath} (${spec.width}x${spec.height})`);
      return cached;
    }

    // Check if already in-flight
    const inFlight = this.inFlight.get(key);
    if (inFlight) {
      logger.waveform.fetch(`Request already in-flight for ${filePath}, waiting...`);
      return inFlight;
    }

    logger.waveform.fetch(`Fetching waveform for ${filePath} (${spec.width}x${spec.height})`);

    // Create new request
    const promise = this.fetchWaveform(filePath, spec)
      .then(waveform => {
        this.cache.set(key, waveform);
        this.inFlight.delete(key);
        this.evictIfNeeded();
        logger.waveform.success(
          `Waveform cached for ${filePath} (duration: ${waveform.duration}s)`
        );
        return waveform;
      })
      .catch(error => {
        this.inFlight.delete(key);
        logger.waveform.error(`Failed to fetch waveform for ${filePath}:`, error);
        throw error;
      });

    this.inFlight.set(key, promise);
    return promise;
  }

  /**
   * Fetch waveform from backend
   */
  private async fetchWaveform(filePath: string, spec: WaveformSpec): Promise<Waveform> {
    const response = await invoke<WaveformResponse>('get_waveform', {
      request: {
        filePath,
        width: spec.width,
        height: spec.height,
        normalize: spec.normalize,
      },
    });

    return response.waveform;
  }

  /**
   * Batch fetch waveforms for multiple files
   */
  async getBatch(
    filePaths: string[],
    spec: WaveformSpec = { width: 1000, height: 70, normalize: false }
  ): Promise<Map<string, Waveform>> {
    logger.waveform.batch(
      `Batch request for ${filePaths.length} waveforms (${spec.width}x${spec.height})`
    );

    const result = new Map<string, Waveform>();
    const toFetch: string[] = [];

    // Check local cache first
    for (const filePath of filePaths) {
      const key = createCacheKey(filePath, spec.width, spec.height);
      const cached = this.cache.get(key);
      if (cached) {
        result.set(filePath, cached);
        logger.waveform.cache(`Batch cache hit for ${filePath}`);
      } else {
        toFetch.push(filePath);
      }
    }

    logger.waveform.batch(`${result.size} cache hits, ${toFetch.length} to fetch from backend`);

    // Fetch remaining from backend
    if (toFetch.length > 0) {
      try {
        const response = await invoke<BatchWaveformResponse>('get_waveforms_batch', {
          request: {
            filePaths: toFetch,
            width: spec.width,
            height: spec.height,
            normalize: spec.normalize,
          },
        });

        logger.waveform.batch(
          `Backend returned ${response.items.length} items (hits: ${response.totalCacheHits}, computed: ${response.totalComputed}, errors: ${response.totalErrors})`
        );

        for (const item of response.items) {
          if (item.waveform) {
            const key = createCacheKey(item.filePath, spec.width, spec.height);
            this.cache.set(key, item.waveform);
            result.set(item.filePath, item.waveform);
            logger.waveform.success(
              `Batched waveform for ${item.filePath} (${item.cacheHit ? 'backend cache hit' : 'computed'})`
            );
          } else if (item.error) {
            logger.waveform.error(`Batch error for ${item.filePath}: ${item.error}`);
          }
        }

        this.evictIfNeeded();
      } catch (error) {
        logger.waveform.error('Batch fetch failed:', error);
        throw error;
      }
    }

    logger.waveform.batch(`Batch complete: ${result.size} waveforms ready`);
    return result;
  }

  /**
   * Check if a waveform is cached locally
   */
  isCached(
    filePath: string,
    spec: WaveformSpec = { width: 1000, height: 70, normalize: false }
  ): boolean {
    const key = createCacheKey(filePath, spec.width, spec.height);
    return this.cache.has(key);
  }

  /**
   * Invalidate cached waveform for a file
   */
  invalidate(filePath: string): void {
    let removedCount = 0;
    // Remove all entries for this file (any resolution)
    for (const key of this.cache.keys()) {
      if (key.startsWith(filePath + ':')) {
        this.cache.delete(key);
        removedCount++;
      }
    }

    if (removedCount > 0) {
      logger.waveform.cache(`Invalidated ${removedCount} cache entries for ${filePath}`);
    }
  }

  /**
   * Clear all cached waveforms
   */
  clear(): void {
    const oldSize = this.cache.size;
    this.cache.clear();
    this.inFlight.clear();

    if (oldSize > 0) {
      logger.waveform.cache(`Cleared ${oldSize} waveform cache entries`);
    }
  }

  /**
   * Get cache size
   */
  get size(): number {
    return this.cache.size;
  }

  /**
   * Evict oldest entries if over max size
   */
  private evictIfNeeded(): void {
    const evicted: string[] = [];
    while (this.cache.size > this.maxEntries) {
      const firstKey = this.cache.keys().next().value;
      if (firstKey) {
        this.cache.delete(firstKey);
        evicted.push(firstKey);
      }
    }

    if (evicted.length > 0) {
      logger.waveform.cache(
        `Evicted ${evicted.length} cache entries (over max size ${this.maxEntries})`
      );
    }
  }
}

// ============================================================================
// SINGLETON INSTANCE
// ============================================================================

export const waveformCache = new WaveformCache(500);

// ============================================================================
// OPERATION WAVEFORM STORE
// ============================================================================

export interface OperationWaveforms {
  operationName: string | null;
  filePaths: string[];
  waveforms: Map<string, Waveform>;
  loading: boolean;
  error: string | null;
}

/**
 * Store that holds waveforms for the currently selected operation
 */
function createOperationWaveformStore() {
  const { subscribe, set, update } = writable<OperationWaveforms>({
    operationName: null,
    filePaths: [],
    waveforms: new Map(),
    loading: false,
    error: null,
  });

  return {
    subscribe,

    /**
     * Load waveforms for an operation
     */
    async loadForOperation(operationName: string, filePaths: string[]): Promise<void> {
      logger.waveform.operation(
        `Loading waveforms for operation "${operationName}" (${filePaths.length} files)`
      );

      update(state => ({
        ...state,
        operationName,
        filePaths,
        loading: true,
        error: null,
      }));

      try {
        const waveforms = await waveformCache.getBatch(filePaths);
        update(state => ({
          ...state,
          waveforms,
          loading: false,
        }));

        logger.waveform.operation(
          `Operation "${operationName}" waveforms loaded successfully (${waveforms.size}/${filePaths.length})`
        );
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : String(error);
        update(state => ({
          ...state,
          loading: false,
          error: errorMessage,
        }));

        logger.waveform.error(
          `Failed to load waveforms for operation "${operationName}": ${errorMessage}`
        );
      }
    },

    /**
     * Clear the current operation waveforms
     */
    clear(): void {
      logger.waveform.operation('Clearing current operation waveforms');
      set({
        operationName: null,
        filePaths: [],
        waveforms: new Map(),
        loading: false,
        error: null,
      });
    },
  };
}

export const operationWaveforms = createOperationWaveformStore();

// ============================================================================
// DERIVED TIMELINE ITEMS STORE
// ============================================================================

/**
 * Get file paths from an operation's sections
 */
function getOperationFilePaths(operation: OperationDef | undefined): string[] {
  if (!operation) return [];

  // Operations have their own sections array
  const sections = operation.sections || [];
  return sections.flatMap(section => section.files.filter(f => f.active).map(f => f.path));
}

/**
 * Get file items from an operation's sections (with metadata)
 */
function getOperationFileItems(operation: OperationDef | undefined): Array<{
  id: string;
  path: string;
  active: boolean;
  index: number;
}> {
  if (!operation) return [];

  const sections = operation.sections || [];
  return sections.flatMap(section =>
    section.files.map(f => ({
      id: f.id,
      path: f.path,
      active: f.active,
      index: f.index,
    }))
  );
}

/**
 * Derived store that provides timeline items for the currently selected operation.
 *
 * This is the key store that the Timeline component should use instead of
 * appState.timelineItems. It reactively updates when:
 * - The selected operation changes
 * - The operation's sections change
 * - Waveforms are loaded
 */
export const operationTimelineItems: Readable<TimelineItem[]> = derived(
  [appState, operationWaveforms],
  ([$appState, $operationWaveforms]) => {
    const selectedOpName = $appState.uiSettings?.selectedOperationName;

    // If no operation selected, return empty or fall back to legacy timeline items
    if (!selectedOpName || !$appState.operations?.defs) {
      // Fall back to legacy timeline items for backward compatibility
      const legacyItems = $appState.timelineItems || [];
      if (legacyItems.length > 0) {
        logger.waveform.info(`Using legacy timeline items (${legacyItems.length} items)`);
      }
      return legacyItems;
    }

    const operation = $appState.operations.defs[selectedOpName];
    if (!operation) {
      logger.waveform.warning(`Operation "${selectedOpName}" not found in definitions`);
      return [];
    }

    // Get active file items from the operation
    const fileItems = getOperationFileItems(operation).filter(f => f.active);

    if (fileItems.length === 0) {
      logger.waveform.info(`No active files in operation "${selectedOpName}"`);
      return [];
    }

    // Calculate total duration based on waveforms
    let totalDuration = 0;
    let loadedWaveforms = 0;
    for (const file of fileItems) {
      const waveform = $operationWaveforms.waveforms.get(file.path);
      if (waveform) {
        totalDuration += waveform.duration;
        loadedWaveforms++;
      }
    }

    if (loadedWaveforms < fileItems.length) {
      logger.waveform.info(
        `Operation "${selectedOpName}" has ${loadedWaveforms}/${fileItems.length} waveforms loaded`
      );
    }

    // Build timeline items with start offsets
    const items: TimelineItem[] = [];
    let currentOffset = 0;

    for (const file of fileItems) {
      const waveform = $operationWaveforms.waveforms.get(file.path);
      if (waveform) {
        const size = totalDuration > 0 ? waveform.duration / totalDuration : 0;

        items.push({
          type: 'audio-file',
          id: file.id,
          fileName: file.path,
          svgPath: waveform.svgPath,
          startOffset: currentOffset,
          size,
          active: file.active,
        } as AudioFileTimelineItem);

        currentOffset += size;
      }
    }

    logger.waveform.info(
      `Generated ${items.length} timeline items for operation "${selectedOpName}" (total duration: ${totalDuration.toFixed(1)}s)`
    );
    return items;
  }
);

/**
 * Derived store for the total duration of the selected operation's audio
 */
export const operationDuration: Readable<number> = derived(
  operationWaveforms,
  $operationWaveforms => {
    let totalDuration = 0;
    for (const waveform of $operationWaveforms.waveforms.values()) {
      totalDuration += waveform.duration;
    }
    return totalDuration || 30; // Default to 30 seconds if no waveforms
  }
);

/**
 * Helper to check if waveforms are still loading
 */
export const operationWaveformsLoading: Readable<boolean> = derived(
  operationWaveforms,
  $operationWaveforms => $operationWaveforms.loading
);

// ============================================================================
// SUBSCRIPTION TO SELECTED OPERATION CHANGES
// ============================================================================

let lastSelectedOperationName: string | null = null;
let lastOperationRevision: string | null = null;
let unsubscribe: (() => void) | null = null;

/**
 * Create a revision key for an operation based on its content
 */
function createOperationRevisionKey(
  operationName: string,
  operation: OperationDef | undefined,
  globalRev: number
): string {
  if (!operation) return `${operationName}:empty:${globalRev}`;

  // Create a hash based on the operation's sections and key properties
  const sectionsHash = JSON.stringify(
    operation.sections?.map(s => ({
      folderPath: s.folderPath,
      files: s.files.map(f => ({ id: f.id, path: f.path, active: f.active })),
    })) || []
  );

  return `${operationName}:${sectionsHash}:${globalRev}`;
}

/**
 * Initialize the waveform service to react to operation changes
 */
export function initWaveformService(): () => void {
  logger.waveform.info('Initializing waveform service');

  unsubscribe = appState.subscribe($appState => {
    const selectedOpName = $appState.uiSettings?.selectedOperationName || null;
    const globalRev = $appState._rev || 0;
    const operation =
      selectedOpName && $appState.operations?.defs
        ? $appState.operations.defs[selectedOpName]
        : undefined;

    // Create revision key that includes both operation name and content
    const currentRevision = createOperationRevisionKey(
      selectedOpName || 'none',
      operation,
      globalRev
    );

    // React if either the operation name or its content changed
    if (currentRevision !== lastOperationRevision) {
      const operationNameChanged = selectedOpName !== lastSelectedOperationName;
      const operationContentChanged =
        !operationNameChanged && currentRevision !== lastOperationRevision;

      if (operationNameChanged) {
        logger.waveform.operation(
          `Operation changed: "${lastSelectedOperationName}" → "${selectedOpName}"`
        );
      } else if (operationContentChanged) {
        logger.waveform.operation(
          `Operation "${selectedOpName}" content changed, reloading waveforms`
        );
      }

      lastSelectedOperationName = selectedOpName;
      lastOperationRevision = currentRevision;

      if (!selectedOpName || !$appState.operations?.defs) {
        logger.waveform.operation('No operation selected, clearing waveforms');
        operationWaveforms.clear();
        return;
      }

      if (operation) {
        const filePaths = getOperationFilePaths(operation);
        if (filePaths.length > 0) {
          logger.waveform.operation(
            `Loading waveforms for operation "${selectedOpName}" (${filePaths.length} files)`
          );
          operationWaveforms.loadForOperation(selectedOpName, filePaths);
        } else {
          logger.waveform.operation(
            `No active files in operation "${selectedOpName}", clearing waveforms`
          );
          operationWaveforms.clear();
        }
      }
    }
  });

  return () => {
    logger.waveform.info('Cleaning up waveform service');
    if (unsubscribe) {
      unsubscribe();
      unsubscribe = null;
    }
    lastSelectedOperationName = null;
    lastOperationRevision = null;
  };
}

// ============================================================================
// BACKEND COMMAND WRAPPERS
// ============================================================================

/**
 * Invalidate backend cache for a file
 */
export async function invalidateWaveformBackend(filePath: string): Promise<void> {
  logger.waveform.cache(`Invalidating backend cache for ${filePath}`);
  try {
    await invoke('invalidate_waveform', { filePath });
    waveformCache.invalidate(filePath);
    logger.waveform.success(`Successfully invalidated cache for ${filePath}`);
  } catch (error) {
    logger.waveform.error(`Failed to invalidate backend cache for ${filePath}:`, error);
    throw error;
  }
}

/**
 * Clear all backend waveform cache
 */
export async function clearWaveformCacheBackend(): Promise<void> {
  logger.waveform.cache('Clearing all backend waveform cache');
  try {
    await invoke('clear_waveform_cache');
    waveformCache.clear();
    logger.waveform.success('Successfully cleared all waveform cache');
  } catch (error) {
    logger.waveform.error('Failed to clear backend waveform cache:', error);
    throw error;
  }
}

/**
 * Get backend cache statistics
 */
export async function getWaveformCacheStats(): Promise<CacheStats> {
  logger.waveform.cache('Fetching backend cache statistics');
  try {
    const stats = await invoke<CacheStats>('get_waveform_cache_stats');
    logger.waveform.cache(
      `Cache stats - hits: ${stats.hits}, misses: ${stats.misses}, evictions: ${stats.evictions}, compute time: ${stats.totalComputeTimeMs}ms`
    );
    return stats;
  } catch (error) {
    logger.waveform.error('Failed to get cache statistics:', error);
    throw error;
  }
}
