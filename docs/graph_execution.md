# Graph-Based Execution System

## Overview

The Sound Stitch application implements a sophisticated graph-based execution system for audio operations. This system provides dependency management, invalidation cascading, and parallel execution of audio processing tasks.

## Architecture Components

### 1. Operation Graph (`graph/graph.rs`)

The `OperationGraph` is a Directed Acyclic Graph (DAG) that manages dependencies between operations.

```rust
pub struct OperationGraph {
    dependencies: HashMap<OpId, Vec<OpId>>,  // node -> list of dependencies
    dependents: HashMap<OpId, Vec<OpId>>,    // node -> list of nodes that depend on it
    nodes: HashSet<OpId>,
}
```

**Key Features:**

- **Cycle Detection**: Prevents circular dependencies that would cause infinite loops
- **Dependency Resolution**: Ensures operations execute in correct order
- **Topological Sorting**: Provides execution order that respects all dependencies

**Example Usage:**

```rust
let mut graph = OperationGraph::new();
graph.add_node(normalize_id);
graph.add_node(merge_id);
graph.add_dependency(normalize_id, merge_id)?; // normalize depends on merge
```

### 2. Invalidation Manager (`graph/invalidation.rs`)

Handles dirty propagation and invalidation cascading through the operation graph.

```rust
pub struct InvalidationManager {
    graph: OperationGraph,
    dirty_nodes: HashSet<OpId>,
    cook_queue: VecDeque<OpId>,
}
```

**Key Features:**

- **Dirty Tracking**: Marks nodes that need recomputation
- **Cascade Invalidation**: When a node changes, all dependent nodes are marked dirty
- **Cook Queue**: Maintains execution order based on dependencies

**Workflow:**

1. **Mark Dirty**: `invalidate_node(id)` marks a node and all dependents as needing recomputation
2. **Get Next**: `get_next_cook_node()` returns the next ready-to-execute operation
3. **Mark Clean**: `validate_node(id)` marks a node as completed and removes from queue

### 3. Operation Node Manager (`graph/op_node.rs`)

Manages individual operation nodes and their execution context.

```rust
pub struct OperationNode {
    pub op_id: OpId,
    pub operation_type: String,
    pub parameters: serde_json::Value,
    pub inputs: HashMap<String, OpId>,
    pub outputs: HashMap<String, OpId>,
    pub status: NodeStatus,
    // ... other fields
}
```

**Node Lifecycle:**

- **Pending**: Initial state, waiting for dependencies
- **Ready**: All dependencies satisfied, ready to execute
- **Running**: Currently executing
- **Completed**: Successfully finished
- **Failed**: Execution failed
- **Cancelled**: Execution was cancelled

## Cook Scheduler Integration

The `CookScheduler` orchestrates the entire graph execution system:

```rust
pub struct CookScheduler {
    operation_registry: Arc<OperationRegistry>,
    node_manager: Arc<Mutex<OperationNodeManager>>,
    invalidation_manager: Arc<Mutex<InvalidationManager>>,
    artifact_storage: Arc<ArtifactStorage>,
    // ... other components
}
```

### Execution Flow

1. **Task Submission**:

   ```rust
   scheduler.submit_task(task)?;
   ```

2. **Dependency Resolution**:
   - Task is converted to operation node
   - Dependencies are analyzed and graph is updated
   - Node is queued for execution

3. **Parallel Execution**:
   - Worker threads pull ready tasks from the queue
   - Dependencies ensure correct execution order
   - Results are cached in artifact storage

4. **Invalidation Handling**:
   - When inputs change, dependent operations are invalidated
   - Only affected operations are re-executed
   - Unchanged results are reused from cache

## Operation System

### Operation Registry

Central registry for all available operations:

```rust
pub struct OperationRegistry {
    operations: HashMap<String, Box<dyn Operation>>,
}
```

**Supported Operations:**

- `MergeOperation`: Combines multiple audio files with crossfading
- `NormalizeOperation`: Applies volume normalization
- `ExportOperation`: Converts to different audio formats
- _Extensible_: New operations can be easily added

### Operation Context

Each operation receives a context with everything needed for execution:

```rust
pub struct OperationContext {
    pub op_id: OpId,
    pub work_dir: PathBuf,
    pub inputs: HashMap<String, Artifact>,
    pub parameters: serde_json::Value,
    pub progress_callback: Option<Box<dyn Fn(f32) + Send + Sync>>,
}
```

## Artifact Management

### Artifact Types

```rust
pub enum Artifact {
    Audio(AudioArtifact),
    AudioList(Vec<AudioArtifact>),
    Metadata(HashMap<String, serde_json::Value>),
    Raw(Vec<u8>),
}
```

### Storage System

The `ArtifactStorage` provides:

- **Persistent Caching**: Results survive application restarts
- **LRU Eviction**: Automatically manages disk space
- **Content Addressing**: Duplicate results are deduplicated
- **Efficient Retrieval**: Fast access to cached artifacts

**Storage Location:**

