# Child Operations and Merge System

## Overview

The render operation system now supports a hierarchical structure where merge operations can accept child operations as inputs. This allows for composable audio processing pipelines where individual operations can be executed and their results fed into a merge operation.

## Architecture

### RenderOperation Trait

All operations implement the `RenderOperation` trait which defines:

- **`name()`**: Unique operation identifier
- **`required_inputs()`**: List of required input artifact names
- **`optional_inputs()`**: List of optional input artifact names
- **`parameter_schema()`**: JSON schema for operation parameters
- **`validate_parameters()`**: Parameter validation logic
- **`execute()`**: Main execution logic that processes inputs and returns artifacts
- **`category()`**: Operation category for UI grouping (Audio, Effects, Analysis, IO, Utility)

### Key Operation Types

#### SampleOpRender (Audio Loader)

Loads audio files from disk and returns `AudioArtifact` results.

**Parameters:**
```json
{
  "file_path": "path/to/audio.wav",  // Required
  "name": "My Audio"                  // Optional
}
```

**Behavior:**
- Reads audio using Symphonia decoder
- Supports multiple formats (WAV, MP3, FLAC, etc.)
- Returns `AudioArtifact` with metadata (sample rate, channels, duration)
- No required inputs (leaf operation)

#### MergeOpRender (Audio Combiner)

Concatenates multiple audio files sequentially.

**Required Inputs:**
- `inputs`: An `AudioArtifact` or `AudioList` containing audio files to merge

**Behavior:**
- Accepts multiple audio artifacts
- Concatenates them in order
- Returns single merged `AudioArtifact`

## Child Operations Flow

### Example Usage

```json
{
  "operation_name": "merge",
  "parameters": {
    "sample_rate": 44100,
    "bit_depth": 16,
    "output_format": "wav",
    "child_operations": [
      {
        "type": "sample_load",
        "parameters": {
          "file_path": "path/to/audio1.wav",
          "name": "Audio 1"
        }
      },
      {
        "type": "sample_load",
        "parameters": {
          "file_path": "path/to/audio2.wav",
          "name": "Audio 2"
        }
      }
    ]
  }
}
```

### Execution Flow

1. **Parse Child Operations**: Extract `child_operations` array from parameters
2. **Execute Each Child**: For each child operation:
   - Create appropriate operation instance (e.g., `SampleOpRender`)
   - Validate child parameters
   - Create `OperationContext` with unique work directory
   - Execute child operation to get `AudioArtifact`
3. **Collect Results**: Gather all child operation artifacts into a list
4. **Execute Parent**: Feed collected artifacts into merge operation
5. **Return Final Result**: Merge operation returns combined `AudioArtifact`

### Implementation Details

The `execute_child_operation` helper function:

```rust
fn execute_child_operation(
    op_type: &str,              // Operation type (e.g., "sample_load")
    parameters: serde_json::Value,  // Operation-specific parameters
    base_artifacts_dir: &Path,  // Base directory for artifacts
    op_map: &mut SlotMap<OpId, ()>,  // Operation ID manager
    default_sample_rate: u32,   // Default audio settings
) -> Result<AudioArtifact, Error>
```

**Key Features:**
- Type-based dispatch to correct operation implementation
- Parameter validation before execution
- Unique work directory per operation
- Error propagation with context
- Returns `AudioArtifact` for compatibility with merge inputs

## Benefits

### Composability
Operations can be combined in flexible ways without modifying core logic.

### Testability
Individual operations can be tested in isolation before composition.

### Extensibility
New operation types can be added by:
1. Implementing `RenderOperation` trait
2. Adding case to `execute_child_operation` match statement
3. Documenting parameters and behavior

### Type Safety
Rust's type system ensures artifacts flow correctly between operations.

## Future Enhancements

### Planned Features

1. **Graph-Based Execution**
   - Full dependency graph support
   - Parallel execution of independent operations
   - Automatic result caching

2. **Additional Operation Types**
   - Audio effects (reverb, delay, EQ)
   - Analysis operations (peak detection, spectrum analysis)
   - Format conversion operations
   - Normalization and mastering

3. **Advanced Merge Modes**
   - Crossfade between audio files
   - Mix multiple tracks (not just concatenate)
   - Time-based alignment

4. **Resource Management**
   - Memory usage tracking
   - Automatic cleanup of intermediate artifacts
   - Streaming for large files

## Example: Complex Pipeline

```json
{
  "operation_name": "merge",
  "parameters": {
    "sample_rate": 48000,
    "merge_mode": "crossfade",
    "crossfade_duration_ms": 500,
    "child_operations": [
      {
        "type": "sample_load",
        "parameters": { "file_path": "intro.wav" }
      },
      {
        "type": "normalize",
        "parameters": { 
          "target_db": -12.0,
          "child_operations": [{
            "type": "sample_load",
            "parameters": { "file_path": "main.wav" }
          }]
        }
      },
      {
        "type": "sample_load",
        "parameters": { "file_path": "outro.wav" }
      }
    ]
  }
}
```

This creates a pipeline: Load intro → (Load main → Normalize) → Load outro → Merge all

## Testing

Use the `test_operation_with_params` Tauri command:

```typescript
await invoke('test_operation_with_params', {
  operationName: 'merge',
  params: {
    parameters: {
      sample_rate: 44100,
      child_operations: [
        { type: 'sample_load', parameters: { file_path: 'test1.wav' } },
        { type: 'sample_load', parameters: { file_path: 'test2.wav' } }
      ]
    }
  }
});
```

## Error Handling

All operations return `Result<Artifact, OperationError>` with specific error types:

- **InvalidInput**: Parameter validation failed
- **MissingDependency**: Required input artifact not found
- **ProcessingError**: Operation execution failed
- **IoError**: File system operation failed
- **AudioError**: Audio decoding/encoding failed

Child operation errors are propagated with context to help debugging.
