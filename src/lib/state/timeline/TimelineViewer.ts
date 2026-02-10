import { derived, get, writable, type Readable, type Writable } from 'svelte/store';
import { durationCache } from '../durationCache';
import { logger } from '../logging';
import type { OperationId, OperationDef } from '../operation';
import { appState, type TimelineItem, type AudioFileTimelineItem } from '../state.svelte';
import { waveformCache } from '../waveformCache';
import { buildHierarchyMaps, type FlattenedTimelineItem } from './timelineGraph';
import type { TimelineId, TimelineSource, TimelineWaveformState } from './timelines';
import { getHierarchicalTimelineItems } from './timelines';
import { WAVEFORM_CONFIG } from '$lib/config/timelineConfig';

/**
 * TimelineViewer provides a reactive interface for viewing operation timelines.
 * It reads serializable timeline data from appState and manages runtime waveform
 * state independently (since Maps are not serializable).
 *
 * This replaces the old Timeline interface which used Readable<TimelineItem[]> and
 * was persisted in timelinesStore.
 */
export class TimelineViewer {
  private _operationId: OperationId;
  private _timelineId: TimelineId;
  private _waveformStore: Writable<TimelineWaveformState>;

  constructor(operationId: OperationId) {
    this._operationId = operationId;
    this._timelineId = `tl_op_${operationId}`;

    // Create runtime waveform state (not persisted — Maps aren't serializable)
    this._waveformStore = writable<TimelineWaveformState>({
      timelineId: this._timelineId,
      filePaths: [],
      durations: new Map(),
      waveforms: new Map(),
      totalDuration: 0,
      pxPerSecond: 10,
      loading: false,
      loadingWaveforms: false,
      error: null,
    });
  }

  get id(): TimelineId {
    return this._timelineId;
  }

  get operationId(): OperationId {
    return this._operationId;
  }

  /**
   * Get the timeline source (derived from appState)
   */
  get source(): Readable<TimelineSource | null> {
    return derived(appState, $appState => {
      const timelineData = $appState.timelines?.timelines[this._timelineId];
      if (!timelineData) return null;
      return timelineData.source as TimelineSource;
    });
  }