```
C:\Users\{User}\AppData\Local\Temp\{package-name}\
├── artifacts/           # Cached operation results
├── test_op_1v1/        # Test operation workspaces
├── merged_op_2v1.wav   # Scheduler operation outputs
└── metadata.json       # Storage metadata
```

## Benefits of the Graph System

### 1. **Smart Recomputation**

- Only affected operations re-run when inputs change
- Unchanged results are reused from cache
- Significant performance improvements for complex pipelines

### 2. **Parallel Execution**

- Independent operations run simultaneously
- Automatic load balancing across CPU cores
- Configurable worker thread pools

### 3. **Dependency Safety**

- Impossible to create circular dependencies
- Guaranteed correct execution order
- Automatic deadlock prevention

### 4. **Incremental Processing**

- Large pipelines can be paused and resumed
- Individual operations can be debugged in isolation
- Partial results are preserved on failure

## Configuration

### Scheduler Configuration

```rust
pub struct SchedulerConfig {
    pub max_workers: usize,
    pub queue_size: usize,
    pub enable_parallel_execution: bool,
    pub worker_timeout: Duration,
    pub artifact_cache_size: usize,
}
```

### Default Settings

- **Workers**: Number of CPU cores
- **Queue Size**: 1000 pending tasks
- **Parallel Execution**: Enabled
- **Worker Timeout**: 5 minutes
- **Cache Size**: 100MB

## Usage Examples

### Simple Pipeline

```rust
// Create operations
let merge_task = CookTask {
    operation_type: "merge".to_string(),
    parameters: json!({
        "crossfade_ms": 100.0,
        "normalize": false
    }),
    // ... other fields
};

let normalize_task = CookTask {
    operation_type: "normalize".to_string(),
    dependencies: vec![merge_task.op_id], // Depends on merge
    // ... other fields
};

// Submit to scheduler
scheduler.submit_task(merge_task)?;
scheduler.submit_task(normalize_task)?;
```

### Testing Operations

The system provides built-in testing through Tauri commands:

```typescript
// Test individual operation
const result = await invoke<string>('test_operation', {
  operationName: 'merge',
});

// Test scheduler with multiple tasks
const schedulerResult = await invoke<string>('test_scheduler');
```

## Debugging and Monitoring

### Logging Integration

The system provides comprehensive logging:

```rust
log_info!(logger, LogSystem::Cook, "Task {} submitted", task.op_id);
log_debug!(logger, LogSystem::Cook, "Executing operation: {}", task.operation_type);
```

### Statistics

Real-time scheduler statistics:

```rust
pub struct SchedulerStats {
    pub is_running: bool,
    pub queued_tasks: usize,
    pub executing_tasks: usize,
    pub completed_tasks: usize,
    pub total_tasks_executed: u64,
    pub max_concurrent_tasks: usize,
}
```

### UI Integration

The Svelte frontend provides:

- **Test Operations**: Individual operation testing
- **Scheduler Testing**: End-to-end pipeline testing
- **Artifacts Browser**: Direct access to generated files
- **Real-time Status**: Live scheduler statistics

## Error Handling

### Graceful Degradation

- **Operation Failures**: Don't crash the entire pipeline
- **Dependency Handling**: Failed operations mark dependents as failed
- **Recovery**: Individual operations can be retried
- **Resource Cleanup**: Temporary files are automatically cleaned up

### Error Types

```rust
pub enum OperationError {
    InvalidInput(String),
    ProcessingError(String),
    IOError(std::io::Error),
    DependencyFailed(OpId),
    Timeout,
}
```

## Performance Considerations

### Optimization Strategies

1. **Caching**: Aggressive result caching reduces redundant work
2. **Parallelism**: Multiple operations execute simultaneously
3. **Streaming**: Large audio files are processed in chunks
4. **Memory Management**: Configurable memory limits prevent OOM

### Monitoring

- **Memory Usage**: Track artifact cache size
- **Execution Time**: Monitor operation duration
- **Queue Depth**: Watch for backlog buildup
- **Error Rate**: Track operation success/failure ratios

## Future Enhancements

### Planned Features

1. **Distributed Execution**: Run operations across multiple machines
2. **GPU Acceleration**: Leverage GPU for audio processing
3. **Visual Pipeline Editor**: Drag-and-drop operation builder
4. **Custom Operations**: User-defined operation plugins
5. **Batch Processing**: Process multiple files simultaneously
6. **Cloud Storage**: Remote artifact caching

### Extension Points

The system is designed for extensibility:

- **New Operation Types**: Implement the `Operation` trait
- **Custom Artifacts**: Add new artifact types
- **Storage Backends**: Pluggable storage systems
- **Execution Engines**: Alternative execution strategies

## Conclusion

The graph-based execution system provides a robust, scalable foundation for complex audio processing pipelines. It combines the benefits of functional programming (immutable artifacts, pure operations) with imperative execution (stateful scheduling, mutable graphs) to create an efficient and maintainable system.

The system is production-ready and provides all the tools needed for debugging, monitoring, and extending functionality as requirements evolve.
