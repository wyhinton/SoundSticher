# Timeline Configuration System

This document explains the centralized configuration system for timeline-related constants to ensure consistency across the frontend.

## Overview

The timeline configuration system provides a single source of truth for timeline-related constants that are used across multiple files. This prevents mismatches and makes it easy to adjust timeline behavior from one location.

**Location**: `src/lib/config/timelineConfig.ts`

## Problem Statement

Before this system, timeline constants were duplicated across files:

- `Timeline.svelte` had `baseContentHeight = 80`
- `waveformCache.ts` had waveform default height of `70`
- Resize constraints (`10%`, `60%`) were hardcoded in `Greet.svelte`
- Cache size limit (`500`) was hardcoded in `waveformCache.ts`

This led to:
- **Visual misalignment**: Waveforms at 70px height didn't match the 80px content region
- **Maintenance issues**: Changing one value required finding all related values
- **Inconsistency**: No clear relationship between related constants

## Solution

All timeline-related constants are now defined in `timelineConfig.ts`:

```typescript
export const TIMELINE_LAYOUT = {
  TOP_PADDING: 20,
  AXIS_HEIGHT: 20,
  BASE_CONTENT_HEIGHT: 80,
  DEFAULT_HEIGHT: 120,
} as const;

export const WAVEFORM_CONFIG = {
  DEFAULT_WIDTH: 1000,
  DEFAULT_HEIGHT: TIMELINE_LAYOUT.BASE_CONTENT_HEIGHT, // ✅ Now linked!
  DEFAULT_NORMALIZE: false,
  MAX_CACHE_ENTRIES: 500,
} as const;

export const TIMELINE_RESIZE = {
  MIN_HEIGHT_PERCENT: 10,
  MAX_HEIGHT_PERCENT: 60,
  DEFAULT_HEIGHT_PERCENT: 30,
} as const;

export const TIMELINE_DERIVED = {
  get CENTER_Y(): number {
    return TIMELINE_LAYOUT.BASE_CONTENT_HEIGHT / 2;
  },
  // ... other computed values
} as const;
```

## Configuration Groups

### TIMELINE_LAYOUT

SVG layout constants for the timeline component:

| Constant | Value | Description |
|----------|-------|-------------|
| `TOP_PADDING` | 20 | Header region height (px) |
| `AXIS_HEIGHT` | 20 | X-axis footer height (px) |
| `BASE_CONTENT_HEIGHT` | 80 | Design height for waveform content (px) |
| `DEFAULT_HEIGHT` | 120 | Default total timeline height (px) |

**Usage**: `Timeline.svelte` uses these for SVG layout and scaling calculations.

### WAVEFORM_CONFIG

Waveform request and caching configuration:

| Constant | Value | Description |
|----------|-------|-------------|
| `DEFAULT_WIDTH` | 1000 | Default waveform width (px) |
| `DEFAULT_HEIGHT` | 80 | **Links to TIMELINE_LAYOUT.BASE_CONTENT_HEIGHT** |
| `DEFAULT_NORMALIZE` | false | Whether to normalize amplitude |
| `MAX_CACHE_ENTRIES` | 500 | Maximum cached waveforms |

**Critical**: `DEFAULT_HEIGHT` is linked to `BASE_CONTENT_HEIGHT` to ensure visual alignment.

**Usage**: `waveformCache.ts` uses these for waveform requests and cache management.

### TIMELINE_RESIZE

User resize constraints and defaults:

| Constant | Value | Description |
|----------|-------|-------------|
| `MIN_HEIGHT_PERCENT` | 10 | Minimum timeline height (10% of viewport) |
| `MAX_HEIGHT_PERCENT` | 60 | Maximum timeline height (60% of viewport) |
| `DEFAULT_HEIGHT_PERCENT` | 30 | Default timeline height (30% of viewport) |

**Usage**: `Greet.svelte` uses these for the draggable divider constraints, `state.svelte.ts` uses for default state.

### TIMELINE_DERIVED

Computed values derived from other constants:

| Property | Formula | Description |
|----------|---------|-------------|
| `CENTER_Y` | `BASE_CONTENT_HEIGHT / 2` | Center line Y position (40px) |
| `FIXED_HEIGHT` | `TOP_PADDING + AXIS_HEIGHT` | Total fixed height (40px) |
| `MIN_CONTENT_HEIGHT` | 40 | Minimum usable content height (px) |

**Usage**: Provides computed values to avoid recalculating in components.

## Benefits

### 1. Single Source of Truth

All timeline constants in one file:
- Easy to find and understand relationships
- Change in one place affects all consumers
- No duplication or drift

### 2. Type Safety

All configs use `as const` for:
- Readonly types (prevents accidental modification)
- Type inference for config consumers
- Exported types for external validation

