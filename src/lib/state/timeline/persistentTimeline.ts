import { persisted } from 'svelte-persisted-store';
import { get } from 'svelte/store';
import { TimelineItem } from '../state.svelte';
import {
  TimelineId,
  TimelineSource,
  timelinesStore,
  TimelineViewState,
  TimelinesState,
  Timeline,
} from './timelines';

export interface SerializableTimeline {
  id: TimelineId;
  source: TimelineSource;
  view: TimelineViewState;
  items: TimelineItem[]; // plain array
}

export interface SerializableTimelinesState {
  timelines: Record<TimelineId, SerializableTimeline>;
  //   layout?: TimelineLayout;
}

/**
 * Serialize a Timeline to a SerializableTimeline
 * This converts the reactive items store to a plain array
 */
function serializeTimeline(timeline: Timeline): SerializableTimeline {
  // Get the current value of the items store
  const currentItems = get(timeline.items);

  return {
    id: timeline.id,
    source: timeline.source,
    view: timeline.view,
    items: currentItems, // Convert Readable<TimelineItem[]> to TimelineItem[]
  };
}

/**
 * Serialize a TimelinesState to a SerializableTimelinesState
 * This converts all timelines to their serializable form
 */
function serializeTimelineState(state: TimelinesState): SerializableTimelinesState {
  const serializedTimelines: Record<TimelineId, SerializableTimeline> = {};

  for (const [timelineId, timeline] of Object.entries(state.timelines)) {
    if (timeline) {
      serializedTimelines[timelineId] = serializeTimeline(timeline);
    }
  }

  return {
    timelines: serializedTimelines,
  };
}

export const timelineSnapshotStore = persisted<SerializableTimelinesState>('timelines:v1', {
  timelines: {},
}, {});

let timeout: number | null = null;

export const subscribeForTimelineStoreSerialization = () => {
  timelinesStore.subscribe(state => {
    if (timeout) return;

    timeout = window.setTimeout(() => {
      timelineSnapshotStore.set(serializeTimelineState(state));
      timeout = null;
    }, 500);
  });
};
