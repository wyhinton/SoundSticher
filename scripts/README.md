# Logging Configuration Sync

This directory contains scripts to automatically synchronize configuration between the Rust backend and TypeScript frontend.

## Overview

The system has two main configuration interfaces that must be kept in sync:

- **Logging**: `LoggingConfig` struct ↔ `LoggingState` interface
- **Performance**: Tauri commands ↔ `PerformanceState` interface

## Sync Scripts

### `sync-logging-config.js`

This Node.js script automatically:

1. **Parses** the Rust `LoggingConfig` struct to extract field names
2. **Converts** Rust `snake_case` fields to TypeScript `camelCase` + "Log" suffix
3. **Updates** the TypeScript `LoggingState` interface
4. **Generates** the default state values
5. **Updates** the `updateBackendLoggingConfig` function
6. **Syncs** the `BackendLogMessage` system union type from `LogSystem` enum

### `sync-performance-types.js`

This Node.js script automatically:

1. **Scans** all Rust files for `#[tauri::command]` functions
2. **Extracts** command function names from the Tauri backend
3. **Updates** the TypeScript `PerformanceState` interface
4. **Synchronizes** the `performanceStore` default values
5. **Categorizes** commands by functionality for better organization

## When to Run

**Automatically**: These scripts are run during development via:

```bash
npm run dev          # Includes sync:logging
npm run build        # Includes sync:logging
npm run sync:logging # Manual logging sync
npm run sync:performance # Manual performance sync
```

**Manually**: Run when you:

- Add new `#[tauri::command]` functions (performance sync)
- Add new logging fields to `LoggingConfig` (logging sync)
- Want to audit command coverage
- See TypeScript errors about missing performance metrics

### Field Mapping

| Rust Field         | TypeScript Field |
| ------------------ | ---------------- |
| `encoder_enabled`  | `encoderLog`     |
| `combine_enabled`  | `combineLog`     |
| `playback_enabled` | `playbackLog`    |
| `sorting_enabled`  | `sortingLog`     |

### Usage

```bash
# Manual sync
npm run sync:logging

# Automatic sync (runs before dev/build)
npm run dev      # Auto-syncs then starts dev server
npm run build    # Auto-syncs then builds
```

#### Performance Sync Usage

1. **Add Tauri commands** to your Rust files:

   ```rust
   #[tauri::command]
   pub async fn new_audio_operation() -> Result<String, String> {
       // Implementation
       Ok("Success".to_string())
   }
   ```

2. **Run sync script**:
   ```bash
   npm run sync:performance
   ```

The TypeScript `PerformanceState` interface will be automatically updated with:

- `new_audio_operation: PerformanceMetric[]` field
- Default empty array value
- Performance tracking integration

### Adding New Backend Systems

1. **Add to Rust enum**:

   ```rust
   pub enum LogSystem {
       Encoder,
       Combine,
       NewSystem,  // Add here
       // ...
   }
   ```

2. **Add to Rust config**:

   ```rust
   pub struct LoggingConfig {
       pub encoder_enabled: bool,
       pub combine_enabled: bool,
       pub new_system_enabled: bool,  // Add here
       // ...
   }
   ```

3. **Run sync script**:
   ```bash
   npm run sync:logging
   ```

The TypeScript interface will be automatically updated with:

- `newSystemLog: boolean` field
- Default value: `false`
- Backend config mapping
- System colors and styling

### Files Modified

- `src/lib/state/logging.ts` - Frontend logging interface
- Package.json scripts - Automatic sync hooks

### Frontend-Only Fields

Some logging fields are frontend-only and won't be synced:

- `groupsLog` - UI groups logging
- `selectionLog` - Selection system logging
- `dragdropLog` - Drag & drop logging

These must be maintained manually in the TypeScript interface.

## Architecture

```
┌─────────────────────┐    ┌──────────────────────┐
│  Rust Backend       │    │  TypeScript Frontend │
│                     │    │                      │
│  LoggingConfig      │◄──┤  sync-logging-config │
│  - encoder_enabled  │   │  - Parses Rust       │
│  - combine_enabled  │   │  - Generates TS      │
│  - playback_enabled │   │  - Updates Interface │
│                     │   │                      │
│  LogSystem enum     │◄──┤  LoggingState        │
│  - Encoder          │   │  - encoderLog        │
│  - Combine          │   │  - combineLog        │
│  - Playback         │   │  - playbackLog       │
└─────────────────────┘    └──────────────────────┘
```

## Benefits

- ✅ **Type Safety**: Ensures frontend/backend logging config never gets out of sync
- ✅ **Automatic**: Runs before every build to catch drift early
- ✅ **Maintainable**: Single source of truth in Rust, TypeScript follows
- ✅ **Extensible**: Adding new systems requires only Rust changes
- ✅ **Consistent**: Field naming and mapping rules are automated
