# Child Operations and Merge System

## Overview

The render operation system supports a hierarchical structure where merge operations can accept child operations as inputs. The frontend's `MergeOp.sources` property is converted to `child_operations` for the backend, enabling composable audio processing pipelines.

## Architecture Flow

```
Frontend (TypeScript)          Backend (Rust)
─────────────────────          ──────────────

MergeOp {                      test_operation_with_params(params) {
  sources: [                     child_operations: [
    { type: 'file', ... },         { type: 'sample', ... },
    { type: 'group', ... }         { type: 'sample', ... }
  ]                              ]
}                              }
     │                              │
     ├─> buildChildOperationsFromSources()
     │                              │
     └─> child_operations ─────────┤
                                    │
                                    ├─> execute_child_operation()
                                    │   ├─> SampleOpRender::execute()
                                    │   └─> returns AudioArtifact
                                    │
                                    └─> MergeOpRender::execute(artifacts)
```

## Frontend: MergeOp Sources

### MergeOp Interface

```typescript
export interface MergeOp extends BaseOperation {
  kind: 'merge';
  sources: OperationSource[]; // ← This is the key property
  outputPath: string;
  format: string;
}

export type OperationSource =
  | { type: 'group'; groupRef: string }
  | { type: 'file'; fileId: string }
  | { type: 'files'; fileIds: string[] }
  | { type: 'all' }
  | { type: 'active' }
  | { type: 'section'; sectionIndex: number }
  | { type: 'operation'; operationId: OperationId }
  | { type: 'previousOperation'; operationId: OperationId };
```

### Converting Sources to Child Operations

The `buildChildOperationsFromSources()` function in `OperationParamsDebugPanel.svelte` converts the sources:

```typescript
function buildChildOperationsFromSources(operation: OperationDef) {
  if (operation.kind !== 'merge' || !operation.sources) {
    return [];
  }

  const childOps = [];

  // For each source, create a sample operation
  for (const source of operation.sources) {
    childOps.push({
      type: 'sample',
      parameters: {
        file_path: resolveFilePath(source), // Resolve from timeline/state
        name: `${source.type}_${index}`,
      },
    });
  }

  return childOps;
}
```

## Backend: Processing Child Operations

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
  "file_path": "path/to/audio.wav", // Required
  "name": "My Audio" // Optional
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

## Complete Flow Example

### 1. Frontend: User Creates Merge Operation

```typescript
const mergeOp: MergeOp = {
  id: 'op_123',
  name: 'Combine Tracks',
  kind: 'merge',
  sources: [
    { type: 'file', fileId: 'file_1' },
    { type: 'file', fileId: 'file_2' },
  ],
  outputPath: './output',
  format: 'wav',
};
```

### 2. Frontend: Test Operation Conversion

```typescript
// In handleTestWithParams()
const childOperations = buildChildOperationsFromSources(mergeOp);
// Result:
// [
//   { type: 'sample', parameters: { file_path: 'path/to/file1.wav', name: 'file_1' } },
//   { type: 'sample', parameters: { file_path: 'path/to/file2.wav', name: 'file_2' } }
// ]

const params = {
  ...operationParams,
  child_operations: childOperations,
};

await invoke('test_operation_with_params', {
  operationName: 'merge',
  params: { parameters: params },
});
```

### 3. Backend: Execute Child Operations

```rust
// In test_operation_with_params()
if let Some(child_ops) = params.parameters.get("child_operations") {
    let mut artifacts = Vec::new();

    for child_op_data in child_ops.as_array() {
        let op_type = child_op_data.get("type").unwrap();
        let op_params = child_op_data.get("parameters").unwrap();

        // Execute child operation
        let artifact = execute_child_operation(
            op_type,
            op_params,
            &base_artifacts_dir,
            &mut op_map,
            sample_rate,
        )?;

        artifacts.push(artifact);
    }

    // Feed artifacts into merge operation
    let mut inputs = HashMap::new();
    inputs.insert("inputs".to_string(), Artifact::AudioList(artifacts));

    // Execute merge
    let result = operation.execute(context_with_inputs)?;
}
```

