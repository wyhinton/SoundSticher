# Audio Combination Flow Documentation

## Overview

This document describes how the audio application processes clips from UI interactions to waveform visualization. The system maintains synchronization between frontend state and backend audio processing while preserving user-defined ordering.

## High-Level Flow

```mermaid
graph TD
    A[UI Action: Add/Rearrange] --> B[Frontend State Update]
    B --> C[updateInputs() Call]
    C --> D[Backend: update_inputs()]
    D --> E[Audio File Caching]
    E --> F[Custom Order Processing]
    F --> G[Waveform Generation]
    G --> H[UI Updates via Events]
```

## Process Breakdown

### 1. User Interactions

**Adding Audio Sources**: When users add folders or files, the frontend discovers all audio files, assigns sequential indices, and creates sections with metadata. New files are marked as active by default.

**Rearranging Clips**: Drag-and-drop operations update file indices and trigger visual animations. The custom order is preserved and sent to the backend for processing.

### 2. Frontend-Backend Synchronization

The `updateInputs()` function acts as the bridge between frontend state and backend processing:

- Filters only active files for processing
- Sends file paths and folder structure to backend
- Triggers automatic combination after successful sync

### 3. Backend Audio Processing (`update_inputs()`)

**File Management**:

- Extracts valid file paths from all sections
- Removes files no longer present in frontend state
- Cleans up custom ordering to match current files

**Audio Caching**:

- Decodes new audio files into raw samples (i16 format)
- Caches decoded data in memory with unique UUIDs
- Emits progress events during loading for UI feedback

### 4. Combination Process

**Order Resolution**: The system checks for custom drag-and-drop order, falling back to default BTreeMap ordering if none exists.

**Sample Combination**: Active files are processed in the determined order:

- Calculates relative positioning and sizing for each clip
- Generates individual SVG waveform paths with proper offsets
- Combines all audio samples into a single buffer

**Progress Streaming**: Real-time events are sent to frontend containing:

- Individual waveform SVG paths
- Positioning data (start_offset, size)
- File metadata (name, ID, active status)

### 5. Waveform Generation Algorithm

The backend generates SVG paths using a pixel-based approach:

- Groups audio samples into pixel-width chunks
- Finds min/max amplitude in each chunk
- Creates vertical line segments representing waveform shape
- Applies horizontal offset based on file position in timeline

### 6. UI Updates and Rendering

**Event Handling**: Frontend listens for combination progress events and updates the timeline state incrementally as each file is processed.

**Timeline Rendering**: The Timeline component renders individual segments using helper functions that abstract away the complexity of different timeline item types (audio files vs spacers).

**Visual Feedback**: Users see real-time progress as waveforms appear, with proper positioning and visual styling based on item type.

## Key Data Flow

Frontend State (sections[])
↓ (active files only)
Backend Cache (BTreeMap<path, AudioFile>)
↓ (with custom_order)
Ordered Processing → SVG Generation
↓ (streaming events)
Timeline UI Updates (timelineItems[])

```

## Performance Characteristics

- **Lazy Loading**: Audio files are only decoded when first added
- **Incremental Updates**: Only new/changed files are processed
- **Background Processing**: Heavy audio work happens in Rust threads
- **Streaming Updates**: UI updates progressively during combination
- **Memory Caching**: Decoded samples stay in memory for fast access

## Error Handling

- Invalid audio files are filtered out during processing
- File removal is handled gracefully with cleanup
- Progress events ensure UI never gets stuck in loading states
- Combination process can be cancelled if needed

```
