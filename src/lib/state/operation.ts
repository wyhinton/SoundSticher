import { get } from 'svelte/store';
import { appState, AppState, AudioFileItem } from './state.svelte';
import { loggingState, logger } from './logging';
import { groupRegistry, GroupResult } from './groups';

// ============================================================================
// TYPES & INTERFACES
// ============================================================================

export interface OperationsState {
  defs: Record<string, OperationDef>; // serialized operation definitions
  pipelines?: Record<string, string[]>; // optional grouping of operations into pipelines
  _version?: number;
}

/**
 * An operation can be either a file rendering operation (produces output files)
 * or a sample editing operation (modifies audio in-place).
 */
export type OperationDef =
  | CombineOperation
  | TrimOperation
  | NormalizeOperation
  | FadeOperation
  | ReverseOperation
  | PitchShiftOperation
  | TimeStretchOperation
  | SilenceOperation
  | GainOperation
  | PipelineOperation;

// ============================================================================
// FILE RENDERING OPERATIONS (produce new output files)
// ============================================================================

interface SourceItem {
  kind: 'file';
}

export interface CombineOperation {
  kind: 'combine';
  /** Group reference or explicit file IDs to combine */
  source: OperationSource;
  /** Output file path (can use templates like {date}, {name}) */
  sources: SourceItem[];
}

export interface PipelineOperation {
  kind: 'pipeline';
  /** Ordered list of operation references to execute in sequence */
  operations: string[];
  /** Source for the first operation in the pipeline */
  source: OperationSource;
}

export type OperationSource =
  | { type: 'group'; groupRef: string }
  | { type: 'files'; fileIds: string[] }
  | { type: 'all' }
  | { type: 'active' }
  | { type: 'section'; sectionIndex: number }
  | { type: 'previousOperation'; operationRef: string };

export type AudioFormat = 'wav' | 'mp3' | 'flac' | 'ogg' | 'aiff';

export type OperationStatus = 'pending' | 'running' | 'completed' | 'failed' | 'cancelled';

export interface OperationResult {
  operationId: string;
  status: OperationStatus;
  progress?: number; // 0-100
  message?: string;
  error?: string;
  outputFiles?: string[];
  startTime?: number;
  endTime?: number;
}

// ============================================================================
// OPERATION INFO DICTIONARY (for UI)
// ============================================================================

export interface OperationInfo {
  icon: string;
  label: string;
  description: string;
  category: 'render' | 'edit' | 'meta';
  params: string[];
}

export const OperationInfoDictionary: Record<OperationDef['kind'], OperationInfo> = {
  combine: {
    icon: '🔗',
    label: 'Combine',
    description: 'Concatenate multiple audio files into a single output file',
    category: 'render',
    params: ['source', 'outputPath', 'gapSeconds', 'crossfadeSeconds', 'format'],
  },
  trim: {
    icon: '✂️',
    label: 'Trim',
    description: 'Remove audio from the start or end of files',
    category: 'edit',
    params: ['source', 'startSeconds', 'endSeconds', 'keepDurationSeconds'],
  },
  normalize: {
    icon: '📊',
    label: 'Normalize',
    description: 'Adjust audio levels to a target peak or RMS level',
    category: 'edit',
    params: ['source', 'targetPeakDb', 'useRms', 'targetRmsDb'],
  },
  fade: {
    icon: '🌅',
    label: 'Fade',
    description: 'Apply fade in/out effects to audio',
    category: 'edit',
    params: ['source', 'fadeInSeconds', 'fadeOutSeconds', 'curve'],
  },
  reverse: {
    icon: '🔄',
    label: 'Reverse',
    description: 'Reverse the audio playback direction',
    category: 'edit',
    params: ['source'],
  },
  pitchShift: {
    icon: '🎵',
    label: 'Pitch Shift',
    description: 'Change the pitch of audio by semitones',
    category: 'edit',
    params: ['source', 'semitones', 'preserveTempo'],
  },
  timeStretch: {
    icon: '⏱️',
    label: 'Time Stretch',
    description: 'Change the speed/duration of audio',
    category: 'edit',
    params: ['source', 'factor', 'preservePitch'],
  },
  silence: {
    icon: '🔇',
    label: 'Silence',
    description: 'Detect, remove, or trim silence from audio',
    category: 'edit',
    params: ['source', 'action', 'thresholdDb', 'minDurationSeconds'],
  },
  gain: {
    icon: '🔊',
    label: 'Gain',
    description: 'Adjust the volume level of audio',
    category: 'edit',
    params: ['source', 'gainDb'],
  },
  pipeline: {
    icon: '🔀',
    label: 'Pipeline',
    description: 'Chain multiple operations together in sequence',
    category: 'meta',
    params: ['source', 'operations'],
  },
};

