import { derived, get, writable } from 'svelte/store';
import { appState } from './state.svelte';
import type { OperationId } from './operation';

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
}

export interface TimelineLayout {
  docked: TimelineId[];
  floating: TimelineId[];
}

export interface TimelinesState {
  timelines: Record<TimelineId, Timeline>;
  activeTimelineId: TimelineId | null;
  layout: TimelineLayout;
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
  activeTimelineId: null,
  layout: {
    docked: [],
    floating: [],
  },
});

export const timelinesById = derived(timelinesStore, state => state.timelines);
export const activeTimelineId = derived(timelinesStore, state => state.activeTimelineId);
export const activeTimeline = derived(timelinesStore, state => {
  const id = state.activeTimelineId;
  return id ? state.timelines[id] ?? null : null;
});

export function findTimelineBySource(source: TimelineSource): Timeline | null {
  const timelines = Object.values(get(timelinesStore).timelines);
  for (const timeline of timelines) {
    if (timeline.source.kind !== source.kind) continue;
    if (source.kind === 'operation' && timeline.source.operationId === source.operationId) {
      return timeline;
    }
    if (source.kind === 'audioFile' && timeline.source.fileId === source.fileId) {
      return timeline;
    }
    if (
      source.kind === 'comparison' &&
      timeline.source.a === source.a &&
      timeline.source.b === source.b
    ) {
      return timeline;
    }
    if (source.kind === 'custom' && timeline.source.tracks === source.tracks) {
      return timeline;
    }
  }
  return null;
}

export function setActiveTimelineId(id: TimelineId | null): void {
  timelinesStore.update(state => {
    if (id && !state.timelines[id]) {
      return state;
    }
    return {
      ...state,
      activeTimelineId: id,
    };
  });

  if (!id) return;
  const timeline = get(timelinesStore).timelines[id];
  if (!timeline || timeline.source.kind !== 'operation') return;

  const selectedOperationId = get(appState).uiSettings?.selectedOperationId ?? null;
  if (selectedOperationId === timeline.source.operationId) return;

  appState.update(state => {
    if (!state.uiSettings) {
      state.uiSettings = {};
    }
    state.uiSettings.selectedOperationId = timeline.source.operationId;
    return state;
  });
}

export function createTimeline(params: {
  source: TimelineSource;
  view?: TimelineViewState;
  makeActive?: boolean;
  docked?: boolean;
}): TimelineId {
  const id = createTimelineId();
  const timeline: Timeline = {
    id,
    source: params.source,
    view: params.view ?? defaultTimelineViewState(),
  };

  timelinesStore.update(state => {
    const nextTimelines = { ...state.timelines, [id]: timeline };
    const nextLayout: TimelineLayout = {
      docked: [...state.layout.docked],
      floating: [...state.layout.floating],
    };

    if (params.docked === false) {
      nextLayout.floating.push(id);
    } else {
      nextLayout.docked.push(id);
    }

    return {
      ...state,
      timelines: nextTimelines,
      layout: nextLayout,
    };
  });

  if (params.makeActive !== false) {
    setActiveTimelineId(id);
  }

  return id;
}

export function ensureOperationTimeline(
  operationId: OperationId,
  makeActive: boolean = true
): TimelineId {
  const existing = findTimelineBySource({ kind: 'operation', operationId });
  if (existing) {
    if (makeActive) {
      setActiveTimelineId(existing.id);
    }
    return existing.id;
  }

  return createTimeline({
    source: { kind: 'operation', operationId },
    makeActive,
  });
}

let lastSelectedOperationId: OperationId | null = null;

appState.subscribe($appState => {
  const selectedOperationId = $appState.uiSettings?.selectedOperationId ?? null;
  if (selectedOperationId === lastSelectedOperationId) return;

  lastSelectedOperationId = selectedOperationId;
  if (!selectedOperationId) return;

  ensureOperationTimeline(selectedOperationId, true);
});
