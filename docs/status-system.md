# Status System Documentation

## Overview

The application now uses a centralized status management system that decouples status logic from UI components. This makes it easy to publish status updates from anywhere in the app.

## Architecture

### 1. Core Status Store (`status.ts`)

The central status store manages all application statuses with automatic priority resolution.

**Key Features:**

- Priority-based status resolution (error > warning > working > success > info > idle)
- Multiple concurrent statuses with automatic highest-priority selection
- Sticky statuses that persist until manually cleared
- Progress tracking (0-1 scale)
- Source tracking for targeted clearing

### 2. Status Publishers (`status-publishers.ts`)

Automatic status publishers for system events. Currently implements:

- **Buffering progress**: Automatically publishes buffering status from Tauri events

### 3. Integration Examples (`status-integration-examples.ts`)

Reference implementations showing how to publish statuses for common operations:

- Export operations with progress
- Playback status
- Audio processing
- File loading
- Error handling

### 4. Status Footer (`StatusFooter.svelte`)

A "dumb" component that simply displays the active status. No logic, just presentation.

## Status Levels

| Level     | Use Case                     | Color  | Animation     |
| --------- | ---------------------------- | ------ | ------------- |
| `idle`    | Default/ready state          | Green  | None          |
| `info`    | Informational messages       | Blue   | None          |
| `working` | Operations in progress       | Orange | Pulse         |
| `success` | Successful completion        | Green  | Success pulse |
| `warning` | Non-critical issues          | Orange | None          |
| `error`   | Failures requiring attention | Red    | Error pulse   |

## Usage Guide

### Publishing a Status

```typescript
import { publishStatus } from '$lib/state/status';

publishStatus({
  source: 'export', // Identifies where status came from
  level: 'working', // Status level
  message: 'Exporting...', // User-visible message
  progress: 0.5, // Optional: 0-1 progress indicator
  sticky: false, // Optional: stays until cleared
});
```

### Clearing Statuses

```typescript
import { clearSource, clearStatus, clearTransient } from '$lib/state/status';

// Clear all statuses from a specific source
clearSource('export');

// Clear all non-sticky statuses
clearTransient();

// Clear with custom predicate
clearStatus(s => s.level === 'error');

// Clear all statuses
clearStatus();
```

### Reading Status in UI

```typescript
import { activeStatus } from '$lib/state/status';

$: status = $activeStatus;
// status = { message: string, level: StatusLevel, progress?: number, ... }
```

## Integration Pattern

### For Async Operations

```typescript
async function performExport() {
  try {
    // 1. Clear previous statuses
    clearSource('export');

    // 2. Publish initial status
    publishStatus({
      source: 'export',
      level: 'working',
      message: 'Starting export...',
      progress: 0,
    });

    // 3. Update progress
    for (let i = 0; i < steps; i++) {
      await doStep(i);
      clearSource('export');
      publishStatus({
        source: 'export',
        level: 'working',
        message: 'Exporting...',
        progress: (i + 1) / steps,
      });
    }

    // 4. Publish success
    clearSource('export');
    publishStatus({
      source: 'export',
      level: 'success',
      message: 'Export completed!',
      sticky: true,
    });
  } catch (error) {
    // 5. Handle errors
    clearSource('export');
    publishStatus({
      source: 'export',
      level: 'error',
      message: `Export failed: ${error.message}`,
      sticky: true,
    });
  }
}
```

### For State Changes

```typescript
// Watch for playback state changes
$: {
  if (isPlaying) {
    publishStatus({
      source: 'playback',
      level: 'info',
      message: 'Playing',
    });
  } else {
    clearSource('playback');
  }
}
```

## Migration Checklist

When converting existing operations to use the status system:

- [ ] Identify status messages in the operation
- [ ] Choose appropriate source identifier
- [ ] Publish `working` status at start
- [ ] Update with progress if available
- [ ] Publish `success` or `error` at completion
- [ ] Clear source when appropriate
- [ ] Remove status logic from UI components

## Current Status

### ✅ Implemented

- Core status store with priority resolution
- Status types and levels
- StatusFooter displays active status
- Buffering event auto-publisher
- Integration examples and documentation

### 🔜 To Be Migrated

Operations that should publish statuses:

- Export operations (from Export.svelte)
- Playback state changes (from state.svelte.ts)
- Audio combining/processing
- File loading operations
- Any other async operations that currently update statusMessage directly

## Benefits

1. **Decoupling**: Status logic lives with the operation, not the UI
2. **Automatic Priority**: Errors always show above info messages
3. **No Conflicts**: Multiple operations can publish statuses simultaneously
4. **Easy Testing**: Status logic can be tested independently
5. **Consistent UX**: All statuses follow the same patterns and styling
6. **Maintainability**: Adding new status sources doesn't require UI changes

## Future Enhancements

Possible additions:

- Status history/log viewer
- Multiple status bars for different categories
- Toast notifications for completed operations
- Status persistence across sessions
- Custom status templates