// ============================================================================
// OPERATION REGISTRY (similar to GroupRegistry)
// ============================================================================

export type OperationExecutor = (
  state: AppState,
  operation: OperationDef,
  fileIds: Set<string>
) => Promise<OperationResult>;

export class OperationRegistry {
  private cache = new Map<string, { version: number; fileIds: Set<string> }>();

  constructor(private getDefs: () => Record<string, OperationDef> | undefined) {}

  /**
   * Resolve the source files for an operation
   */
  resolveSource(name: string, state: AppState): Set<string> {
    const version = state._version ?? 0;
    const isLogging = get(loggingState).operationsLog;

    // Check cache
    const cached = this.cache.get(name);
    if (cached && cached.version === version) {
      if (isLogging) {
        console.log(`💾 Operations: Using cached source for "${name}" (version ${version})`);
      }
      return cached.fileIds;
    }

    const defs = this.getDefs();
    const def = defs?.[name];

    if (!def) {
      if (isLogging) {
        console.error(`❌ Operations: Unknown operation "${name}"`);
      }
      throw new Error(`Unknown operation "${name}"`);
    }

    if (isLogging) {
      console.log(`🔍 Operations: Resolving source for "${name}"`, def);
    }

    const fileIds = this.resolveOperationSource(def.source, state);

    this.cache.set(name, { version, fileIds });

    if (isLogging) {
      console.log(
        `✅ Operations: Resolved "${name}" -> ${fileIds.size} files`,
        Array.from(fileIds)
      );
    }

    return fileIds;
  }

  /**
   * Resolve an OperationSource to a set of file IDs
   */
  private resolveOperationSource(source: OperationSource, state: AppState): Set<string> {
    switch (source.type) {
      case 'group':
        return groupRegistry.eval(source.groupRef, state);

      case 'files':
        return new Set(source.fileIds);

      case 'all':
        return new Set(state.sections.flatMap(s => s.files.map(f => f.id)));

      case 'active':
        return new Set(state.sections.flatMap(s => s.files.filter(f => f.active).map(f => f.id)));

      case 'section':
        const section = state.sections[source.sectionIndex];
        if (!section) return new Set();
        return new Set(section.files.map(f => f.id));

      case 'previousOperation':
        // For pipeline operations, this would reference output from previous op
        // For now, return empty set as this requires execution context
        console.warn('previousOperation source type requires execution context');
        return new Set();
    }
  }

  /**
   * Get the definition for an operation
   */
  getDefinition(name: string): OperationDef | undefined {
    return this.getDefs()?.[name];
  }

  /**
   * Get all operation names
   */
  getOperationNames(): string[] {
    const defs = this.getDefs();
    return defs ? Object.keys(defs) : [];
  }

  /**
   * Invalidate all cached source resolutions
   */
  invalidateAll() {
    const isLogging = get(loggingState).operationsLog;
    const cacheSize = this.cache.size;

    this.cache.clear();

    if (isLogging) {
      console.log(`🗑️ Operations: Invalidated ${cacheSize} cached entries`);
    }
  }
}

// ============================================================================
// SINGLETON REGISTRY INSTANCE
// ============================================================================

export const operationRegistry = new OperationRegistry(() => {
  return get(appState).operations?.defs;
});

// Subscribe to appState changes to invalidate cache
let lastRev = get(appState)._rev ?? 0;

appState.subscribe(state => {
  const currentRev = state._rev ?? 0;
  const isLogging = get(loggingState).operationsLog;

  if (currentRev !== lastRev) {
    if (isLogging) {
      console.log(
        `🔄 Operations: Content revision changed from ${lastRev} to ${currentRev} - invalidating cache`
      );
    }

    operationRegistry.invalidateAll();
    lastRev = currentRev;
  }
});

// ============================================================================
// CRUD OPERATIONS FOR OPERATIONS STATE
// ============================================================================

/**
 * Add a new operation definition
 */
export function addOperation(name: string, def: OperationDef): void {
  const isLogging = get(loggingState).operationsLog;

  if (isLogging) {
    console.log(`➕ Operations: Adding operation "${name}"`, def);
  }

  appState.update(s => {
    if (!s.operations) {
      s.operations = { defs: {}, _version: 1 };
    }

    s.operations.defs[name] = def;
    s.operations._version = (s.operations._version ?? 0) + 1;
    s._rev = (s._rev ?? 0) + 1;

    return s;
  });
}

/**
 * Update an existing operation's parameters
 */
