# Playback Graph Documentation

## Overview

The operation playback system in this application uses a graph-based architecture where audio operations are scheduled on a timeline and mixed together in real-time. This document explains what happens when a user calls `op_playback_play`.

## Key Components

### SampleOp vs SamplePlayableOp

- **`SampleOp`** (`src-tauri/src/ops/sample.rs`): A sample-based operation that implements the `PlayableOp` trait. This is the actual playable operation that gets scheduled in the graph.
- **`SamplePlayableOp`** (`src-tauri/src/playback/op_playback/ops/sample_playback.rs`): A similar but separate implementation. Currently, `SampleOp` is used in the main playback commands.

### Construction and Usage

`SampleOp` instances are constructed in `op_playback_build_graph`:

```rust
// In op_playback_commands.rs line ~202
let op = Box::new(SampleOp::new(samples.clone(), spec));
```

The samples come from either:

1. **Pre-loaded samples**: Passed directly in the `AddOpRequest.samples`
2. **File loading**: Loaded using `load_audio_samples()` which uses Symphonia to decode audio files

## Playback Flow: What Happens After `op_playback_play`

### 1. Initial Setup and Validation

When `op_playback_play` is called:

```rust
pub fn op_playback_play(
    start_seconds: Option<f64>,
    state: State<'_, Arc<OpPlaybackState>>,
    app: AppHandle,
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<(), String>
```

1. **Graph Retrieval**: Gets the current playback graph or returns an error
2. **State Preparation**: Reads the audio spec and loop settings
3. **Logging**: Logs operation count, names, total duration, and start position
4. **Stop Current**: Stops any existing playback

### 2. Position Calculation

The system determines where to start playback:

- **Explicit Start**: If `start_seconds` is provided, converts to `SampleTime`
- **Resume**: Otherwise, calculates position from current progress percentage

```rust
let start_position = if let Some(start) = start_seconds {
    SampleTime::from_seconds(start, spec.sample_rate)
} else {
    // Resume from current progress
    let progress = *state.progress.lock().unwrap();
    let duration = graph.duration();
    let calculated_samples = (duration.samples() as f64 * progress as f64) as u64;
    SampleTime::new(calculated_samples)
};
```

### 3. Thread Spawning

Playback happens in a dedicated background thread to avoid blocking:

```rust
thread::spawn(move || {
    // Audio setup and playback loop
});
```

### 4. Audio System Setup

Within the spawned thread:

1. **Output Stream**: Creates a Rodio `OutputStream` for system audio output
2. **Sink**: Creates a Rodio `Sink` for audio playback control
3. **Timeline Source**: Creates a `TimelineSource` that pulls from the graph

```rust
let source = TimelineSourceBuilder::new()
    .spec(spec)
    .looping(loop_playback)
    .start_position(start_position)
    .build(graph.clone());
```

### 5. Timeline Source Operation

The `TimelineSource` is a Rodio-compatible audio source that:

- Implements the `Source` trait for Rodio compatibility
- Pulls samples from the `PlaybackGraph` on demand
- Handles real-time mixing of multiple operations
- Uses block-based rendering (512-frame blocks) for efficiency

### 6. Real-time Rendering

During playback, the `TimelineSource`:

1. **Position Tracking**: Maintains current playback position in samples
2. **Operation Query**: Asks the graph which operations are active at the current time
3. **Sample Rendering**: For each active operation, calls `render_at()` to get samples
4. **Mixing**: Combines samples from multiple operations
5. **Buffering**: Uses internal buffering for smooth audio delivery

### 7. Progress Tracking Loop

The main thread runs a progress tracking loop that:

```rust
loop {
    // Check for stop/pause conditions
    if !state_clone.is_playing.load(Ordering::Relaxed) {
        break;
    }

    // Handle pause state
    if state_clone.is_paused.load(Ordering::Relaxed) {
        // Pause timing logic
        continue;
    }

    // Calculate current position and progress
    let seek_start = *state_clone.seek_position.lock().unwrap();
    let total_elapsed = tracking_start.elapsed();
    let current_position = seek_start + total_elapsed.as_secs_f32();

    // Update progress and emit to frontend
    let progress = calculate_progress(current_position, total_duration);
    *state_clone.progress.lock().unwrap() = progress;
    emit_logged!(app_clone, "op-timeline-progress", progress);

    thread::sleep(Duration::from_millis(16)); // ~60 FPS updates
}
```

### 8. Operation Rendering Details

Each `SampleOp` when asked to render:

```rust
fn render_at(&mut self, t: SampleTime, out: &mut [f32], _spec: &AudioSpec) -> PlaybackResult<usize> {
    let channels = self.spec.channels as usize;
    let start_sample = t.samples() as usize;
    let start_idx = start_sample * channels;

    // Handle end-of-file
    if start_idx >= self.samples.len() {
        out.fill(0.0);  // Fill with silence
        return Ok(0);
    }

    // Copy available samples
    let available = self.samples.len() - start_idx;
    let to_copy = available.min(out.len());
    out[..to_copy].copy_from_slice(&self.samples[start_idx..start_idx + to_copy]);

    // Fill remainder with silence if needed
    if to_copy < out.len() {
        out[to_copy..].fill(0.0);
    }

    Ok(to_copy)
}
```

## Key Features

### Zero-Copy Architecture

- Operations share `Arc<Vec<f32>>` for sample data
- No intermediate file generation
- No full buffer pre-allocation

### Real-time Mixing

- Operations are evaluated live during playback
- Multiple operations can overlap on the timeline
- Gain and timing adjustments are applied in real-time

### Seamless Seeking

- Start position can be set to any point in the timeline
- Progress tracking allows resuming from any position
- Loop playback wraps around automatically

### Timeline Scheduling

Operations are scheduled with:

- **Start time**: When the operation begins on the timeline
- **End time**: When the operation ends (can extend beyond operation duration)
- **Gain**: Volume level for the operation
- **Operation reference**: The actual `SampleOp` containing audio data

## Audio File Loading Pipeline

1. **File Request**: User provides file path in `AddOpRequest`
2. **Symphonia Decoding**: `load_audio_samples()` uses Symphonia library to:
   - Probe file format
   - Create appropriate decoder
   - Read and decode packets
   - Convert to f32 samples
3. **Operation Creation**: Samples are wrapped in a `SampleOp`
4. **Graph Scheduling**: Operation is scheduled on the timeline
5. **Playback**: `TimelineSource` pulls samples during playback

This architecture enables efficient, real-time audio playback with support for multiple overlapping operations and seamless seeking.