### 3. Linked Values

Related constants reference each other:
```typescript
DEFAULT_HEIGHT: TIMELINE_LAYOUT.BASE_CONTENT_HEIGHT
```

This ensures waveform height always matches content region height.

### 4. Documentation

Config file serves as living documentation:
- Clear purpose for each constant
- Comments explain relationships
- Easy to onboard new developers

## Usage Examples

### Timeline.svelte

```typescript
import { TIMELINE_LAYOUT, TIMELINE_DERIVED } from '$lib/config/timelineConfig';

const topPadding = TIMELINE_LAYOUT.TOP_PADDING;
const axisHeight = TIMELINE_LAYOUT.AXIS_HEIGHT;
const baseContentHeight = TIMELINE_LAYOUT.BASE_CONTENT_HEIGHT;

let height: number = TIMELINE_LAYOUT.DEFAULT_HEIGHT;

$: tempYCenter = TIMELINE_DERIVED.CENTER_Y;
```

### waveformCache.ts

```typescript
import { WAVEFORM_CONFIG } from '$lib/config/timelineConfig';

async getOrFetch(
  filePath: string,
  spec: WaveformSpec = {
    width: WAVEFORM_CONFIG.DEFAULT_WIDTH,
    height: WAVEFORM_CONFIG.DEFAULT_HEIGHT, // ✅ Now 80, matches timeline!
    normalize: WAVEFORM_CONFIG.DEFAULT_NORMALIZE,
  }
): Promise<Waveform> { ... }
```

### Greet.svelte

```typescript
import { TIMELINE_RESIZE } from './config/timelineConfig';

$: timelineHeight = $appState.uiSettings?.timelineHeight || 
                    TIMELINE_RESIZE.DEFAULT_HEIGHT_PERCENT;

const constrainedHeight = Math.max(
  TIMELINE_RESIZE.MIN_HEIGHT_PERCENT,
  Math.min(TIMELINE_RESIZE.MAX_HEIGHT_PERCENT, newHeightPercent)
);
```

### state.svelte.ts

```typescript
import { TIMELINE_RESIZE } from '$lib/config/timelineConfig';

const defaultState: AppState = {
  uiSettings: {
    timelineHeight: TIMELINE_RESIZE.DEFAULT_HEIGHT_PERCENT,
    // ...
  },
};
```

## Migration Checklist

When adding new timeline-related constants:

- [ ] Add to appropriate config group in `timelineConfig.ts`
- [ ] Add JSDoc comment explaining purpose
- [ ] Link to related constants if applicable
- [ ] Export type if needed for external validation
- [ ] Update this documentation
- [ ] Remove any hardcoded values from components
- [ ] Verify all consumers use the config

## Best Practices

### DO ✅

- Use config constants instead of magic numbers
- Link related constants (e.g., waveform height to content height)
- Add comments explaining non-obvious relationships
- Group related constants together
- Use `as const` for immutability

### DON'T ❌

- Hardcode values in components
- Duplicate constants across files
- Create constants without documentation
- Modify config values at runtime
- Use different values for the same semantic meaning

## Future Enhancements

Potential improvements to consider:

1. **Runtime Validation**: Add Zod schema for config validation
2. **User Preferences**: Allow some values to be user-configurable
3. **Theme Integration**: Link timeline colors to theme system
4. **Performance Profiling**: Add config for performance-related thresholds
5. **Accessibility**: Add config for accessible size minimums

## Related Documentation

- **Timeline Layout**: `timeline-scalable-layout.md` - SVG scaling system
- **Timeline Resize**: `timeline-resizable-layout.md` - User resize feature
- **Waveform Cache**: `waveformCache.ts` - Waveform loading and caching
- **State Management**: `state.svelte.ts` - Application state structure

## Troubleshooting

### Waveforms don't align with timeline

**Cause**: `WAVEFORM_CONFIG.DEFAULT_HEIGHT` doesn't match `TIMELINE_LAYOUT.BASE_CONTENT_HEIGHT`

**Solution**: Ensure `DEFAULT_HEIGHT` references `BASE_CONTENT_HEIGHT`:
```typescript
DEFAULT_HEIGHT: TIMELINE_LAYOUT.BASE_CONTENT_HEIGHT
```

### Timeline won't resize beyond certain limit

**Cause**: Resize constraints in `TIMELINE_RESIZE` are too restrictive

**Solution**: Adjust `MIN_HEIGHT_PERCENT` or `MAX_HEIGHT_PERCENT` as needed

### Cache fills up too quickly

**Cause**: `WAVEFORM_CONFIG.MAX_CACHE_ENTRIES` is too small

**Solution**: Increase `MAX_CACHE_ENTRIES` (current: 500)
