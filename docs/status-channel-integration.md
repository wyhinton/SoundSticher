# Event Channel + Status System Integration

## Overview

The `createTypedEventChannelWithLoggingAndStatusMessages` function combines three concerns into one:

1. **Event Channel Creation** - Type-safe Tauri event handling
2. **Logging** - Development console logging with styled output
3. **Status Publishing** - Automatic status updates in the footer

This eliminates significant boilerplate when setting up operations that need to report progress.

## Before vs After

### ❌ Before (Manual Status Publishing)

```typescript
import { createTypedEventChannelWithLogging } from '../utils/channelMaker';
import { publishStatus, clearSource } from './status';

const onBuildGraphEvent = createTypedEventChannelWithLogging<BuildGraphEvent>('BuildGraph', {
  onStarted: data => {
    logger.opPlayback.info(`Build graph started: ${data.operationCount} operations`);

    // Manual status publishing
    clearSource('build-graph');
    publishStatus({
      source: 'build-graph',
      level: 'working',
      message: `Building playback graph (${data.operationCount} operations)...`,
      progress: 0,
    });

    updateState(s => ({
      /* ... */
    }));
  },
  onProgress: data => {
    const buildProgress = (data.operationIndex + 1) / data.totalOperations;
    logger.opPlayback.info(/* ... */);

    // Manual status publishing
    clearSource('build-graph');
    publishStatus({
      source: 'build-graph',
      level: 'working',
      message: `Building: ${data.operationName} (${data.operationIndex + 1}/${data.totalOperations})`,
      progress: buildProgress,
    });

    updateState(s => ({
      /* ... */
    }));
  },
  onFinished: data => {
    logger.opPlayback.success(/* ... */);

    // Manual status publishing
    clearSource('build-graph');
    publishStatus({
      source: 'build-graph',
      level: 'success',
      message: `Graph built: ${data.operationCount} ops, ${data.totalDurationSeconds.toFixed(1)}s`,
    });

    // Manual auto-clear
    setTimeout(() => clearSource('build-graph'), 2000);

    updateState(s => ({
      /* ... */
    }));
  },
});
```

**Lines of code: ~60+ lines** (with status logic)

### ✅ After (Automatic Status Publishing)

```typescript
import { createTypedEventChannelWithLoggingAndStatusMessages } from '../utils/channelMaker';

const onBuildGraphEvent = createTypedEventChannelWithLoggingAndStatusMessages<BuildGraphEvent>(
  'BuildGraph',
  {
    // Status configuration (declarative)
    source: 'build-graph',
    startedMessage: data => `Building playback graph (${data.operationCount} operations)...`,
    progressMessage: data =>
      `Building: ${data.operationName} (${data.operationIndex + 1}/${data.totalOperations})`,
    finishedMessage: data =>
      `Graph built: ${data.operationCount} ops, ${data.totalDurationSeconds.toFixed(1)}s`,
    getProgress: data => (data.operationIndex + 1) / data.totalOperations,
    autoClearSuccess: 2000,
  },
  {
    // Event handlers (just business logic)
    onStarted: data => {
      updateState(s => ({
        /* ... */
      }));
    },
    onProgress: data => {
      updateState(s => ({
        /* ... */
      }));
    },
    onFinished: data => {
      updateState(s => ({
        /* ... */
      }));
    },
  }
);
```

**Lines of code: ~30 lines** (50% reduction!)

## API Reference

### `createTypedEventChannelWithLoggingAndStatusMessages<TEvent>`

Creates a type-safe event channel with automatic logging and status publishing.

#### Parameters

1. **`channelName: string`**
   - Display name for console logs (e.g., 'BuildGraph', 'Export')

2. **`statusConfig: StatusMessageConfig`**

   ```typescript
   {
     source: string;                                    // Status source ID
     startedMessage?: string | ((data: any) => string); // Message template
     progressMessage?: string | ((data: any) => string);
     finishedMessage?: string | ((data: any) => string);
     autoClearSuccess?: boolean | number;               // Auto-clear delay (ms)
     getProgress?: (data: any) => number | undefined;   // Extract progress (0-1)
   }
   ```

3. **`handlers: TypedChannelEventHandlers<TEvent>`**
   ```typescript
   {
     onStarted?: (data: StartedData) => void;
     onProgress?: (data: ProgressData) => void;
     onFinished?: (data: FinishedData) => void;
   }
   ```

#### Returns

`Channel<TEvent>` - Configured Tauri channel instance

## Message Templates

Messages can be either static strings or functions that extract data:

```typescript
// Static string
startedMessage: 'Starting export...';

// Dynamic function (recommended)
startedMessage: data => `Exporting ${data.fileName}...`;

// Access nested properties
progressMessage: data => `Processing ${data.operation.name} (${data.current}/${data.total})`;
```

