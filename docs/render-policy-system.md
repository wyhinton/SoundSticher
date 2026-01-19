# Render Policy System

## Overview

The **Render Policy System** provides fine-grained control over when operations re-render in response to upstream changes. This is a graph-level invalidation mechanism that allows users to optimize rendering performance and control data flow.

## Core Concepts

### 1. Render Policies

Operations support three render policies:

```typescript
type RenderPolicy = 'auto' | 'manual' | 'frozen';
```

| Policy       | Behavior                                           | Use Case                                            |
| ------------ | -------------------------------------------------- | --------------------------------------------------- |
| **`auto`**   | Re-render whenever any upstream input changes      | Default behavior, good for active development       |
| **`manual`** | Never auto-rerender, only on explicit user request | Full user control, good for expensive operations    |
| **`frozen`** | Treat output as immutable, cut invalidation chain  | Cache stable results, prevent downstream re-renders |

### 2. Invalidation Chain

The system models operations as a directed acyclic graph (DAG):

```
A → B → C → D
```

When operation `A` changes:

- **Normal behavior**: A, B, C, D all re-render
- **If B is frozen**: Only A re-renders; B, C, D remain unchanged
- **If C is frozen**: A and B re-render; C and D remain unchanged

**Key principle**: Frozen operations act as **invalidation barriers** in the dependency graph.

### 3. Separation of Concerns

The system cleanly separates:

| Concern                      | Frozen?    | Notes                                   |
| ---------------------------- | ---------- | --------------------------------------- |
| **Resolve input files**      | ✅ Always  | Source resolution ignores render policy |
| **Cache output**             | ✅ Always  | Frozen ops keep cached outputs          |
| **Auto-rerender on changes** | ❌ Blocked | This is what freezing controls          |
| **Manual re-render**         | ✅ Always  | Users can force re-render anytime       |

## Architecture

### Data Model

Render policy is stored in `BaseOperation`:

```typescript
interface BaseOperation {
  id: OperationId;
  name: string;
  renderPolicy?: RenderPolicy; // defaults to 'auto'
}
```

This makes it:

- **Serializable**: Persists with operation definitions
- **Undo-friendly**: Changes tracked in undo/redo system
- **Generic**: Applies to all operation types equally

### Key Functions

#### 1. `createOperation()`

Operations default to `'auto'` policy on creation:

```typescript
const opId = createOperation('My Merge', {
  kind: 'merge',
  sources: [...],
  outputPath: '...',
  gapSeconds: 0,
  format: 'wav'
}, 'frozen'); // Optional: override default policy
```

#### 2. `setRenderPolicy()`

Change policy at any time:

```typescript
import { setRenderPolicy } from '$lib/state/operation';

setRenderPolicy(operationId, 'frozen');
```

#### 3. `toggleFreezeOperation()`

Quick toggle between `'auto'` and `'frozen'`:

```typescript
import { toggleFreezeOperation } from '$lib/state/operation';

toggleFreezeOperation(operationId);
// auto → frozen → auto → ...
```

#### 4. `shouldRerender()`

Check if an operation should re-render:

```typescript
import { shouldRerender } from '$lib/state/operation';

const op = getOperationById(opId);
const upstreamChanged = true;

if (shouldRerender(op, upstreamChanged)) {
  // Trigger re-render
}
```

#### 5. `computeInvalidatedOps()`

Compute full invalidation set after a change:

```typescript
import { computeInvalidatedOps } from '$lib/state/operation';

const invalidated = computeInvalidatedOps(changedOpId);
// Returns: Set<OperationId> of ops that need re-rendering
```

This function:

- Traverses the dependency graph downstream
- Respects frozen operations as barriers
- Returns only ops that should actually re-render

#### 6. `getUpstreamOps()` / `getDownstreamOps()`

Navigate the dependency graph:

```typescript
import { getUpstreamOps, getDownstreamOps } from '$lib/state/operation';

const inputs = getUpstreamOps(opId); // What this op depends on
const outputs = getDownstreamOps(opId); // What depends on this op
```

## UI Integration

### OpSettingsTools Component

The `OpSettingsTools.svelte` component provides visual controls:

```svelte
<OpSettingsTools {operationId} {operationName} />
```

**Features**:

- 🚩 **Render button**: Force manual re-render (TODO: implementation)
- ❄️ **Freeze button**: Toggle between auto/frozen policies
- 🔓 **Unfrozen indicator**: Shows when operation is frozen
- **Visual feedback**: Blue glow and background when frozen

**States**:

- Normal (auto): ❄️ snowflake icon, no background
- Frozen: 🔓 unlock icon, blue background + glow

### Status Badges (Future)

Consider adding policy indicators to operation nodes:

```svelte
{#if op.renderPolicy === 'frozen'}
  <span class="badge frozen">🧊 Frozen</span>
{:else if op.renderPolicy === 'manual'}
  <span class="badge manual">✋ Manual</span>
{/if}
```

## Usage Patterns

### Pattern 1: Expensive Intermediate Results

```typescript
// Create a complex merge operation
const mergeId = createOperation('Master Mix', {
  kind: 'merge',
  sources: [...100sources],
  outputPath: './output/master.wav',
  gapSeconds: 0,
  format: 'wav'
});

// Freeze it once rendered
setRenderPolicy(mergeId, 'frozen');

// Now you can modify downstream operations without
// re-rendering this expensive 100-file merge
```

### Pattern 2: Stable Reference Points