### 4. Backend: Execute Individual Child Operations

```rust
fn execute_child_operation(
    op_type: &str,
    parameters: serde_json::Value,
    base_artifacts_dir: &Path,
    op_map: &mut SlotMap<OpId, ()>,
    default_sample_rate: u32,
) -> Result<AudioArtifact, Error> {
    match op_type {
        "sample" => {
            let operation = SampleOpRender::new(Vec::new(), AudioSpec {
                sample_rate: default_sample_rate,
                channels: 2,
            });

            // Validate and execute
            operation.validate_parameters(&parameters)?;
            let artifact = operation.execute(context)?;

            // Extract AudioArtifact
            match artifact {
                Artifact::Audio(audio) => Ok(audio),
                _ => Err(...)
            }
        }
    }
}
```

## Current Test Implementation

For testing purposes, the system uses static test audio files:

```typescript
const testAudioFiles = [
  'assets/test_audio/420688__abletunes__abletunes-tsd-808-04-e.wav',
  'assets/test_audio/420689__abletunes__abletunes-tsd-808-03-c.wav',
];
```

## TODO: Production Implementation

### File Path Resolution

The production system needs to resolve actual file paths from the operation's sources:

```typescript
function resolveFilePath(source: OperationSource, state: AppState): string {
  switch (source.type) {
    case 'file':
      // Find file in timeline items
      const item = state.timelineItems.find(i => i.id === source.fileId);
      return item?.filePath || '';

    case 'group':
      // Resolve group to files using groupRegistry
      const fileIds = groupRegistry.eval(source.groupRef, state);
    // ... resolve fileIds to paths

    case 'operation':
    // Reference another operation's output
    // This requires executing that operation first

    // ... etc
  }
}
```

### Recursive Operation Execution

For operation references, the system needs to execute dependencies first:

```typescript
async function resolveOperationSource(source: OperationSource, state: AppState): Promise<string> {
  if (source.type === 'operation') {
    // Execute the referenced operation first
    const op = state.operations.defs[source.operationId];
    const result = await executeOperation(op);
    return result.outputPath;
  }
}
```

## Benefits

### Composability

Operations can be combined in flexible ways without modifying core logic.

### Type Safety

Rust's type system ensures artifacts flow correctly between operations.

### Testability

Individual operations can be tested in isolation before composition.

### Extensibility

New operation types can be added by:

1. Implementing `RenderOperation` trait
2. Adding case to `execute_child_operation` match statement
3. Documenting parameters and behavior

## Error Handling

All operations return `Result<Artifact, OperationError>` with specific error types:

- **InvalidInput**: Parameter validation failed
- **MissingDependency**: Required input artifact not found
- **ProcessingError**: Operation execution failed
- **IoError**: File system operation failed
- **AudioError**: Audio decoding/encoding failed

Child operation errors are propagated with context to help debugging.

## Operation Type Naming Convention

### Why "sample" instead of "sample_load"?

The child operation type is `"sample"` (not `"sample_load"`) to align with the `SampleOpRender` struct name and the render operation naming conventions:

- **Backend Struct**: `SampleOpRender` - Implements `RenderOperation` trait
- **Operation Type**: `"sample"` - Used in the frontend and backend communication
- **Consistency**: All render operations follow the pattern `{Type}OpRender` with type `"{type}"`

Examples:

- `SampleOpRender` → type: `"sample"`
- `MergeOpRender` → type: `"merge"`
- `NormalizeOpRender` → type: `"normalize"`

This convention ensures:

1. Clear mapping between frontend operation types and backend implementations
2. Consistent naming across the codebase
3. Easy to extend with new operation types
4. Type-safe communication between frontend and backend
