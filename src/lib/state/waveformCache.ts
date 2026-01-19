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
import { opPlaybackService, type AddOpRequest, type MergeInputRequest } from './opPlaybackService';
import { invokeWithPerf } from './performance';
import { durationCache } from './durationCache';

import {
  type FlattenedTimelineItem,
  type TimelineHierarchy,
  buildHierarchyMaps,
  getIndicesToMoveOnDrag,
} from './timelineGraph';

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
        console.log(Array.from(durationsMap.values()));
        console.log(Array.from(durationsMap.entries()));
        // Convert to our format and compute total
        const durations = new Map<string, number>();
        let totalDuration = 0;
        // console.log(durationsMap.entries())
        for (const [filePath, duration] of durationsMap.entries()) {
          console.log(duration);
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
        const waveformPromises = filePaths.map(async filePath => {
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

        logger.waveform.error(`Failed to load operation "${operationName}": ${errorMessage}`);
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
 * Build a playbook graph from the currently selected operation (MergeOp or SampleOp)
 *
 * IMPORTANT: Uses duration cache for timing, NOT waveforms
 * Waveforms are purely visual and should not affect playbook timing
 */
async function buildPlaybackGraphFromMergeOp(): Promise<void> {
  const appStateValue = get(appState);
  const selectedOpId =
    appStateValue.uiSettings?.selectedOperationId ?? appStateValue.uiSettings?.selectedOperationId;

  if (!selectedOpId) {
    logger.waveform.operation('No operation selected, cannot build playbook graph');
    return;
  }

  const operation = appStateValue.operations?.defs?.[selectedOpId];
  if (!operation) {
    logger.waveform.operation(`Operation "${selectedOpId}" not found, cannot build playbook graph`);
    return;
  }

  logger.waveform.operation(
    `Building playbook graph for ${operation.kind}Op "${operation.name}" (id: ${selectedOpId})`
  );

  const operationDefs = appStateValue.operations?.defs;

  if (!operationDefs) {
    logger.waveform.warning('No operation definitions available');
    return;
  }

  // Get duration store state for cached durations
  const opWaveformsState = get(operationWaveforms);

  const operations: AddOpRequest[] = [];

  // Recursive function to convert operations to AddOpRequest
  function convertOperationToAddOpRequest(
    op: OperationDef,
    opId: string,
    startTime: number
  ): { operations: AddOpRequest[]; totalDuration: number } {
    const result: AddOpRequest[] = [];
    let totalDuration = 0;

    if (op.kind === 'sample') {
      // Handle sample operation
      const fileSource = op.sources.find(s => s.type === 'file');
      if (fileSource && fileSource.type === 'file') {
        const duration = opWaveformsState.durations.get(fileSource.fileId);

        if (!duration) {
          logger.waveform.warning(
            `No duration cached for ${fileSource.fileId}, skipping from playbook graph`
          );
          return { operations: result, totalDuration: 0 };
        }

        result.push({
          name: `${op.name}_sample`,
          opType: 'sample',
          filePath: fileSource.fileId,
          startTime: startTime,
          endTime: startTime + duration,
          gain: 1.0,
        });

        totalDuration = duration;
      }
    } else if (op.kind === 'merge') {
      // Handle merge operation - create separate operations for each source
      let currentOffset = startTime;
      const mergeInputs: MergeInputRequest[] = [];

      for (let i = 0; i < op.sources.length; i++) {
        const source = op.sources[i];
        if (!source || source.type !== 'operation') {
          logger.waveform.warning(`Unsupported source type "${source?.type}" in MergeOp, skipping`);
          continue;
        }

        const sourceOp = operationDefs?.[source.operationId];
        if (!sourceOp) {
          logger.waveform.warning(`Referenced operation id="${source.operationId}" not found`);
          continue;
        }

        if (sourceOp.kind === 'sample') {
          // For sample operations, add them as merge inputs
          const fileSource = sourceOp.sources.find(s => s.type === 'file');
          if (fileSource && fileSource.type === 'file') {
            const duration = opWaveformsState.durations.get(fileSource.fileId);

            if (!duration) {
              logger.waveform.warning(
                `No duration cached for ${fileSource.fileId}, skipping from merge`
              );
              continue;
            }

            mergeInputs.push({
              filePath: fileSource.fileId,
              offset: currentOffset - startTime, // Offset relative to merge start
              gain: 1.0,
            });

            currentOffset += duration;

            // Add gap if specified in MergeOp
            if (op.gapSeconds > 0) {
              currentOffset += op.gapSeconds;
            }
          }
        } else if (sourceOp.kind === 'merge') {
          // For nested merge operations, recursively convert them
          const nestedResult = convertOperationToAddOpRequest(sourceOp, sourceOp.id, currentOffset);

          result.push(...nestedResult.operations);
          currentOffset += nestedResult.totalDuration;

          // Add gap if specified in MergeOp
          if (op.gapSeconds > 0) {
            currentOffset += op.gapSeconds;
          }
        }
      }

      // If we have merge inputs, create a merge operation
      if (mergeInputs.length > 0) {
        result.push({
          name: `${op.name}_merge`,
          opType: 'merge',
          startTime: startTime,
          endTime: currentOffset,
          gain: 1.0,
          inputs: mergeInputs,
        });
      }

      totalDuration = currentOffset - startTime;
    }

    return { operations: result, totalDuration };
  }

  // Convert the selected operation
  const conversionResult = convertOperationToAddOpRequest(operation, selectedOpId, 0);
  operations.push(...conversionResult.operations);

  if (operations.length === 0) {
    logger.waveform.warning('No valid operations to build playbook graph');
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
 * Handles recursive merge operations
 */
function getOperationFilePaths(operation: OperationDef | undefined): string[] {
  if (!operation) return [];

  const fileIds: string[] = [];

  /**
   * Recursively extract file paths from an operation (handles nested merge ops)
   */
  function extractFilePathsFromOperation(op: OperationDef): string[] {
    const paths: string[] = [];

    if (op.kind === 'sample') {
      // Extract file IDs from the SampleOp's sources (should have one 'file' type source)
      for (const sampleSource of op.sources) {
        if (sampleSource.type === 'file') {
          paths.push(sampleSource.fileId);
        }
      }
    } else if (op.kind === 'merge') {
      // Need to access the operations state to resolve operation references
      const appStateValue = get(appState);
      const operations = appStateValue.operations?.defs;

      if (!operations) return paths;

      // Recursively process all sources in the nested merge op
      for (const nestedSource of op.sources) {
        if (nestedSource.type === 'operation') {
          const nestedOp = operations[nestedSource.operationId];
          if (nestedOp) {
            const nestedPaths = extractFilePathsFromOperation(nestedOp);
            paths.push(...nestedPaths);
          }
        }
      }
    }

    return paths;
  }

  // For MergeOp, we need to get file paths from referenced operations
  if (operation.kind === 'merge') {
    const paths = extractFilePathsFromOperation(operation);
    fileIds.push(...paths);
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
 * Handles recursive merge operations
 */
function getOperationFileItems(operation: OperationDef | undefined): Array<{
  id: string;
  path: string;
  active: boolean;
  index: number;
}> {
  if (!operation) return [];

  let fileItems: Array<{
    id: string;
    path: string;
    active: boolean;
    index: number;
  }> = [];

  // For MergeOp, we need to get file items from referenced operations (recursively)
  if (operation.kind === 'merge') {
    fileItems = [...fileItems, ...getTimelineItemsForMergeOp(operation)];
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

function getTimelineItemsForMergeOp(operation: OperationDef) {
  let fileItems = [];
  // Need to access the operations state to resolve operation references
  const appStateValue = get(appState);
  const operations = appStateValue.operations?.defs;

  if (!operations) return [];

  let index = 0;

  /**
   * Recursively extract file items from an operation (handles nested merge ops)
   */
  function extractFileItemsFromOperation(op: OperationDef): Array<{
    id: string;
    path: string;
    active: boolean;
    index: number;
  }> {
    const items = [];

    if (op.kind === 'sample') {
      // Extract file items from the SampleOp's sources
      for (const sampleSource of op.sources) {
        if (sampleSource.type === 'file') {
          items.push({
            id: sampleSource.fileId,
            path: sampleSource.fileId,
            active: true, // Assume active since it's in the operation
            index: index++,
          });
        }
      }
    } else if (op.kind === 'merge') {
      // Recursively process all sources in the nested merge op
      console.log('Processing nested merge op:', op);
      for (const nestedSource of op.sources) {
        if (nestedSource.type === 'operation' && operations) {
          const nestedOp = operations[nestedSource.operationId];
          if (nestedOp) {
            const nestedItems = extractFileItemsFromOperation(nestedOp);
            items.push(...nestedItems);
          }
        }
      }
    }

    return items;
  }

  // For each source in the MergeOp (which should be operation references)
  console.log(operation);
  for (const source of operation.sources) {
    if (source.type === 'operation') {
      // Get the referenced operation by ID (could be SampleOp or MergeOp)
      const sourceOp = operations[source.operationId];
      console.log(sourceOp);
      if (sourceOp) {
        const extractedItems = extractFileItemsFromOperation(sourceOp);
        fileItems.push(...extractedItems);
      }
    }
  }
  return fileItems;
}

// ============================================================================
// HIERARCHY-AWARE TIMELINE ITEM EXTRACTION
// ============================================================================

/**
 * Metadata for a timeline item with hierarchy information
 *
 * IMPORTANT: operationId is the stable identifier for the operation that
 * produced this timeline item. Use this for deletion and updates, NOT names.
 */
interface TimelineItemWithHierarchy {
  id: string; // Timeline item ID (sample file ID or merge group ID)
  path: string;
  active: boolean;
  index: number;
  kind: 'sample' | 'merge';
  operationId: string; // 🔑 Immutable operation ID (source of truth)
  operationName: string; // Display name (for UI only)
  depth: number;
  parentId: string | undefined;
  children: string[];
  isGroup: boolean;
}

/**
 * Flatten an operation graph into timeline items while preserving hierarchy
 *
 * This walks the operation graph recursively and produces a flat array
 * where each item knows its:
 * - kind ('sample' or 'merge')
 * - depth (nesting level)
 * - parentId (immediate parent MergeOp)
 * - children (for MergeOps, the IDs of direct children)
 * - isGroup (true for MergeOps)
 */
function flattenOperationToTimelineItems(
  operation: OperationDef | undefined,
  operationId: string,
  operations: Record<string, OperationDef>,
  depth: number = 0,
  parentId: string | undefined = undefined
): TimelineItemWithHierarchy[] {
  if (!operation) return [];

  const items: TimelineItemWithHierarchy[] = [];
  let globalIndex = 0;

  function processOperation(
    op: OperationDef,
    opId: string,
    currentDepth: number,
    currentParentId: string | undefined
  ): TimelineItemWithHierarchy[] {
    const result: TimelineItemWithHierarchy[] = [];

    if (op.kind === 'sample') {
      // Leaf node - extract file from sources
      for (const source of op.sources) {
        if (source.type === 'file') {
          result.push({
            id: source.fileId,
            path: source.fileId,
            active: true,
            index: globalIndex++,
            kind: 'sample',
            operationId: op.id,
            operationName: op.name,
            depth: currentDepth,
            parentId: currentParentId,
            children: [],
            isGroup: false,
          });
        }
      }
    } else if (op.kind === 'merge') {
      // MergeOp - this is a group container
      const mergeId = `merge:${op.id}`;
      const childIds: string[] = [];
      const childItems: TimelineItemWithHierarchy[] = [];

      // Process each source in the MergeOp
      for (const source of op.sources) {
        if (source.type === 'operation') {
          const childOp = operations[source.operationId];
          if (childOp) {
            // Recursively process child operations
            const childResult = processOperation(
              childOp,
              source.operationId,
              currentDepth + 1,
              mergeId
            );

            // Collect child IDs (direct children only, not grandchildren)
            for (const item of childResult) {
              if (item.depth === currentDepth + 1) {
                childIds.push(item.id);
              }
            }

            childItems.push(...childResult);
          }
        }
      }

      // Add the MergeOp itself as a group container (at the current depth)
      // Note: We insert the MergeOp BEFORE its children for proper ordering
      result.push({
        id: mergeId,
        path: op.name,
        active: true,
        index: globalIndex++,
        kind: 'merge',
        operationId: op.id,
        operationName: op.name,
        depth: currentDepth,
        parentId: currentParentId,
        children: childIds,
        isGroup: true,
      });

      // Add all child items after the MergeOp
      result.push(...childItems);
    }

    return result;
  }

  return processOperation(operation, operationId, depth, parentId);
}

/**
 * Get flattened timeline items with hierarchy for the root operation
 * This is used when we want to show nested MergeOps as distinct visual groups
 */
function getHierarchicalTimelineItems(
  operation: OperationDef | undefined,
  operationId: string
): TimelineItemWithHierarchy[] {
  if (!operation) return [];

  const appStateValue = get(appState);
  const operations = appStateValue.operations?.defs;

  if (!operations) return [];

  // For the root operation, we only show its contents, not the root itself
  // (the root is implied by the selection)
  if (operation.kind === 'merge') {
    const items: TimelineItemWithHierarchy[] = [];
    let globalIndex = 0;

    // Process each source in the root MergeOp
    for (const source of operation.sources) {
      if (source.type === 'operation') {
        const childOp = operations[source.operationId];
        if (childOp) {
          // Check if this child is itself a MergeOp
          if (childOp.kind === 'merge') {
            // This is a nested MergeOp - flatten it with depth tracking
            const nestedItems = flattenOperationToTimelineItems(
              childOp,
              childOp.id,
              operations,
              0, // Start at depth 0 for nested MergeOps (they're top-level within our view)
              undefined
            );
            // Re-index the items
            for (const item of nestedItems) {
              item.index = globalIndex++;
            }
            items.push(...nestedItems);
          } else if (childOp.kind === 'sample') {
            // Regular sample - add as leaf
            for (const sampleSource of childOp.sources) {
              if (sampleSource.type === 'file') {
                items.push({
                  id: sampleSource.fileId,
                  path: sampleSource.fileId,
                  active: true,
                  index: globalIndex++,
                  kind: 'sample',
                  operationId: childOp.id,
                  operationName: childOp.name,
                  depth: 0,
                  parentId: undefined,
                  children: [],
                  isGroup: false,
                });
              }
            }
          }
        }
      }
    }

    return items;
  } else if (operation.kind === 'sample') {
    // Single sample operation
    const items: TimelineItemWithHierarchy[] = [];
    for (const source of operation.sources) {
      if (source.type === 'file') {
        items.push({
          id: source.fileId,
          path: source.fileId,
          active: true,
          index: 0,
          kind: 'sample',
          operationId: operation.id,
          operationName: operation.name,
          depth: 0,
          parentId: undefined,
          children: [],
          isGroup: false,
        });
      }
    }
    return items;
  }

  return [];
}

/**
 * Derived store that provides timeline items for the currently selected operation.
 *
 * This is the key store that the Timeline component should use instead of
 * appState.timelineItems. It reactively updates when:
 * - The selected operation changes (uiSettings.selectedOperationId)
 * - The operation's sections change
 * - Durations are loaded (layout changes)
 * - Waveforms are loaded (visual updates only)
 *
 * ARCHITECTURE:
 * - Duration from $operationWaveforms.durations -> determines layout (size, startOffset)
 * - Waveform from $operationWaveforms.waveforms -> purely visual (svgPath)
 * - Timeline layout is stable as soon as durations are loaded
 * - Waveforms can arrive later without affecting layout
 * - MergeOps are flattened but preserve hierarchy info (kind, depth, parentId, children)
 */
export const operationTimelineItems: Readable<TimelineItem[]> = derived(
  [appState, operationWaveforms],
  ([$appState, $operationWaveforms]) => {
    const selectedOpId = $appState.uiSettings?.selectedOperationId;
    if (!selectedOpId || !$appState.operations?.defs) {
      return $appState.timelineItems || [];
    }

    const operation = $appState.operations.defs[selectedOpId];
    if (!operation) {
      logger.waveform.warning(`Operation id="${selectedOpId}" not found in definitions`);
      return [];
    }

    // Get hierarchical file items from the operation sources
    const hierarchicalItems = getHierarchicalTimelineItems(operation, selectedOpId);

    if (hierarchicalItems.length === 0) {
      logger.waveform.info(`No files found in operation "${operation.name}" (id: ${selectedOpId})`);
      return [];
    }

    // ✅ Use durations from duration cache (source of truth for layout)
    // NOT waveforms - waveforms are purely visual
    const { durations, totalDuration } = $operationWaveforms;

    // If durations aren't loaded yet, we can't compute layout
    if (durations.size === 0 || totalDuration === 0) {
      logger.waveform.info(
        `Operation "${operation.name}" waiting for durations (${durations.size} loaded)`
      );
      return [];
    }

    // Log waveform loading status (informational, doesn't affect layout)
    const sampleItems = hierarchicalItems.filter(item => item.kind === 'sample');
    const loadedWaveforms = $operationWaveforms.waveforms.size;
    if (loadedWaveforms < sampleItems.length) {
      logger.waveform.info(
        `Operation "${operation.name}" has ${loadedWaveforms}/${sampleItems.length} waveforms loaded (layout is stable)`
      );
    }

    // Build timeline items with start offsets based on DURATIONS (not waveforms)
    // We need to handle both samples (have durations) and MergeOps (span their children)
    const items: TimelineItem[] = [];
    let currentOffset = 0;

    // First pass: compute offsets and sizes for samples
    const sampleOffsets = new Map<string, { offset: number; size: number }>();

    for (const item of hierarchicalItems) {
      if (item.kind === 'sample') {
        const duration = durations.get(item.path);
        if (!duration || duration <= 0) {
          logger.waveform.warning(`No valid duration for ${item.path}, skipping`);
          continue;
        }

        const size = duration / totalDuration;
        sampleOffsets.set(item.id, { offset: currentOffset, size });
        currentOffset += size;
      }
    }

    // Second pass: build timeline items with hierarchy info
    // MergeOps span from their first child to their last child
    for (const item of hierarchicalItems) {
      if (item.kind === 'sample') {
        const layout = sampleOffsets.get(item.id);
        if (!layout) continue;

        const duration = durations.get(item.path);
        const waveform = $operationWaveforms.waveforms.get(item.path);

        items.push({
          kind: 'sample',
          id: item.id,
          fileName: item.path,
          svgPath: waveform?.svgPath || '',
          startOffset: layout.offset,
          size: layout.size,
          active: item.active,
          duration,
          // Hierarchy properties
          children: [],
          parentId: item.parentId,
          depth: item.depth,
          isGroup: false,
          operationId: item.operationId,
          operationName: item.operationName,
        } as AudioFileTimelineItem);
      } else if (item.kind === 'merge') {
        // Calculate MergeOp span from its descendant samples
        let minOffset = 1;
        let maxEnd = 0;

        // Find all descendant samples to compute span
        const findDescendantOffsets = (itemId: string): void => {
          const targetItem = hierarchicalItems.find(i => i.id === itemId);
          if (!targetItem) return;

          if (targetItem.kind === 'sample') {
            const layout = sampleOffsets.get(targetItem.id);
            if (layout) {
              minOffset = Math.min(minOffset, layout.offset);
              maxEnd = Math.max(maxEnd, layout.offset + layout.size);
            }
          } else if (targetItem.kind === 'merge') {
            for (const childId of targetItem.children) {
              findDescendantOffsets(childId);
            }
          }
        };

        for (const childId of item.children) {
          findDescendantOffsets(childId);
        }

        // Only add MergeOp if it spans some samples
        if (maxEnd > minOffset) {
          items.push({
            kind: 'merge',
            id: item.id,
            fileName: item.operationName,
            svgPath: '', // MergeOps don't have waveforms
            startOffset: minOffset,
            size: maxEnd - minOffset,
            active: item.active,
            duration: totalDuration * (maxEnd - minOffset),
            // Hierarchy properties
            children: item.children,
            parentId: item.parentId,
            depth: item.depth,
            isGroup: true,
            operationId: item.operationId,
            operationName: item.operationName,
          } as AudioFileTimelineItem);
        }
      }
    }

    // Sort by startOffset to ensure proper rendering order
    items.sort((a, b) => {
      // MergeOps should render before their children (background)
      const aItem = a as AudioFileTimelineItem;
      const bItem = b as AudioFileTimelineItem;

      // First sort by depth (lower depth = render first = background)
      const depthA = aItem.depth ?? 0;
      const depthB = bItem.depth ?? 0;
      if (depthA !== depthB) {
        return depthA - depthB;
      }

      // Then by startOffset
      return a.startOffset - b.startOffset;
    });

    logger.waveform.info(
      `Generated ${items.length} timeline items for operation "${operation.name}" (id: ${selectedOpId}) ` +
        `(${items.filter(i => (i as AudioFileTimelineItem).kind === 'merge').length} groups, ` +
        `${items.filter(i => (i as AudioFileTimelineItem).kind === 'sample').length} samples, ` +
        `total duration: ${totalDuration.toFixed(1)}s)`
    );
    console.log(items);
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

/**
 * Derived store that provides hierarchy information for drag/selection operations
 *
 * This builds the hierarchy maps from the timeline items, enabling:
 * - Group drag behavior (dragging a MergeOp moves all descendants)
 * - Finding parent/child relationships
 * - Resolving which items to move together
 */
export const operationTimelineHierarchy: Readable<TimelineHierarchy | null> = derived(
  operationTimelineItems,
  $items => {
    if ($items.length === 0) return null;

    // Convert TimelineItems to FlattenedTimelineItems for hierarchy building
    const flattenedItems: FlattenedTimelineItem[] = $items.map(item => {
      const audioItem = item as AudioFileTimelineItem;
      return {
        kind: (audioItem.kind || 'sample') as 'sample' | 'merge',
        id: audioItem.id,
        fileName: audioItem.fileName,
        svgPath: audioItem.svgPath,
        startOffset: audioItem.startOffset,
        size: audioItem.size,
        active: audioItem.active,
        duration: audioItem.duration,
        children: audioItem.children || [],
        parentId: audioItem.parentId,
        depth: audioItem.depth ?? 0,
        isGroup: audioItem.isGroup ?? false,
        operationName: audioItem.operationName || '',
        rootGroupId: undefined,
      };
    });

    return buildHierarchyMaps(flattenedItems);
  }
);

// ============================================================================
// SUBSCRIPTION TO SELECTED OPERATION CHANGES
// ============================================================================

let lastSelectedOperationId: string | null = null;
let lastOperationRevision: string | null = null;
let unsubscribe: (() => void) | null = null;

/**
 * Create a revision key for an operation based on its content
 */
function createOperationRevisionKey(
  operationId: string,
  operation: OperationDef | undefined,
  globalRev: number
): string {
  if (!operation) return `${operationId}:empty:${globalRev}`;

  // Create a hash based on the operation's sources and key properties
  const sourcesHash = JSON.stringify(operation.sources || []);

  return `${operationId}:${sourcesHash}:${globalRev}`;
}

/**ope
 * Initialize the waveform service to react to operation changes
 */
export function initWaveformService(): () => void {
  logger.waveform.info('Initializing waveform service');

  unsubscribe = appState.subscribe($appState => {
    const selectedOpId = $appState.uiSettings?.selectedOperationId || null;
    const globalRev = $appState._rev || 0;
    const operation =
      selectedOpId && $appState.operations?.defs
        ? $appState.operations.defs[selectedOpId]
        : undefined;

    // Create revision key that includes both operation ID and content
    const currentRevision = createOperationRevisionKey(
      selectedOpId || 'none',
      operation,
      globalRev
    );

    // React if either the operation ID or its content changed
    if (currentRevision !== lastOperationRevision) {
      const operationIdChanged = selectedOpId !== lastSelectedOperationId;
      const operationContentChanged =
        !operationIdChanged && currentRevision !== lastOperationRevision;

      if (operationIdChanged) {
        logger.waveform.operation(
          `Operation changed: "${lastSelectedOperationId}" → "${selectedOpId}"`
        );
      } else if (operationContentChanged) {
        logger.waveform.operation(
          `Operation "${selectedOpId}" content changed, reloading waveforms`
        );
      }

      lastSelectedOperationId = selectedOpId;
      lastOperationRevision = currentRevision;

      if (!selectedOpId || !$appState.operations?.defs) {
        logger.waveform.operation('No operation selected, clearing waveforms');
        operationWaveforms.clear();
        return;
      }

      if (operation) {
        const filePaths = getOperationFilePaths(operation);
        if (filePaths.length > 0) {
          logger.waveform.operation(
            `Loading waveforms for operation "${operation.name}" (id: ${selectedOpId}, ${filePaths.length} files)`
          );
          operationWaveforms.loadForOperation(operation.name, filePaths);
        } else {
          logger.waveform.operation(
            `No files found in operation "${operation.name}" (id: ${selectedOpId}), clearing waveforms`
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
    lastSelectedOperationId = null;
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
