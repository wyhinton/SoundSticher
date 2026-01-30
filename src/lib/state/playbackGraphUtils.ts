import { get } from 'svelte/store';
import { logger } from './logging';
import { invokeWithPerf } from './performance';
import { createTypedEventChannelWithLoggingAndStatusMessages } from '../utils/channelMaker';
import type { BuildGraphRequest, BuildGraphResponse, AddOpRequest } from './opPlaybackService';
import { useSampleCache } from './opPlaybackService';

/**
 * Typed event definitions for build graph progress
 */
type BuildGraphStartedEvent = {
  event: 'started';
  data: { operationCount: number };
};

type BuildGraphProgressEvent = {
  event: 'progress';
  data: {
    operationName: string;
    operationIndex: number;
    totalOperations: number;
    durationSeconds: number;
  };
};

type BuildGraphFinishedEvent = {
  event: 'finished';
  data: {
    operationCount: number;
    totalDurationSeconds: number;
    sampleRate: number;
    channels: number;
  };
};

type BuildGraphEvent = BuildGraphStartedEvent | BuildGraphProgressEvent | BuildGraphFinishedEvent;

/**
 * Internal state update function type
 */
type StateUpdater = (updater: (state: any) => any) => void;

/**
 * Build a playback graph from operations using typed event channels
 */
export async function buildGraph(
  request: BuildGraphRequest,
  updateState: StateUpdater
): Promise<BuildGraphResponse> {
  logger.opPlayback.info(`Building playback graph with ${request.operations.length} operations`);

  // Track build time
  let buildStartTime: number;

  try {
    // Reset build state
    updateState(s => ({
      ...s,
      hasGraph: false,
      buildState: {
        isBuilding: true,
        currentIndex: 0,
        totalOperations: request.operations.length,
        buildProgress: 0,
      },
    }));

    // Create typed event channel with automatic logging and status publishing
    const onBuildGraphEvent = createTypedEventChannelWithLoggingAndStatusMessages<BuildGraphEvent>(
      'BuildGraph',
      {
        source: 'build-graph',
        startedMessage: data => `Building playback graph (${data.operationCount} operations)...`,
        progressMessage: data =>
          `Building: ${data.operationName} (${data.operationIndex + 1}/${data.totalOperations})`,
        finishedMessage: data => {
          const buildTimeMs = Date.now() - buildStartTime;
          const buildTimeSec = (buildTimeMs / 1000).toFixed(2);
          const audioDuration = data.totalDurationSeconds.toFixed(1);
          return `Built ${data.operationCount} ops in ${buildTimeSec}s → ${audioDuration}s audio`;
        },
        getProgress: data => (data.operationIndex + 1) / data.totalOperations,
        autoClearSuccess: 2000,
      },
      {
        onStarted: data => {
          // Record build start time
          buildStartTime = Date.now();

          updateState(s => ({
            ...s,
            buildState: {
              isBuilding: true,
              currentIndex: 0,
              totalOperations: data.operationCount,
              buildProgress: 0,
            },
          }));
        },
        onProgress: data => {
          const buildProgress = (data.operationIndex + 1) / data.totalOperations;
          updateState(s => ({
            ...s,
            buildState: {
              ...s.buildState,
              currentOperation: data.operationName,
              currentIndex: data.operationIndex,
              totalOperations: data.totalOperations,
              buildProgress,
            },
          }));
        },
        onFinished: data => {
          updateState(s => ({
            ...s,
            hasGraph: true,
            durationSeconds: data.totalDurationSeconds,
            progress: 0,
            positionSeconds: 0,
            isPlaying: false,
            isPaused: false,
            buildState: {
              isBuilding: false,
              currentIndex: data.operationCount,
              totalOperations: data.operationCount,
              buildProgress: 1.0,
            },
          }));
        },
      }
    );

    const result = await invokeWithPerf<BuildGraphResponse>(
      get(useSampleCache) ? 'op_playback_build_graph' : 'op_playback_build_graph_legacy',
      {
        request,
        onEvent: onBuildGraphEvent,
      }
    );

    if (!result.ok) {
      throw new Error(`Failed to build graph: ${result.error.message}`);
    }

    const response = result.value;

    logger.opPlayback.success(
      `Graph built: ${response.operationCount} ops, ${response.totalDurationSeconds.toFixed(2)}s duration`
    );

    return response;
  } catch (error) {
    logger.opPlayback.error('Failed to build graph:', error);

    // Error status is handled by re-importing for this specific case
    const { publishStatus, clearSource } = await import('./status');
    clearSource('build-graph');
    publishStatus({
      source: 'build-graph',
      level: 'error',
      message: `Failed to build graph: ${error instanceof Error ? error.message : 'Unknown error'}`,
      sticky: true,
    });

    updateState(s => ({
      ...s,
      buildState: {
        ...s.buildState,
        isBuilding: false,
      },
    }));
    throw error;
  }
}

/**
 * Build a graph from file paths with automatic timing
 * Operations are scheduled sequentially (one after another)
 */
export async function buildGraphFromFiles(
  filePaths: string[],
  updateState: StateUpdater,
  options: {
    sampleRate?: number;
    channels?: number;
    loopPlayback?: boolean;
    gap?: number; // Gap between files in seconds
  } = {}
): Promise<BuildGraphResponse> {
  const { gap = 0 } = options;

  // For now, we estimate duration from file - in practice the backend will determine actual duration
  // We'll schedule operations sequentially with estimated 5 second durations as placeholder
  // The backend will use actual durations

  let currentTime = 0;
  const operations: AddOpRequest[] = filePaths.map((filePath, index) => {
    const op: AddOpRequest = {
      name: `file_${index}`,
      filePath,
      startTime: currentTime,
      gain: 1.0,
    };
    // Estimate 5 seconds per file for now - backend will override with actual
    currentTime += 5 + gap;
    return op;
  });

  return buildGraph(
    {
      operations,
      sampleRate: options.sampleRate,
      channels: options.channels,
      loopPlayback: options.loopPlayback,
    },
    updateState
  );
}
