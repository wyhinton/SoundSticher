// Waveform cache service for frontend
//
// This mirrors the Rust-side waveform cache and provides:
// - Local caching of waveforms by audio key
// - Batch requests when switching operations
// - Reactive stores for timeline items derived from operations
//
// IMPORTANT ARCHITECTURE NOTE:
// - Duration comes from the DurationCache (durationCache.ts), NOT from waveforms
// - Waveform generation only cares about: file path, width, height, normalize
// - Layout depends on duration, waveforms are purely visual
// - Waveforms can arrive out of order, fail, or be re-requested without breaking layout

import { writable, derived, get, type Writable, type Readable } from 'svelte/store';
import { appState, type TimelineItem, type AudioFileTimelineItem } from './state.svelte';
import type { OperationDef, MergeOp } from './operation';
import { logger } from './logging';
import { opPlaybackService, type AddOpRequest } from './opPlaybackService';
import { invokeWithPerf } from './performance';
import { durationCache } from './durationCache';

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
    const result = await invokeWithPerf<WaveformResponse>('get_waveform', {
      request: {
        filePath,
        width: spec.width,
        height: spec.height,
        normalize: spec.normalize,
      },
    });

    if (!result.ok) {
      throw new Error(`Failed to fetch waveform: ${result.error.message}`);
    }

    return result.value.waveform;
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
        const batchResult = await invokeWithPerf<BatchWaveformResponse>('get_waveforms_batch', {
          request: {
            filePaths: toFetch,
            width: spec.width,
            height: spec.height,
            normalize: spec.normalize,
          },
        });

        if (!batchResult.ok) {
          throw new Error(`Batch fetch failed: ${batchResult.error.message}`);
        }

        const response = batchResult.value;

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
   * Get a cached waveform if available
   */
  getCached(
    filePath: string,
    spec: WaveformSpec = { width: 1000, height: 70, normalize: false }
  ): Waveform | undefined {
    const key = createCacheKey(filePath, spec.width, spec.height);
    return this.cache.get(key);
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
  /** Durations loaded from duration cache - source of truth for layout */
  durations: Map<string, number>;
  /** Waveforms are purely visual - optional and can arrive independently */
  waveforms: Map<string, Waveform>;
  /** Total duration in seconds - computed from durations map */
  totalDuration: number;
  /** Pixels per second for timeline layout */
  pxPerSecond: number;
  loading: boolean;
  loadingWaveforms: boolean;
  error: string | null;
}

/**
 * Store that holds waveforms for the currently selected operation
 * 
 * ARCHITECTURE:
 * 1. Load durations FIRST (from durationCache)
 * 2. Compute layout (totalDuration, pxPerSecond, item widths)
 * 3. THEN request waveforms with computed widths
 * 4. Waveforms populate progressively without affecting layout
 */