export function updateOperation(
  name: string,
  patch: Partial<OperationDef>,
  expectedKind?: OperationDef['kind']
): void {
  const isLogging = get(loggingState).operationsLog;

  if (isLogging) {
    console.log(`📝 Operations: Updating operation "${name}"`, { patch, expectedKind });
  }

  appState.update(s => {
    const def = s.operations?.defs?.[name];
    if (!def) {
      if (isLogging) {
        console.warn(`⚠️ Operations: Cannot update "${name}" - not found`);
      }
      return s;
    }

    if (expectedKind && def.kind !== expectedKind) {
      if (isLogging) {
        console.warn(
          `⚠️ Operations: Cannot update "${name}" - expected kind "${expectedKind}" but got "${def.kind}"`
        );
      }
      return s;
    }

    s.operations!.defs[name] = { ...def, ...patch } as OperationDef;
    s.operations!._version = (s.operations!._version ?? 0) + 1;
    s._rev = (s._rev ?? 0) + 1;

    if (isLogging) {
      console.log(`✅ Operations: Updated "${name}" successfully`);
    }

    return s;
  });
}

/**
 * Delete an operation
 */
export function deleteOperation(name: string): void {
  const isLogging = get(loggingState).operationsLog;

  if (isLogging) {
    console.log(`🗑️ Operations: Deleting operation "${name}"`);
  }

  appState.update(s => {
    if (!s.operations?.defs?.[name]) {
      if (isLogging) {
        console.warn(`⚠️ Operations: Cannot delete "${name}" - not found`);
      }
      return s;
    }

    delete s.operations.defs[name];

    // Also remove from any pipelines
    if (s.operations.pipelines) {
      for (const pipelineName of Object.keys(s.operations.pipelines)) {
        const pipeline = s.operations.pipelines[pipelineName];
        if (pipeline) {
          s.operations.pipelines[pipelineName] = pipeline.filter(op => op !== name);
        }
      }
    }

    s.operations._version = (s.operations._version ?? 0) + 1;
    s._rev = (s._rev ?? 0) + 1;

    return s;
  });
}

/**
 * Delete all operations
 */
export function deleteAllOperations(): void {
  const isLogging = get(loggingState).operationsLog;

  if (isLogging) {
    console.log(`🗑️ Operations: Deleting all operations`);
  }

  appState.update(s => {
    if (!s.operations) {
      s.operations = { defs: {}, _version: 1 };
    }

    s.operations.defs = {};
    s.operations.pipelines = {};
    s.operations._version = (s.operations._version ?? 0) + 1;
    s._rev = (s._rev ?? 0) + 1;

    return s;
  });
}

/**
 * Add operation to a pipeline
 */
export function addToPipeline(pipelineName: string, operationName: string): void {
  appState.update(s => {
    if (!s.operations) {
      s.operations = { defs: {}, pipelines: {}, _version: 1 };
    }
    if (!s.operations.pipelines) {
      s.operations.pipelines = {};
    }
    if (!s.operations.pipelines[pipelineName]) {
      s.operations.pipelines[pipelineName] = [];
    }

    if (!s.operations.pipelines[pipelineName].includes(operationName)) {
      s.operations.pipelines[pipelineName].push(operationName);
      s.operations._version = (s.operations._version ?? 0) + 1;
      s._rev = (s._rev ?? 0) + 1;
    }

    return s;
  });
}

/**
 * Remove operation from a pipeline
 */
export function removeFromPipeline(pipelineName: string, operationName: string): void {
  appState.update(s => {
    if (!s.operations?.pipelines?.[pipelineName]) return s;

    s.operations.pipelines[pipelineName] = s.operations.pipelines[pipelineName].filter(
      op => op !== operationName
    );
    s.operations._version = (s.operations._version ?? 0) + 1;
    s._rev = (s._rev ?? 0) + 1;

    return s;
  });
}

// ============================================================================
// TEST/EXAMPLE OPERATIONS
// ============================================================================

export interface NamedOperationDef {
  name: string;
  def: OperationDef;
}

export const testOperations: NamedOperationDef[] = [
  {
    name: 'combine_active',
    def: {
      kind: 'combine',
      source: { type: 'active' },
      outputPath: 'output/combined_{date}.wav',
      gapSeconds: 0,
      format: 'wav',
    },
  },
  {
    name: 'master_pipeline',
    def: {
      kind: 'pipeline',
      source: { type: 'active' },
      operations: ['combine_active'],
    },
  },
];

/**
 * Add test operations to state (for development/debugging)
 */
export function addTestOperations(): void {
  appState.update(state => {
    if (!state.operations) {
      state.operations = { defs: {}, pipelines: {}, _version: 1 };
    }

    testOperations.forEach(op => {
      state.operations!.defs[op.name] = op.def;
    });

    state.operations.pipelines = {
      ...state.operations.pipelines,
      'Audio Processing': ['normalize_all', 'fade_section_0', 'trim_silence'],
      'Final Output': ['combine_active', 'master_pipeline'],
    };

    state.operations._version = (state.operations._version ?? 0) + 1;
    state._rev = (state._rev ?? 0) + 1;

    return state;
  });

  console.log('🧪 Test operations added');
}
