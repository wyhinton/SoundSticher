# Timeline View State Separation

## Overview

Refactored timeline view state (zoom, scroll, selection) into a separate persisted store, independent from the Timeline object itself. This provides better separation of concerns and allows view state to persist independently from timeline data.

## Changes Made

### 1. Updated Timeline Interface

**Before:**

```typescript
export interface Timeline {
  id: TimelineId;
  source: TimelineSource;
  view: TimelineViewState;  // ❌ View state mixed with data
  items: Readable<TimelineItem[]>;
  waveformState?: {...};
}
```

**After:**

```typescript
export interface Timeline {
  id: TimelineId;
  source: TimelineSource;
  // ✅ No view property - kept separate
  items: Readable<TimelineItem[]>;
  waveformState?: {...};
}
```

### 2. Created Separate View State Store

Added a new persisted store for timeline view states:

```typescript
/**
 * Separate persisted store for timeline view states (zoom, scroll, selection, etc.)
 * This is kept separate from Timeline objects to allow view state to persist independently
 */
export const timelineViewStates = persisted<Record<TimelineId, TimelineViewState>>(
  'timeline-views:v1',
  {}
);
```

### 3. Added Helper Functions

```typescript
/**
 * Get the view state for a specific timeline, or return default if not found
 */
export function getTimelineViewState(timelineId: TimelineId): TimelineViewState {
  const viewStates = get(timelineViewStates);
  return viewStates[timelineId] || defaultTimelineViewState();
}

/**
 * Update the view state for a specific timeline
 */
export function updateTimelineViewState(
  timelineId: TimelineId,
  update: Partial<TimelineViewState>
): void {
  timelineViewStates.update(states => ({
    ...states,
    [timelineId]: {
      ...getTimelineViewState(timelineId),
      ...update,
    },
  }));
}
```

### 4. Updated Timeline Creation

When creating a timeline, the view state is now initialized in the separate store:

```typescript
export function createTimelineStateForOp(operationId: OperationId): Timeline {
  const timelineId = createTimelineId();
  // ... create timeline ...

  // Initialize view state for this timeline
  timelineViewStates.update(states => ({
    ...states,
    [timelineId]: defaultTimelineViewState(),
  }));

  return {
    id: timelineId,
    source: { kind: 'operation', operationId },
    items: createOperationTimelineItems(operationIdReadable, waveformState),
    waveformState,
  };
}
```

### 5. Updated Timeline Removal

When removing a timeline, the view state is also cleaned up:

```typescript
// Timeline exists - remove it (toggle off)
timelinesStore.update(state => {
  const newTimelines = { ...state.timelines };
  delete newTimelines[existingTimelineId];
  return { ...state, timelines: newTimelines };
});

// Also clean up the view state for this timeline
timelineViewStates.update(states => {
  const newStates = { ...states };
  delete newStates[existingTimelineId];
  return newStates;
});
```

### 6. Updated Serialization

Removed `view` from serialization interfaces:

```typescript
export interface SerializableTimeline {
  id: TimelineId;
  source: TimelineSource;
  // ✅ No view property
  items: TimelineItem[];
}
```

## Benefits

1. **Separation of Concerns**: Timeline data (source, items) is separate from UI state (zoom, scroll)
2. **Independent Persistence**: View states can be persisted/restored independently
3. **Cleaner Data Model**: Timeline objects are now purely about data, not presentation
4. **Easier Testing**: Can test timeline logic without worrying about view state
5. **Future Flexibility**: Can easily add multiple views for the same timeline

## Migration Notes

Components that previously accessed `timeline.view` should now:

```typescript
// Before:
const zoom = timeline.view.zoom;

// After:
import { getTimelineViewState } from '$lib/state/timeline/timelines';
const viewState = getTimelineViewState(timeline.id);
const zoom = viewState.zoom;

// Or use the store directly:
import { timelineViewStates } from '$lib/state/timeline/timelines';
$: viewState = $timelineViewStates[timeline.id] || defaultTimelineViewState();
```

## Store Structure

```
┌─────────────────────────────────────────────────────┐
│ timelinesStore (persisted)                           │
│ - timelines: {[id]: Timeline}                        │
│ - activeTimelineId                                   │
└─────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────┐
│ timelineViewStates (persisted)                       │
│ {[timelineId]: TimelineViewState}                    │
│   - zoom, scrollX, selection, visibleTracks          │
└─────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────┐
│ timelinePlaybackState (runtime only)                 │
│ {[timelineId]: {playheadTime}}                       │
└─────────────────────────────────────────────────────┘
```

## Related Files

- `src/lib/state/timeline/timelines.ts`: Main timeline state management
- Components using timelines will need to import `timelineViewStates` or `getTimelineViewState()`