function createOperationWaveformStore() {
  const { subscribe, set, update } = writable<OperationWaveforms>({
    operationName: null,
    filePaths: [],
    durations: new Map(),
    waveforms: new Map(),
    totalDuration: 0,
    pxPerSecond: 10, // Default pixels per second
    loading: false,
    loadingWaveforms: false,
    error: null,
  });

  return {
    subscribe,

    /**
     * Load durations and waveforms for an operation
     * 
     * Flow:
     * 1. Load durations from duration cache (mandatory for layout)
     * 2. Compute total duration and layout metrics
     * 3. Request waveforms with computed widths (optional, visual only)
     * 4. Build playback graph
     */
    async loadForOperation(
      operationName: string, 
      filePaths: string[],
      timelineWidth: number = 1000
    ): Promise<void> {
      logger.waveform.operation(
        `Loading operation "${operationName}" (${filePaths.length} files, ${timelineWidth}px timeline)`
      );

      update(state => ({
        ...state,
        operationName,
        filePaths,
        loading: true,
        loadingWaveforms: false,
        error: null,
      }));

      try {
        // STEP 1: Load durations FIRST (from duration cache)
        logger.waveform.operation(`Step 1: Loading durations for ${filePaths.length} files`);
        const durationsMap = await durationCache.getBatch(filePaths);
        
        // Convert to our format and compute total
        const durations = new Map<string, number>();
        let totalDuration = 0;
        
        for (const [filePath, duration] of durationsMap.entries()) {
          if (duration && duration > 0) {
            durations.set(filePath, duration);
            totalDuration += duration;
          } else {
            logger.waveform.warning(`No valid duration for ${filePath}, skipping from layout`);
          }
        }

        if (totalDuration === 0) {
          throw new Error('No valid durations found for any files');
        }

        // STEP 2: Compute layout metrics
        const pxPerSecond = timelineWidth / totalDuration;
        logger.waveform.operation(
          `Step 2: Layout computed - total: ${totalDuration.toFixed(2)}s, ${pxPerSecond.toFixed(2)}px/sec`
        );

        // Update state with durations (layout is now stable)
        update(state => ({
          ...state,
          durations,
          totalDuration,
          pxPerSecond,
          loading: false,
          loadingWaveforms: true,
        }));

        // STEP 3: Request waveforms with computed widths
        logger.waveform.operation(`Step 3: Requesting waveforms with computed widths`);
        
        // Request waveforms for each file with its computed width
        const waveformPromises = filePaths.map(async (filePath) => {
          const duration = durations.get(filePath);
          if (!duration) return null;
          
          const widthPx = Math.max(1, Math.floor(duration * pxPerSecond));
          
          try {
            const waveform = await waveformCache.getOrFetch(filePath, {
              width: widthPx,
              height: 70,
              normalize: false,
            });
            
            // Update waveforms progressively
            update(state => {
              const newWaveforms = new Map(state.waveforms);
              newWaveforms.set(filePath, waveform);
              return { ...state, waveforms: newWaveforms };
            });
            
            return { filePath, waveform };
          } catch (error) {
            logger.waveform.error(`Failed to load waveform for ${filePath}:`, error);
            return null;
          }
        });

        await Promise.allSettled(waveformPromises);

        update(state => ({
          ...state,
          loadingWaveforms: false,
        }));

        logger.waveform.operation(
          `Operation "${operationName}" loaded: ${durations.size} durations, waveforms loading complete`
        );

        // STEP 4: Build playback graph (uses durations, not waveforms)
        await buildPlaybackGraphFromMergeOp();
        
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : String(error);
        update(state => ({
          ...state,
          loading: false,
          loadingWaveforms: false,
          error: errorMessage,
        }));

        logger.waveform.error(
          `Failed to load operation "${operationName}": ${errorMessage}`
        );
      }
    },

    /**
     * Clear the current operation waveforms
     */
    clear(): void {
      logger.waveform.operation('Clearing current operation');
      set({
        operationName: null,
        filePaths: [],
        durations: new Map(),
        waveforms: new Map(),
        totalDuration: 0,
        pxPerSecond: 10,
        loading: false,
        loadingWaveforms: false,
        error: null,
      });

      // Clear the playback graph as well
      opPlaybackService.clearGraph().catch(err => {
        logger.waveform.error('Failed to clear playback graph:', err);
      });
    },
  };
}

/**
 * Build a playback graph from the currently selected MergeOp
 * 
 * IMPORTANT: Uses duration cache for timing, NOT waveforms
 * Waveforms are purely visual and should not affect playback timing
 */
async function buildPlaybackGraphFromMergeOp(): Promise<void> {
  const appStateValue = get(appState);
  const selectedOpName = appStateValue.uiSettings?.selectedOperationName;

  if (!selectedOpName) {
    logger.waveform.operation('No operation selected, cannot build playback graph');
    return;
  }

  const operation = appStateValue.operations?.defs?.[selectedOpName];
  if (!operation || operation.kind !== 'merge') {
    logger.waveform.operation(
      `Operation "${selectedOpName}" is not a MergeOp, cannot build playback graph`
    );
    return;
  }

  logger.waveform.operation(
    `Building playback graph for MergeOp "${selectedOpName}" (${operation.sources.length} sources)`
  );

  const operations: AddOpRequest[] = [];
  const operationDefs = appStateValue.operations?.defs;

  if (!operationDefs) {
    logger.waveform.warning('No operation definitions available');
    return;
  }

  // Get duration store state for cached durations
  const opWaveformsState = get(operationWaveforms);
  
  let currentTime = 0;

  // Iterate through the MergeOp's sources
  for (let i = 0; i < operation.sources.length; i++) {
    const source = operation.sources[i];
    if (!source) continue;

    if (source.type === 'operation') {
      // Get the referenced SampleOp
      const sampleOp = operationDefs[source.operationRef];
      if (sampleOp && sampleOp.kind === 'sample') {
        // Get the file path from the SampleOp's sources
        const fileSource = sampleOp.sources.find(s => s.type === 'file');
        if (fileSource && fileSource.type === 'file') {
          // ✅ Get duration from duration cache, NOT waveform
          const duration = opWaveformsState.durations.get(fileSource.fileId);
          
          if (!duration) {
            logger.waveform.warning(
              `No duration cached for ${fileSource.fileId}, skipping from playback graph`
            );
            continue;
          }

          operations.push({
            name: source.operationRef,
            filePath: fileSource.fileId,
            startTime: currentTime,
            endTime: currentTime + duration,
            gain: 1.0,
          });

          currentTime += duration;

          // Add gap if specified in MergeOp
          if (operation.gapSeconds > 0) {
            currentTime += operation.gapSeconds;
          }
        }
      }
    } else {
      logger.waveform.warning(`Unsupported source type "${source.type}" in MergeOp, skipping`);
    }
  }

  if (operations.length === 0) {
    logger.waveform.warning('No operations to build playback graph');
    return;
  }

  try {
    const response = await opPlaybackService.buildGraph({
      operations,
      sampleRate: 44100,
      channels: 2,
      loopPlayback: true,
    });

    logger.waveform.operation(
      `Playback graph built: ${response.operationCount} ops, ${response.totalDurationSeconds.toFixed(2)}s duration`
    );
  } catch (error) {
    logger.waveform.error('Failed to build playback graph:', error);
  }
}

