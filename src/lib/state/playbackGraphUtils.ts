import { get } from 'svelte/store';
import { logger } from './logging';
import { invokeWithPerf } from './performance';
import { createTypedEventChannelWithLogging } from '../utils/channelMaker';
import type { BuildGraphRequest, BuildGraphResponse, AddOpRequest } from './opPlaybackService';

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

    // Create typed event channel with logging for build graph progress
    const onEvent = createTypedEventChannelWithLogging<BuildGraphEvent>('BuildGraph', {
      onStarted: data => {
        logger.opPlayback.info(`Build graph started: ${data.operationCount} operations`);
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
        logger.opPlayback.info(
          `Building operation ${data.operationIndex + 1}/${data.totalOperations}: ${data.operationName} (${data.durationSeconds.toFixed(2)}s)`
        );
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
        logger.opPlayback.success(
          `Build graph finished: ${data.operationCount} operations, ${data.totalDurationSeconds.toFixed(2)}s total duration`
        );
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
    });

    const result = await invokeWithPerf<BuildGraphResponse>('op_playback_build_graph', {
      request,
      onEvent: onEvent,
    });

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
