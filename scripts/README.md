# Logging Configuration Sync

This directory contains scripts to automatically synchronize logging configuration between the Rust backend and TypeScript frontend.

## Overview

The logging system has two main configuration interfaces that must be kept in sync:

- **Rust**: `LoggingConfig` struct in `src-tauri/src/logging.rs`
- **TypeScript**: `LoggingState` interface in `src/lib/state/logging.ts`

## Sync Script

### `sync-logging-config.js`

This Node.js script automatically:

1. **Parses** the Rust `LoggingConfig` struct to extract field names
2. **Converts** Rust `snake_case` fields to TypeScript `camelCase` + "Log" suffix
3. **Updates** the TypeScript `LoggingState` interface
4. **Generates** the default state values
5. **Updates** the `updateBackendLoggingConfig` function
6. **Syncs** the `BackendLogMessage` system union type from `LogSystem` enum

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