export const operationWaveforms = createOperationWaveformStore();

// ============================================================================
// DERIVED TIMELINE ITEMS STORE
// ============================================================================

/**
 * Get file paths from an operation's sources
 */
function getOperationFilePaths(operation: OperationDef | undefined): string[] {
  if (!operation) return [];

  const fileIds: string[] = [];

  // For MergeOp, we need to get file paths from referenced SampleOps
  if (operation.kind === 'merge') {
    // Need to access the operations state to resolve operation references
    const appStateValue = get(appState);
    const operations = appStateValue.operations?.defs;

    if (!operations) return [];

    // For each source in the MergeOp (which should be operation references)
    for (const source of operation.sources) {
      if (source.type === 'operation') {
        // Get the referenced SampleOp
        const sampleOp = operations[source.operationRef];
        if (sampleOp && sampleOp.kind === 'sample') {
          // Extract file IDs from the SampleOp's sources (should have one 'file' type source)
          for (const sampleSource of sampleOp.sources) {
            if (sampleSource.type === 'file') {
              fileIds.push(sampleSource.fileId);
            }
          }
        }
      }
    }
  }
  // For SampleOp, directly get file from sources
  else if (operation.kind === 'sample') {
    for (const source of operation.sources) {
      if (source.type === 'file') {
        fileIds.push(source.fileId);
      }
    }
  }

  return fileIds;
}

/**
 * Get file items from an operation's sources (with metadata)
 */
function getOperationFileItems(operation: OperationDef | undefined): Array<{
  id: string;
  path: string;
  active: boolean;
  index: number;
}> {
  if (!operation) return [];

  const fileItems: Array<{
    id: string;
    path: string;
    active: boolean;
    index: number;
  }> = [];

  // For MergeOp, we need to get file items from referenced SampleOps
  if (operation.kind === 'merge') {
    // Need to access the operations state to resolve operation references
    const appStateValue = get(appState);
    const operations = appStateValue.operations?.defs;

    if (!operations) return [];

    let index = 0;
    // For each source in the MergeOp (which should be operation references)
    for (const source of operation.sources) {
      if (source.type === 'operation') {
        // Get the referenced SampleOp
        const sampleOp = operations[source.operationRef];
        if (sampleOp && sampleOp.kind === 'sample') {
          // Extract file items from the SampleOp's sources
          for (const sampleSource of sampleOp.sources) {
            if (sampleSource.type === 'file') {
              fileItems.push({
                id: sampleSource.fileId,
                path: sampleSource.fileId,
                active: true, // Assume active since it's in the operation
                index: index++,
              });
            }
          }
        }
      }
    }
  }
  // For SampleOp, directly get file items from sources
  else if (operation.kind === 'sample') {
    let index = 0;
    for (const source of operation.sources) {
      if (source.type === 'file') {
        fileItems.push({
          id: source.fileId,
          path: source.fileId,
          active: true,
          index: index++,
        });
      }
    }
  }

  return fileItems;
}

