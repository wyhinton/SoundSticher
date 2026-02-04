import { derived, get, Readable, writable } from 'svelte/store';
import { appState, AudioFileTimelineItem, TimelineItem } from './state.svelte';
import type { OperationId } from './operation';
import { logger } from './logging';
import { getHierarchicalTimelineItems, operationWaveforms } from './waveformCache';

export type TimelineId = string;
export type TrackId = string;

export type TimelineSource =
  | { kind: 'operation'; operationId: OperationId }
  | { kind: 'audioFile'; fileId: string }
  | { kind: 'comparison'; a: OperationId; b: OperationId }
  | { kind: 'custom'; tracks: TrackSpec[] };

export type TimeRange = {
  start: number;
  end: number;
};

export interface TrackSpec {
  id: TrackId;
  name?: string;
}

export interface TimelineViewState {
  zoom: number;
  scrollX: number;
  playheadTime: number;
  selection?: TimeRange;
  visibleTracks: TrackId[];
}

export interface Timeline {
  id: TimelineId;
  source: TimelineSource;
  view: TimelineViewState;
  items: Readable<TimelineItem[]>;
}

export interface TimelineLayout {
  docked: TimelineId[];
  floating: TimelineId[];
}

export interface TimelinesState {
  timelines: Record<TimelineId, Timeline>;
}

const DEFAULT_VIEW_STATE: TimelineViewState = {
  zoom: 1,
  scrollX: 0,
  playheadTime: 0,
  visibleTracks: [],
};

function createTimelineId(): TimelineId {
  const now = Date.now().toString(36);
  const rand = Math.random().toString(36).slice(2, 8);
  return `tl_${now}_${rand}`;
}

export function defaultTimelineViewState(): TimelineViewState {
  return {
    ...DEFAULT_VIEW_STATE,
    visibleTracks: [],
  };
}

export const timelinesStore = writable<TimelinesState>({
  timelines: {},
});

export function createOperationTimelineItems(
  operationId: Readable<string | null>
): Readable<TimelineItem[]> {
  return derived(
    [operationId, appState, operationWaveforms],
    ([$operationId, $appState, $operationWaveforms]) => {
      if (!$operationId || !$appState.operations?.defs) {
        return [];
      }

      const operation = $appState.operations.defs[$operationId];
      if (!operation) {
        logger.waveform.warning(`Operation id="${$operationId}" not found in definitions`);
        return [];
      }

      // 👇 EVERYTHING BELOW CAN STAY ALMOST IDENTICAL
      // (this is important)

      // Get hierarchical file items from the operation sources
      const hierarchicalItems = getHierarchicalTimelineItems(operation, $operationId);

      if (hierarchicalItems.length === 0) {
        logger.waveform.info(
          `No files found in operation "${operation.name}" (id: ${$operationId})`
        );
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
        `Generated ${items.length} timeline items for operation "${operation.name}" (id: ${$operationId}) ` +
          `(${items.filter(i => (i as AudioFileTimelineItem).kind === 'merge').length} groups, ` +
          `${items.filter(i => (i as AudioFileTimelineItem).kind === 'sample').length} samples, ` +
          `total duration: ${totalDuration.toFixed(1)}s)`
      );
      console.log(items);
      return items;
    }
  );
}