  /**
   * Reactive timeline items derived from appState + waveform state.
   * This replaces the old Readable<TimelineItem[]> from Timeline.items.
   */
  get items(): Readable<TimelineItem[]> {
    return derived([appState, this._waveformStore], ([$appState, $waveformState]) => {
      if (!$appState.operations?.defs) return [];

      const operation = $appState.operations.defs[this._operationId];
      if (!operation) return [];

      // Get hierarchical file items from the operation sources
      const hierarchicalItems = getHierarchicalTimelineItems(operation, this._operationId);
      if (hierarchicalItems.length === 0) return [];

      const { durations, totalDuration, waveforms } = $waveformState;
      if (durations.size === 0 || totalDuration === 0) return [];

      // Build timeline items with start offsets based on DURATIONS
      const items: TimelineItem[] = [];
      let currentOffset = 0;
      const sampleOffsets = new Map<string, { offset: number; size: number }>();

      for (const item of hierarchicalItems) {
        if (item.kind === 'sample') {
          const duration = durations.get(item.path);
          if (!duration || duration <= 0) continue;
          const size = duration / totalDuration;
          sampleOffsets.set(item.id, { offset: currentOffset, size });
          currentOffset += size;
        }
      }

      for (const item of hierarchicalItems) {
        if (item.kind === 'sample') {
          const layout = sampleOffsets.get(item.id);
          if (!layout) continue;
          const duration = durations.get(item.path);
          const waveform = waveforms.get(item.path);

          items.push({
            kind: 'sample',
            id: item.id,
            fileName: item.path,
            svgPath: waveform?.svgPath || '',
            startOffset: layout.offset,
            size: layout.size,
            active: item.active,
            duration,
            children: [],
            parentId: item.parentId,
            depth: item.depth,
            isGroup: false,
            operationId: item.operationId,
            operationName: item.operationName,
          } as AudioFileTimelineItem);
        } else if (item.kind === 'merge') {
          let minOffset = 1;
          let maxEnd = 0;

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

          if (maxEnd > minOffset) {
            items.push({
              kind: 'merge',
              id: item.id,
              fileName: item.operationName,
              svgPath: '',
              startOffset: minOffset,
              size: maxEnd - minOffset,
              active: item.active,
              duration: totalDuration * (maxEnd - minOffset),
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

      // Sort by depth then startOffset
      items.sort((a, b) => {
        const depthA = (a as AudioFileTimelineItem).depth ?? 0;
        const depthB = (b as AudioFileTimelineItem).depth ?? 0;
        if (depthA !== depthB) return depthA - depthB;
        return a.startOffset - b.startOffset;
      });

      return items;
    });
  }

  /**
   * Timeline hierarchy derived from items
   */
  get hierarchy(): Readable<ReturnType<typeof buildHierarchyMaps> | null> {
    return derived(this.items, $items => {
      if (!$items || $items.length === 0) return null;

      const flattenedItems: FlattenedTimelineItem[] = $items.map(item => {
        const audioItem = item as AudioFileTimelineItem;
        return {
          ...audioItem,
          children: audioItem.children || [],
          parentId: audioItem.parentId,
          depth: audioItem.depth || 0,
          isGroup: audioItem.isGroup || false,
          operationName: audioItem.operationName || '',
          rootGroupId: undefined,
          fileName: audioItem.fileName,
          size: audioItem.size,
          svgPath: audioItem.svgPath,
          startOffset: audioItem.startOffset,
        };
      });

      return buildHierarchyMaps(flattenedItems);
    });
  }

  /**
   * Waveform state (runtime only, not persisted)
   */
  get waveformState(): {
    subscribe: (run: (value: TimelineWaveformState) => void) => () => void;
    load: (filePaths: string[], timelineWidth: number) => Promise<void>;
    clear: () => Promise<void>;
  } {
    const store = this._waveformStore;

    const load = async (filePaths: string[], timelineWidth: number) => {
      logger.waveform.operation(
        `Loading timeline "${this._timelineId}" (${filePaths.length} files, ${timelineWidth}px)`
      );

      store.update(state => ({
        ...state,
        filePaths,
        loading: true,
        loadingWaveforms: false,
        error: null,
      }));

      try {
        // Step 1: Load durations
        const durationsMap = await durationCache.getBatch(filePaths);
        const durations = new Map<string, number>();
        let totalDuration = 0;

        for (const [filePath, duration] of durationsMap.entries()) {
          if (duration && duration > 0) {
            durations.set(filePath, duration);
            totalDuration += duration;
          }
        }

        if (totalDuration === 0) {
          throw new Error('No valid durations found for any files');
        }

        const pxPerSecond = timelineWidth / totalDuration;

        store.update(state => ({
          ...state,
          durations,
          totalDuration,
          pxPerSecond,
          loading: false,
          loadingWaveforms: true,
        }));

        // Step 2: Request waveforms
        const waveformPromises = filePaths.map(async filePath => {
          const duration = durations.get(filePath);
          if (!duration) return null;

          const widthPx = Math.max(1, Math.floor(duration * pxPerSecond));

          try {
            const waveform = await waveformCache.getOrFetch(filePath, {
              width: widthPx,
              height: WAVEFORM_CONFIG.DEFAULT_HEIGHT,
              normalize: WAVEFORM_CONFIG.DEFAULT_NORMALIZE,
            });

            store.update(state => {
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

        store.update(state => ({ ...state, loadingWaveforms: false }));

        logger.waveform.operation(
          `Timeline "${this._timelineId}" loaded: ${durations.size} durations, waveforms complete`
        );
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : String(error);
        store.update(state => ({
          ...state,
          loading: false,
          loadingWaveforms: false,
          error: errorMessage,
        }));
        logger.waveform.error(
          `Failed to load timeline waveforms for ${this._timelineId}: ${errorMessage}`
        );
      }
    };

    const clear = async () => {
      store.set({
        timelineId: this._timelineId,
        filePaths: [],
        durations: new Map(),
        waveforms: new Map(),
        totalDuration: 0,
        pxPerSecond: 10,
        loading: false,
        loadingWaveforms: false,
        error: null,
      });
    };

    return {
      subscribe: store.subscribe,
      load,
      clear,
    };
  }

  /**
   * Get the operation definition
   */
  get operation(): Readable<OperationDef | null> {
    return derived(appState, $appState => {
      return $appState.operations?.defs[this._operationId] || null;
    });
  }

  /**
   * Check if this timeline is the currently active one
   */
  get isActive(): Readable<boolean> {
    return derived(appState, $appState => {
      return $appState.timelines?.activeTimelineId === this._timelineId;
    });
  }

  /**
   * Load waveform data for this operation's files
   */
  async loadWaveformData(timelineWidth: number = 1000): Promise<void> {
    const $appState = get(appState);
    const operation = $appState.operations?.defs[this._operationId];
    if (!operation) return;

    const hierarchicalItems = getHierarchicalTimelineItems(operation, this._operationId);
    const filePaths = hierarchicalItems
      .filter(item => item.kind === 'sample')
      .map(item => item.path);

    if (filePaths.length > 0) {
      await this.waveformState.load(filePaths, timelineWidth);
    }
  }
}
