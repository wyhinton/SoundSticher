import { Channel } from '@tauri-apps/api/core';
import { publishStatus, clearSource } from '$lib/state/status';
import type { StatusLevel } from '$lib/state/status';

// Type utility to extract data types for each event stage
type ExtractEventData<T, E extends string> = T extends { event: E; data: infer D } ? D : never;

// Type-safe event handlers that infer correct data types for each event
export interface TypedChannelEventHandlers<TEvent extends { event: string; data: any }> {
  onStarted?: (data: ExtractEventData<TEvent, 'started'>) => void;
  onProgress?: (data: ExtractEventData<TEvent, 'progress'>) => void;
  onFinished?: (data: ExtractEventData<TEvent, 'finished'>) => void;
}

// Legacy interface for backward compatibility
export interface ChannelEventHandlers<TData = any> {
  onStarted?: (data: TData) => void;
  onProgress?: (data: TData) => void;
  onFinished?: (data: TData) => void;
}

export interface ChannelEvent<TData = any> {
  event: 'started' | 'progress' | 'finished';
  data: TData;
}

/**
 * Creates a type-safe channel with properly inferred data types for each event stage
 * @param handlers Object containing optional handlers for each event type with proper typing
 * @returns Configured Channel instance
 */
export function createTypedEventChannel<TEvent extends { event: string; data: any }>(
  handlers: TypedChannelEventHandlers<TEvent>
): Channel<TEvent> {
  const channel = new Channel<TEvent>();

  channel.onmessage = (message: TEvent) => {
    switch (message.event) {
      case 'started':
        handlers.onStarted?.(message.data as ExtractEventData<TEvent, 'started'>);
        break;
      case 'progress':
        handlers.onProgress?.(message.data as ExtractEventData<TEvent, 'progress'>);
        break;
      case 'finished':
        handlers.onFinished?.(message.data as ExtractEventData<TEvent, 'finished'>);
        break;
      default:
        console.warn('Unknown channel event:', message.event);
    }
  };

  return channel;
}

/**
 * Creates a channel with handlers for started, progress, and finished events
 * @param handlers Object containing optional handlers for each event type
 * @returns Configured Channel instance
 * @deprecated Use createTypedEventChannel for better type safety
 */
export function createEventChannel<TData = any>(
  handlers: ChannelEventHandlers<TData>
): Channel<ChannelEvent<TData>> {
  const channel = new Channel<ChannelEvent<TData>>();

  channel.onmessage = message => {
    switch (message.event) {
      case 'started':
        handlers.onStarted?.(message.data);
        break;
      case 'progress':
        handlers.onProgress?.(message.data);
        break;
      case 'finished':
        handlers.onFinished?.(message.data);
        break;
      default:
        console.warn('Unknown channel event:', message.event);
    }
  };

  return channel;
}

/**
 * Creates a type-safe channel with styled logging for development
 * @param channelName Name to display in logs (e.g., 'Export', 'Combine')
 * @param handlers Event handlers with proper type inference
 * @returns Configured Channel instance
 */
export function createTypedEventChannelWithLogging<TEvent extends { event: string; data: any }>(
  channelName: string,
  handlers: TypedChannelEventHandlers<TEvent>
): Channel<TEvent> {
  const isDev =
    typeof import.meta !== 'undefined' &&
    typeof (import.meta as any).env !== 'undefined' &&
    (import.meta as any).env.DEV === true;

  const logEvent = (event: string, data?: any) => {
    if (isDev) {
      console.log(
        `%c📡 ${channelName} %c${event}`,
        'background: #2196F3; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
        'color: #2196F3; font-weight: normal;',
        data
      );
    }
  };

  return createTypedEventChannel<TEvent>({
    onStarted: data => {
      logEvent('Started', data);
      handlers.onStarted?.(data);
    },
    onProgress: data => {
      logEvent('Progress', data);
      handlers.onProgress?.(data);
    },
    onFinished: data => {
      logEvent('Finished', data);
      handlers.onFinished?.(data);
    },
  });
}

/**
 * Creates a channel with styled logging for development
 * @param channelName Name to display in logs (e.g., 'Export', 'Combine')
 * @param handlers Event handlers
 * @returns Configured Channel instance
 * @deprecated Use createTypedEventChannelWithLogging for better type safety
 */
export function createEventChannelWithLogging<TData = any>(
  channelName: string,
  handlers: ChannelEventHandlers<TData>
): Channel<ChannelEvent<TData>> {
  const isDev =
    typeof import.meta !== 'undefined' &&
    typeof (import.meta as any).env !== 'undefined' &&
    (import.meta as any).env.DEV === true;

  const logEvent = (event: string, data?: TData) => {
    if (isDev) {
      console.log(
        `%c📡 ${channelName} %c${event}`,
        'background: #2196F3; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
        'color: #2196F3; font-weight: normal;',
        data
      );
    }
  };

  return createEventChannel<TData>({
    onStarted: data => {
      logEvent('Started', data);
      handlers.onStarted?.(data);
    },
    onProgress: data => {
      logEvent('Progress', data);
      handlers.onProgress?.(data);
    },
    onFinished: data => {
      logEvent('Finished', data);
      handlers.onFinished?.(data);
    },
  });
}