## Progress Extraction

The `getProgress` function should return a value between 0 and 1:

```typescript
// Simple calculation
getProgress: data => data.current / data.total;

// With index offset
getProgress: data => (data.operationIndex + 1) / data.totalOperations;

// Conditional
getProgress: data => (data.total > 0 ? data.current / data.total : undefined);
```

## Auto-Clear Configuration

Control how long success messages stay visible:

```typescript
// Auto-clear after 2 seconds (default)
autoClearSuccess: true; // or just omit

// Custom delay
autoClearSuccess: 5000; // 5 seconds

// Never auto-clear
autoClearSuccess: false;
```

## Benefits

### 1. **Reduced Boilerplate**

- ~50% less code for operations with status reporting
- No manual `clearSource()` or `publishStatus()` calls
- No manual `setTimeout()` for auto-clearing

### 2. **Declarative Configuration**

- Status behavior defined upfront, separate from business logic
- Easy to see what messages will be shown
- Consistent status patterns across operations

### 3. **Type Safety**

- Full TypeScript type inference
- Compile-time checks for event data access
- Autocomplete for event properties

### 4. **Maintainability**

- Status logic centralized in one place
- Easy to change messages without touching handlers
- Handlers focus purely on state updates

### 5. **Consistency**

- All operations use the same status publishing pattern
- Automatic clearing behavior
- Standardized progress reporting

## Usage Examples

### Simple Export Operation

```typescript
const exportChannel = createTypedEventChannelWithLoggingAndStatusMessages<ExportEvent>(
  'Export',
  {
    source: 'export',
    startedMessage: 'Starting export...',
    progressMessage: data => `Exporting... ${(data.progress * 100).toFixed(0)}%`,
    finishedMessage: data => `Exported to ${data.outputPath}`,
    getProgress: data => data.progress,
  },
  {
    onFinished: data => {
      console.log('Export complete:', data.outputPath);
    },
  }
);
```

### File Processing with Details

```typescript
const processChannel = createTypedEventChannelWithLoggingAndStatusMessages<ProcessEvent>(
  'FileProcessor',
  {
    source: 'file-processing',
    startedMessage: data => `Processing ${data.fileCount} files...`,
    progressMessage: data => `Processing: ${data.currentFile} (${data.index + 1}/${data.total})`,
    finishedMessage: data => `Processed ${data.total} files in ${data.durationMs}ms`,
    getProgress: data => (data.index + 1) / data.total,
    autoClearSuccess: 3000, // Clear after 3 seconds
  },
  {
    onProgress: data => {
      updateFileList(data.currentFile);
    },
    onFinished: data => {
      showNotification('Processing complete!');
    },
  }
);
```

### No Progress Tracking

```typescript
const simpleChannel = createTypedEventChannelWithLoggingAndStatusMessages<SimpleEvent>(
  'Analyzer',
  {
    source: 'analysis',
    startedMessage: 'Analyzing audio...',
    finishedMessage: 'Analysis complete',
    // No progressMessage or getProgress - just started/finished
  },
  {
    onFinished: data => {
      displayResults(data.results);
    },
  }
);
```

## Migration Guide

To convert existing event channels:

1. **Replace the import:**

   ```typescript
   // Before
   import { createTypedEventChannelWithLogging } from '../utils/channelMaker';
   import { publishStatus, clearSource } from './status';

   // After
   import { createTypedEventChannelWithLoggingAndStatusMessages } from '../utils/channelMaker';
   ```

2. **Extract status messages to config:**
   - Find all `publishStatus()` calls in handlers
   - Extract message strings/functions to statusConfig
   - Extract progress calculations to `getProgress`

3. **Remove manual status code:**
   - Delete `clearSource()` calls
   - Delete `publishStatus()` calls
   - Delete `setTimeout()` for auto-clearing

4. **Keep business logic:**
   - Keep state updates
   - Keep side effects (notifications, etc.)
   - Keep logging if needed

## Error Handling

Error handling still needs manual status publishing (errors don't fit the started/progress/finished pattern):

```typescript
try {
  await invokeWithPerf('op_playback_build_graph', {
    request,
    onEvent: channel,
  });
} catch (error) {
  const { publishStatus, clearSource } = await import('./status');
  clearSource('build-graph');
  publishStatus({
    source: 'build-graph',
    level: 'error',
    message: `Failed: ${error.message}`,
    sticky: true,
  });
  throw error;
}
```

## Best Practices

1. **Use descriptive source IDs:** `'build-graph'` not `'bg'`
2. **Keep messages concise:** Footer has limited space
3. **Include context in progress:** Show what's being processed
4. **Use functions for dynamic messages:** More flexible than templates
5. **Set appropriate auto-clear times:** 2-3s for success, longer for important info
6. **Return undefined progress when unknown:** Better than 0 or 1
