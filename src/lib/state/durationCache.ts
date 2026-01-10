// Audio duration cache for frontend
// Integrates with backend duration_cache.rs Tauri commands
// Caches durations for audio files to avoid redundant backend calls

import { logger } from './logging';
import { invokeWithPerf } from './performance';

export interface DurationResponse {
  path: string;
  durationSeconds: number | null;
  cacheHit: boolean;
}

export interface BatchDurationResponse {
  items: DurationResponse[];
  totalCacheHits: number;
  totalComputed: number;
  totalErrors: number;
}

export class DurationCache {
  private cache = new Map<string, number>();
  private inFlight = new Map<string, Promise<number | null>>();
  private maxEntries: number;

  constructor(maxEntries: number = 1000) {
    this.maxEntries = maxEntries;
  }

  /**
   * Get duration from cache or fetch from backend
   */
  async getOrFetch(filePath: string): Promise<number | null> {
    if (this.cache.has(filePath)) {
      logger.waveform.cache(`Duration cache hit for ${filePath}`);
      return this.cache.get(filePath)!;
    }
    if (this.inFlight.has(filePath)) {
      logger.waveform.fetch(`Duration request already in-flight for ${filePath}, waiting...`);
      return this.inFlight.get(filePath)!;
    }
    logger.waveform.fetch(`Fetching duration for ${filePath}`);
    const promise = this.fetchDuration(filePath)
      .then(duration => {
        if (duration !== null) {
          this.cache.set(filePath, duration);
          this.evictIfNeeded();
          logger.waveform.success(`Duration cached for ${filePath}: ${duration}s`);
        }
        this.inFlight.delete(filePath);
        return duration;
      })
      .catch(error => {
        this.inFlight.delete(filePath);
        logger.waveform.error(`Failed to fetch duration for ${filePath}:`, error);
        return null;
      });
    this.inFlight.set(filePath, promise);
    return promise;
  }

  /**
   * Fetch duration from backend
   */
  private async fetchDuration(filePath: string): Promise<number | null> {
    const result = await invokeWithPerf<DurationResponse>('get_duration', {
      path: filePath,
    });
    if (!result.ok || !result.value) {
      return null;
    }
    return result.value.durationSeconds ?? null;
  }

  /**
   * Batch get durations for multiple files
   */
  async getBatch(filePaths: string[]): Promise<Map<string, number | null>> {
    const result = new Map<string, number | null>();
    const toFetch: string[] = [];
    for (const filePath of filePaths) {
      if (this.cache.has(filePath)) {
        result.set(filePath, this.cache.get(filePath)!);
      } else {
        toFetch.push(filePath);
      }
    }
    if (toFetch.length > 0) {
      try {
        const batchResult = await invokeWithPerf<BatchDurationResponse>('get_durations_batch', {
          request: {
            paths: toFetch,
          },
        });
        console.log(batchResult);

        if (batchResult.ok && batchResult.value) {
          for (const resp of batchResult.value.items) {
            if (resp.durationSeconds !== null) {
              console.log(resp.path);
              this.cache.set(resp.path, resp.durationSeconds);
              result.set(resp.path, resp.durationSeconds);
            } else {
              result.set(resp.path, null);
            }
          }
          this.evictIfNeeded();
        } else {
          //   logger.waveform.error('Batch duration fetch failed:', batchResult.error);
        }
      } catch (error) {
        logger.waveform.error('Batch duration fetch failed:', error);
      }
    }
    return result;
  }

  /**
   * Invalidate cached duration for a file
   */
  async invalidate(filePath: string): Promise<void> {
    await invokeWithPerf('invalidate_duration', { path: filePath });
    if (this.cache.delete(filePath)) {
      logger.waveform.cache(`Invalidated duration cache for ${filePath}`);
    }
  }

  /**
   * Clear all cached durations
   */
  async clear(): Promise<void> {
    await invokeWithPerf('clear_duration_cache', {});
    const oldSize = this.cache.size;
    this.cache.clear();
    this.inFlight.clear();
    if (oldSize > 0) {
      logger.waveform.cache(`Cleared ${oldSize} duration cache entries`);
    }
  }

  /**
   * Get cache size
   */
  get size(): number {
    return this.cache.size;
  }

  /**
   * Get backend cache statistics
   */
  async getStats(): Promise<{ entries: number }> {
    const result = await invokeWithPerf<{ entries: number }>('get_duration_cache_stats', {});
    if (result.ok && result.value) {
      return result.value;
    }
    return { entries: this.cache.size };
  }

  /**
   * Evict oldest entries if over max size
   */
  private evictIfNeeded(): void {
    while (this.cache.size > this.maxEntries) {
      const firstKey = this.cache.keys().next().value;
      if (firstKey) {
        this.cache.delete(firstKey);
      }
    }
  }
}

// Singleton instance
export const durationCache = new DurationCache(1000);