/**
 * Configuration for automatic status message generation
 */
export interface StatusMessageConfig {
  /** Source identifier for status system (e.g., 'build-graph', 'export') */
  source: string;
  /** Message to show when started (can use template ${data.propertyName}) */
  startedMessage?: string | ((data: any) => string);
  /** Message to show during progress (can use template ${data.propertyName}) */
  progressMessage?: string | ((data: any) => string);
  /** Message to show when finished (can use template ${data.propertyName}) */
  finishedMessage?: string | ((data: any) => string);
  /** Whether to auto-clear success status after completion (default: 2000ms) */
  autoClearSuccess?: boolean | number;
  /** Function to extract progress from progress event data (0-1 scale) */
  getProgress?: (data: any) => number | undefined;
}

/**
 * Creates a type-safe channel with logging AND automatic status publishing
 * Reduces boilerplate by automatically publishing status for started/progress/finished events
 *
 * @param channelName Name to display in logs (e.g., 'Export', 'BuildGraph')
 * @param statusConfig Configuration for automatic status messages
 * @param handlers Event handlers with proper type inference
 * @returns Configured Channel instance
 *
 * @example
 * ```typescript
 * const channel = createTypedEventChannelWithLoggingAndStatusMessages<BuildGraphEvent>(
 *   'BuildGraph',
 *   {
 *     source: 'build-graph',
 *     startedMessage: data => `Building ${data.operationCount} operations...`,
 *     progressMessage: data => `Building: ${data.operationName} (${data.operationIndex + 1}/${data.totalOperations})`,
 *     finishedMessage: data => `Graph built: ${data.operationCount} ops, ${data.totalDurationSeconds.toFixed(1)}s`,
 *     getProgress: data => (data.operationIndex + 1) / data.totalOperations,
 *     autoClearSuccess: 2000
 *   },
 *   {
 *     onStarted: data => console.log('Started!'),
 *     onProgress: data => console.log('Progress:', data),
 *     onFinished: data => console.log('Done!')
 *   }
 * );
 * ```
 */
export function createTypedEventChannelWithLoggingAndStatusMessages<
  TEvent extends { event: string; data: any },
>(
  channelName: string,
  statusConfig: StatusMessageConfig,
  handlers: TypedChannelEventHandlers<TEvent>
): Channel<TEvent> {
  const isDev =
    typeof import.meta !== 'undefined' &&
    typeof (import.meta as any).env !== 'undefined' &&
    (import.meta as any).env.DEV === true;

  const logEvent = (event: string, data?: any) => {
    if (isDev) {
      console.log(
        `%c📡 ${channelName} %c${event}`,
        'background: #2196F3; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
        'color: #2196F3; font-weight: normal;',
        data
      );
    }
  };

  const getMessage = (
    template: string | ((data: any) => string) | undefined,
    data: any
  ): string => {
    if (!template) return '';
    return typeof template === 'function' ? template(data) : template;
  };

  return createTypedEventChannel<TEvent>({
    onStarted: data => {
      logEvent('Started', data);

      // Clear previous statuses and publish started status
      clearSource(statusConfig.source);
      if (statusConfig.startedMessage) {
        publishStatus({
          source: statusConfig.source,
          level: 'working',
          message: getMessage(statusConfig.startedMessage, data),
          progress: 0,
        });
      }

      handlers.onStarted?.(data);
    },
    onProgress: data => {
      logEvent('Progress', data);

      // Update progress status
      if (statusConfig.progressMessage) {
        clearSource(statusConfig.source);
        publishStatus({
          source: statusConfig.source,
          level: 'working',
          message: getMessage(statusConfig.progressMessage, data),
          progress: statusConfig.getProgress?.(data),
        });
      }

      handlers.onProgress?.(data);
    },
    onFinished: data => {
      logEvent('Finished', data);

      // Publish success status
      if (statusConfig.finishedMessage) {
        clearSource(statusConfig.source);
        publishStatus({
          source: statusConfig.source,
          level: 'success',
          message: getMessage(statusConfig.finishedMessage, data),
        });

        // Auto-clear if configured
        if (statusConfig.autoClearSuccess !== false) {
          const delay =
            typeof statusConfig.autoClearSuccess === 'number'
              ? statusConfig.autoClearSuccess
              : 2000;
          setTimeout(() => clearSource(statusConfig.source), delay);
        }
      }

      handlers.onFinished?.(data);
    },
  });
}