/**
 * Derived store that provides timeline items for the currently selected operation.
 *
 * This is the key store that the Timeline component should use instead of
 * appState.timelineItems. It reactively updates when:
 * - The selected operation changes
 * - The operation's sections change
 * - Durations are loaded (layout changes)
 * - Waveforms are loaded (visual updates only)
 *
 * ARCHITECTURE:
 * - Duration from $operationWaveforms.durations -> determines layout (size, startOffset)
 * - Waveform from $operationWaveforms.waveforms -> purely visual (svgPath)
 * - Timeline layout is stable as soon as durations are loaded
 * - Waveforms can arrive later without affecting layout
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

    // Get file items from the operation sources
    const fileItems = getOperationFileItems(operation);

    if (fileItems.length === 0) {
      logger.waveform.info(`No files found in operation "${selectedOpName}"`);
      return [];
    }

    // ✅ Use durations from duration cache (source of truth for layout)
    // NOT waveforms - waveforms are purely visual
    const { durations, totalDuration } = $operationWaveforms;

    // If durations aren't loaded yet, we can't compute layout
    if (durations.size === 0 || totalDuration === 0) {
      logger.waveform.info(
        `Operation "${selectedOpName}" waiting for durations (${durations.size} loaded)`
      );
      return [];
    }

    // Log waveform loading status (informational, doesn't affect layout)
    const loadedWaveforms = $operationWaveforms.waveforms.size;
    if (loadedWaveforms < fileItems.length) {
      logger.waveform.info(
        `Operation "${selectedOpName}" has ${loadedWaveforms}/${fileItems.length} waveforms loaded (layout is stable)`
      );
    }

    // Build timeline items with start offsets based on DURATIONS (not waveforms)
    const items: TimelineItem[] = [];
    let currentOffset = 0;

    for (const file of fileItems) {
      // ✅ Get duration from duration cache (source of truth)
      const duration = durations.get(file.path);
      if (!duration || duration <= 0) {
        logger.waveform.warning(`No valid duration for ${file.path}, skipping`);
        continue;
      }

      // ✅ Calculate size from duration (stable layout)
      const size = duration / totalDuration;

      // Waveform is optional - purely visual
      const waveform = $operationWaveforms.waveforms.get(file.path);

      items.push({
        type: 'audio-file',
        id: file.id,
        fileName: file.path,
        // svgPath may be empty/undefined until waveform loads - that's OK
        svgPath: waveform?.svgPath || '',
        startOffset: currentOffset,
        size,
        active: file.active,
        // Include duration for potential component use
        duration,
      } as AudioFileTimelineItem);

      currentOffset += size;
    }

    logger.waveform.info(
      `Generated ${items.length} timeline items for operation "${selectedOpName}" (total duration: ${totalDuration.toFixed(1)}s)`
    );
    return items;
  }
);

/**
 * Derived store for the total duration of the selected operation's audio
 * 
 * ARCHITECTURE:
 * - Uses totalDuration from operationWaveforms (computed from duration cache)
 * - NOT from waveforms - waveforms are purely visual
 * - Duration is stable as soon as duration cache is loaded
 */
export const operationDuration: Readable<number> = derived(
  operationWaveforms,
  $operationWaveforms => {
    // ✅ Use totalDuration from duration cache (source of truth)
    // This is computed when durations are loaded, before waveforms
    return $operationWaveforms.totalDuration || 30; // Default to 30 seconds if not loaded
  }
);

/**
 * Helper to check if durations are still loading (affects layout)
 * This is the critical loading state - timeline can't render until durations are loaded
 */
export const operationWaveformsLoading: Readable<boolean> = derived(
  operationWaveforms,
  $operationWaveforms => $operationWaveforms.loading
);

/**
 * Helper to check if waveforms are still loading (visual only, doesn't affect layout)
 */
export const operationVisualsLoading: Readable<boolean> = derived(
  operationWaveforms,
  $operationWaveforms => $operationWaveforms.loadingWaveforms
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

  // Create a hash based on the operation's sources and key properties
  const sourcesHash = JSON.stringify(operation.sources || []);

  return `${operationName}:${sourcesHash}:${globalRev}`;
}

/**ope
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
            `No files found in operation "${selectedOpName}", clearing waveforms`
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
    const result = await invokeWithPerf('invalidate_waveform', { filePath });
    if (!result.ok) {
      throw new Error(`Failed to invalidate: ${result.error.message}`);
    }
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
    const result = await invokeWithPerf('clear_waveform_cache');
    if (!result.ok) {
      throw new Error(`Failed to clear cache: ${result.error.message}`);
    }
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
    const result = await invokeWithPerf<CacheStats>('get_waveform_cache_stats');
    if (!result.ok) {
      throw new Error(`Failed to get cache stats: ${result.error.message}`);
    }
    const stats = result.value;
    logger.waveform.cache(
      `Cache stats - hits: ${stats.hits}, misses: ${stats.misses}, evictions: ${stats.evictions}, compute time: ${stats.totalComputeTimeMs}ms`
    );
    return stats;
  } catch (error) {
    logger.waveform.error('Failed to get cache statistics:', error);
    throw error;
  }
}
