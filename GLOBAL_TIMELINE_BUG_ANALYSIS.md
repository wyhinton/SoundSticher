# Global Timeline Bug Analysis and Debug Solution

## Problem
The playback state was showing a "global" timeline session that should never exist:
```json
{
  "sessions": {
    "tl_mlbhb68i_e06i5a": { ... },
    "global": { ... }  // 🚨 THIS SHOULD NOT EXIST
  }
}
```

## Root Cause Found
The issue is in **`src/lib/state/waveformCache.ts` (line 686)**:

```typescript
// Line 686 in waveformCache.ts
const response = await opPlaybackService.buildGraph({
  operations,
  sampleRate: 44100,
  channels: 2,
  loopPlayback: true,
});
```

This is calling `opPlaybackService.buildGraph()`, which is a **commented-out legacy function** in `opPlaybackService.ts`:

```typescript
// opPlaybackService.ts (line 387-389)
// /**
//  * Build a playback graph from operations (legacy - uses global timeline)
//  */
// export async function buildGraph(request: BuildGraphRequest): Promise<BuildGraphResponse> {
//   return buildGraphForTimeline('global', request);  // ⚠️ USES 'global' AS TIMELINE ID
// }
```

**Problem Chain:**
1. Old `buildGraph()` function is commented out but still exported (or was previously exported)
2. `waveformCache.ts` still tries to call it
3. When called without a timelineId parameter, it somehow defaults to using 'global'
4. A playback session gets created with 'global' as the timeline ID

## Solution: Debug Information Added

### New Debug Panel Features
Added to **OpPlaybackDebug.svelte**:

1. **Session History Tracking**
   - Keeps last 10 state updates
   - Shows timestamps and session IDs for each update
   - Highlights when "global" timeline appears

2. **Console Warnings**
   - Auto-logs warnings when "global" timeline is detected
   - Shows full session details and complete session list
   - Displays ISO timestamp for debugging

3. **Visual Indicators**
   - Red highlight for history entries containing "global"
   - 🚨 Emoji prefix for "global" timeline in history list
   - Separate history section at bottom of debug panel

### Sample Console Output
```
⚠️ WARNING: "global" timeline session detected in playback state!
{
  timestamp: "2026-02-06T12:34:56.789Z",
  globalSession: { ... },
  allSessions: ["tl_xxx", "global"]
}
```

## Next Step: Fix Required
**File to fix:** `src/lib/state/waveformCache.ts` (line 686)

**Action:** Replace the call to `opPlaybackService.buildGraph()` with either:
1. Remove the graph building entirely if it's not needed
2. Use `buildGraphForTimeline()` with a proper timeline ID instead
3. Comment out the offending code if it's legacy code

**Current Code:**
```typescript
const response = await opPlaybackService.buildGraph({
  operations,
  sampleRate: 44100,
  channels: 2,
  loopPlayback: true,
});
```

**Should be replaced with something like:**
```typescript
// Option 1: Remove if not needed
// (just delete this call)

// Option 2: Use proper timeline ID
const timelineId = get(getActiveTimelineId());
if (timelineId) {
  const response = await buildGraphForTimeline(timelineId, {
    operations,
    sampleRate: 44100,
    channels: 2,
    loopPlayback: true,
  });
}
```

## Debug Guide
1. Open the **OpPlaybackDebug** panel in the dev UI
2. Scroll to the **"Session History (Last 10 Updates)"** section
3. Look for red-highlighted entries with 🚨 emoji
4. Check browser console for detailed warning messages
5. The history shows exactly when and how the "global" timeline appears

## References
- `OpPlaybackDebug.svelte` - Added session history tracking
- `waveformCache.ts:686` - Source of the problem (calling undefined `buildGraph()`)
- `opPlaybackService.ts:387-389` - Commented-out legacy function using 'global'
- `opPlaybackService.ts:949-975` - Exported service object (check if buildGraph is actually exported)