```typescript
// Operation A: Raw recording (rarely changes)
const rawId = createOperation('Raw Recording', {...}, 'frozen');

// Operation B: Processed version (can iterate freely)
const processedId = createOperation('Processed', {
  kind: 'merge',
  sources: [{ type: 'operation', operationId: rawId }],
  ...
});

// Changes to processedId won't trigger re-render of rawId
```

### Pattern 3: Manual Control

```typescript
// Heavy processing operation
const heavyId = createOperation('Heavy Process', {...}, 'manual');

// User must explicitly trigger renders
// Good for GPU-intensive or long-running tasks
```

## Implementation Notes

### What NOT to Do

❌ **Don't put policy checks in executors**:

```typescript
// BAD
async function executeMergeOp(op: MergeOp) {
  if (op.renderPolicy === 'frozen') return; // Wrong layer!
  // ...
}
```

✅ **Do check policy before execution**:

```typescript
// GOOD
if (shouldRerender(op, upstreamChanged)) {
  await executeMergeOp(op);
}
```

❌ **Don't tie policies to operation kinds**:

```typescript
// BAD
if (op.kind === 'merge') {
  // Only merges can be frozen? Wrong!
}
```

✅ **Do treat policies generically**:

```typescript
// GOOD - works for any operation type
const policy = op.renderPolicy || 'auto';
```

### Executor Responsibilities

Operation executors remain **pure and deterministic**:

```typescript
async function executeMergeOp(op: MergeOp): Promise<OperationResult> {
  // 1. Resolve inputs (always, regardless of policy)
  const files = resolveOperationSources(op.sources);

  // 2. Process audio (deterministic)
  const output = await mergeAudioFiles(files, op);

  // 3. Return result
  return { status: 'completed', outputFiles: [output] };
}
```

**No policy checks** in the executor. Policy is enforced at the **orchestration layer**.

### Cache Integration (Future)

When implementing render caching:

```typescript
type RenderCacheEntry = {
  operationId: OperationId;
  inputHash: string; // Hash of all upstream inputs
  outputFiles: string[]; // Cached output file paths
  timestamp: number; // When this was rendered
  policy: RenderPolicy; // Policy at time of caching
};
```

Invalidation logic:

1. Check if operation has cached output
2. If `renderPolicy === 'frozen'`, use cache (skip re-render)
3. If `renderPolicy === 'auto'`, check input hash:
   - Hash matches → use cache
   - Hash differs → re-render and update cache
4. If `renderPolicy === 'manual'`, use cache until explicit re-render

## Testing

### Unit Tests

```typescript
import {
  createOperation,
  setRenderPolicy,
  computeInvalidatedOps,
  shouldRerender
} from '$lib/state/operation';

test('frozen operations block invalidation chain', () => {
  const a = createOperation('A', {...});
  const b = createOperation('B', { sources: [{ type: 'operation', operationId: a }] });
  const c = createOperation('C', { sources: [{ type: 'operation', operationId: b }] });

  // Freeze B
  setRenderPolicy(b, 'frozen');

  // Change A
  const invalidated = computeInvalidatedOps(a);

  // Only A should be invalidated, not B or C
  expect(invalidated.has(b)).toBe(false);
  expect(invalidated.has(c)).toBe(false);
});

test('shouldRerender respects policies', () => {
  const op = createOperation('Test', {...});

  // Auto policy
  expect(shouldRerender(getOperationById(op), true)).toBe(true);

  // Frozen policy
  setRenderPolicy(op, 'frozen');
  expect(shouldRerender(getOperationById(op), true)).toBe(false);

  // Manual policy
  setRenderPolicy(op, 'manual');
  expect(shouldRerender(getOperationById(op), true)).toBe(false);
});
```

### Integration Tests

Test the full UI workflow:

1. Create merge operation (defaults to auto)
2. Click freeze button → policy changes to frozen
3. Modify upstream operation → merge doesn't re-render
4. Click render button → forces re-render despite frozen policy
5. Click freeze button again → policy returns to auto

## Future Enhancements

### 1. Smart Freeze

Auto-freeze operations after N renders without user changes:

```typescript
type SmartFreezeConfig = {
  enabled: boolean;
  threshold: number; // Auto-freeze after N identical renders
};
```

### 2. Freeze Groups

Freeze entire sub-graphs at once:

```typescript
function freezeSubgraph(rootOpId: OperationId) {
  const downstream = getAllDownstreamOps(rootOpId);
  for (const opId of downstream) {
    setRenderPolicy(opId, 'frozen');
  }
}
```

### 3. Policy Presets

Save/load policy configurations:

```typescript
type PolicyPreset = {
  name: string;
  policies: Record<OperationId, RenderPolicy>;
};

function applyPolicyPreset(preset: PolicyPreset) {
  for (const [opId, policy] of Object.entries(preset.policies)) {
    setRenderPolicy(opId, policy);
  }
}
```

### 4. Render Scheduling

Queue re-renders with priorities:

```typescript
type RenderPriority = 'high' | 'normal' | 'low';

function scheduleRender(opId: OperationId, priority: RenderPriority) {
  // Add to render queue based on priority
}
```

## Summary

The render policy system provides:

✅ **User control** over when operations re-render  
✅ **Performance optimization** via frozen operations  
✅ **Clean architecture** separating invalidation from execution  
✅ **Future-proof design** for caching and scheduling  
✅ **Undo/redo support** through serialized state

Use frozen operations to create stable checkpoints in your audio processing pipeline and prevent unnecessary re-renders of expensive operations.
