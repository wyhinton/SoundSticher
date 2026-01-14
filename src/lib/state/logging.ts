import { persisted } from 'svelte-persisted-store';
import { get } from 'svelte/store';
import { writable } from 'svelte/store';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';

export interface LoggingState {
  groupsLog: boolean;
  selectionLog: boolean;
  dragdropLog: boolean;
  dragStoreLog: boolean;
  operationsLog: boolean;
  waveformLog: boolean;
  opPlaybackLog: boolean;
  timelineLog: boolean;
  listenersLog: boolean;
  d3timelinemanagerLog: boolean;
  // Backend logging system toggles
  encoderLog: boolean;
  combineLog: boolean;
  playbackLog: boolean;
  sortingLog: boolean;
  waveformBackendLog: boolean;
  timelineBackendLog: boolean;
  operationLog: boolean;
  eventEmitsLog: boolean;
  // Add future logging categories here
  // performanceLog?: boolean;
  // audioLog?: boolean;
  // uiLog?: boolean;
}

export interface FileLocation {
  filePath: string;
  lineNumber?: number;
}

export interface BackendLogMessage {
  timestamp: number;
  level: 'debug' | 'info' | 'warning' | 'error';
  system:
    | 'encoder'
    | 'combine'
    | 'playback'
    | 'sorting'
    | 'cook'
    | 'graph'
    | 'waveform'
    | 'timeline'
    | 'operation'
    | 'eventEmits';
  category?: string;
  message: string;
  data?: any;
  fileLocation?: FileLocation;
}

export const loggingState = persisted<LoggingState>('loggingState', {
  groupsLog: false,
  selectionLog: false,
  dragdropLog: false,
  dragStoreLog: false,
  operationsLog: false,
  waveformLog: false,
  opPlaybackLog: false,
  timelineLog: false,
  listenersLog: false,
  d3timelinemanagerLog: false,
  encoderLog: false,
  combineLog: false,
  playbackLog: false,
  sortingLog: false,
  waveformBackendLog: false,
  timelineBackendLog: false,
  operationLog: false,
  eventEmitsLog: false,
});

// Store for backend log messages
export const backendLogs = writable<BackendLogMessage[]>([]);

// Interface for frontend listener log messages
export interface ListenerLogMessage {
  timestamp: number;
  elementType: string;
  eventType: string;
  action: 'attach' | 'detach' | 'event';
  elementId?: string;
  elementClass?: string;
  details?: any;
}

// Store for frontend listener log messages
export const listenerLogs = writable<ListenerLogMessage[]>([]);

// Helper function to add listener log
export const addListenerLog = (log: Omit<ListenerLogMessage, 'timestamp'>) => {
  const logMessage: ListenerLogMessage = {
    ...log,
    timestamp: Date.now(),
  };

  listenerLogs.update(logs => {
    const newLogs = [...logs, logMessage];
    // Keep only the last 1000 messages
    return newLogs.slice(-1000);
  });
};

// Initialize backend log listener
let isListenerInitialized = false;

export const initializeBackendLogListener = () => {
  if (isListenerInitialized) return;

  isListenerInitialized = true;

  // Listen for backend log messages
  listen<BackendLogMessage>('backend-log', event => {
    const logMessage = event.payload;

    // Add to backend logs store
    backendLogs.update(logs => {
      const newLogs = [...logs, logMessage];
      // Keep only the last 1000 messages
      return newLogs.slice(-1000);
    });

    // Also log to console with styled output
    const { level, system, category, message, data } = logMessage;
    const timestamp = new Date(logMessage.timestamp).toLocaleTimeString();

    const levelColors = {
      debug: { bg: '#9E9E9E', color: 'white' },
      info: { bg: '#2196F3', color: 'white' },
      warning: { bg: '#FF9800', color: 'white' },
      error: { bg: '#f44336', color: 'white' },
    };

    const systemColors = {
      encoder: { bg: '#4CAF50', color: 'white' },
      combine: { bg: '#FF5722', color: 'white' },
      playback: { bg: '#9C27B0', color: 'white' },
      sorting: { bg: '#3F51B5', color: 'white' },
      waveform: { bg: '#9C27B0', color: 'white' },
      cook: { bg: '#607D8B', color: 'white' },
      graph: { bg: '#795548', color: 'white' },
      timeline: { bg: '#FF6F00', color: 'white' },
      operation: { bg: '#E91E63', color: 'white' },
      eventEmits: { bg: '#00BCD4', color: 'white' },
    };

    const levelEmojis = {
      debug: '🔍',
      info: 'ℹ️',
      warning: '⚠️',
      error: '❌',
    };

    const levelColor = levelColors[level];
    const systemColor = systemColors[system];
    const emoji = levelEmojis[level];

    const categoryStr = category ? `[${category}] ` : '';

    const logMethod =
      level === 'error' ? console.error : level === 'warning' ? console.warn : console.log;

    logMethod(
      `%c${emoji} ${level.toUpperCase()} %c%c ${system.toUpperCase()} %c${timestamp} ${categoryStr}${message}`,
      `background: ${levelColor.bg}; color: ${levelColor.color}; padding: 2px 4px; border-radius: 3px; font-weight: bold;`,
      'color: transparent; font-size: 0;', // spacer
      `background: ${systemColor.bg}; color: ${systemColor.color}; padding: 2px 4px; border-radius: 3px; font-weight: bold;`,
      'color: inherit; font-weight: normal;',
      data ? data : ''
    );
  });
};

// Update backend logging configuration
export const updateBackendLoggingConfig = async (config: Partial<LoggingState>) => {
  try {
    const backendConfig = {
      encoder_enabled: config.encoderLog ?? false,
      combine_enabled: config.combineLog ?? false,
      playback_enabled: config.playbackLog ?? false,
      sorting_enabled: config.sortingLog ?? false,
      waveform_enabled: config.waveformBackendLog ?? false,
      console_output: true,
      timeline_enabled: config.timelineBackendLog ?? false,
      operation_enabled: config.operationLog ?? false,
      event_emits_enabled: config.eventEmitsLog ?? false,
    };

    await invoke('update_logging_config', { config: backendConfig });
  } catch (error) {
    console.error('Failed to update backend logging config:', error);
  }
};

// Helper function to check if logging is enabled for a category
const isLoggingEnabled = (category: keyof LoggingState): boolean => {
  return get(loggingState)[category] ?? false;
};

// Logging utility functions with consistent styling and emojis
export const logger = {
  groups: {
    info: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('groupsLog')) {
        console.log(
          `%c�️ Groups %c${message}`,
          'background: #2196F3; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #2196F3; font-weight: normal;',
          ...args
        );
      }
    },
    success: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('groupsLog')) {
        console.log(
          `%c✅ Groups %c${message}`,
          'background: #4CAF50; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #4CAF50; font-weight: normal;',
          ...args
        );
      }
    },
    warning: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('groupsLog')) {
        console.warn(
          `%c⚠️ Groups %c${message}`,
          'background: #FF9800; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #FF9800; font-weight: normal;',
          ...args
        );
      }
    },
    error: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('groupsLog')) {
        console.error(
          `%c❌ Groups %c${message}`,
          'background: #f44336; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #f44336; font-weight: normal;',
          ...args
        );
      }
    },
    cache: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('groupsLog')) {
        console.log(
          `%c💾 Groups-Cache %c${message}`,
          'background: #607D8B; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #607D8B; font-weight: normal;',
          ...args
        );
      }
    },
    eval: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('groupsLog')) {
        console.log(
          `%c🧮 Groups-Eval %c${message}`,
          'background: #9C27B0; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #9C27B0; font-weight: normal;',
          ...args
        );
      }
    },
  },
  selection: {
    info: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('selectionLog')) {
        console.log(
          `%c🎯 Selection %c${message}`,
          'background: #E91E63; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #E91E63; font-weight: normal;',
          ...args
        );
      }
    },
    action: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('selectionLog')) {
        console.log(
          `%c✨ Selection-Action %c${message}`,
          'background: #FF5722; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #FF5722; font-weight: normal;',
          ...args
        );
      }
    },
    change: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('selectionLog')) {
        console.log(
          `%c🔄 Selection-Change %c${message}`,
          'background: #795548; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #795548; font-weight: normal;',
          ...args
        );
      }
    },
    clear: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('selectionLog')) {
        console.log(
          `%c🧹 Selection-Clear %c${message}`,
          'background: #9E9E9E; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #9E9E9E; font-weight: normal;',
          ...args
        );
      }
    },
    error: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('selectionLog')) {
        console.error(
          `%c❌ Selection %c${message}`,
          'background: #f44336; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #f44336; font-weight: normal;',
          ...args
        );
      }
    },
  },
  dragdrop: {
    info: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('dragdropLog')) {
        console.log(
          `%c🎯 DragDrop %c${message}`,
          'background: #4CAF50; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #4CAF50; font-weight: normal;',
          ...args
        );
      }
    },
    start: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('dragdropLog')) {
        console.log(
          `%c▶️ DragDrop-Start %c${message}`,
          'background: #8BC34A; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #8BC34A; font-weight: normal;',
          ...args
        );
      }
    },
    move: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('dragdropLog')) {
        console.log(
          `%c🔄 DragDrop-Move %c${message}`,
          'background: #CDDC39; color: black; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #689F38; font-weight: normal;',
          ...args
        );
      }
    },
    end: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('dragdropLog')) {
        console.log(
          `%c🏁 DragDrop-End %c${message}`,
          'background: #4CAF50; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #4CAF50; font-weight: normal;',
          ...args
        );
      }
    },
    reorder: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('dragdropLog')) {
        console.log(
          `%c📋 DragDrop-Reorder %c${message}`,
          'background: #388E3C; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #388E3C; font-weight: normal;',
          ...args
        );
      }
    },
    error: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('dragdropLog')) {
        console.error(
          `%c❌ DragDrop %c${message}`,
          'background: #f44336; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #f44336; font-weight: normal;',
          ...args
        );
      }
    },
  },
  dragStore: {
    info: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('dragStoreLog')) {
        console.log(
          `%c💾 DragStore %c${message}`,
          'background: #607D8B; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #607D8B; font-weight: normal;',
          ...args
        );
      }
    },
    set: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('dragStoreLog')) {
        console.log(
          `%c✏️ DragStore-Set %c${message}`,
          'background: #2196F3; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #2196F3; font-weight: normal;',
          ...args
        );
      }
    },
    clear: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('dragStoreLog')) {
        console.log(
          `%c🧹 DragStore-Clear %c${message}`,
          'background: #9E9E9E; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #9E9E9E; font-weight: normal;',
          ...args
        );
      }
    },
    dragItem: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('dragStoreLog')) {
        console.log(
          `%c🎁 DragStore-Item %c${message}`,
          'background: #FF5722; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #FF5722; font-weight: normal;',
          ...args
        );
      }
    },
    overTarget: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('dragStoreLog')) {
        console.log(
          `%c🎯 DragStore-OverTarget %c${message}`,
          'background: #FF9800; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #FF9800; font-weight: normal;',
          ...args
        );
      }
    },
    state: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('dragStoreLog')) {
        console.log(
          `%c📊 DragStore-State %c${message}`,
          'background: #3F51B5; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #3F51B5; font-weight: normal;',
          ...args
        );
      }
    },
    error: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('dragStoreLog')) {
        console.error(
          `%c❌ DragStore %c${message}`,
          'background: #f44336; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #f44336; font-weight: normal;',
          ...args
        );
      }
    },
  },
  operations: {
    info: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('operationsLog')) {
        console.log(
          `%c⚙️ Operations %c${message}`,
          'background: #FF6B35; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #FF6B35; font-weight: normal;',
          ...args
        );
      }
    },
    success: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('operationsLog')) {
        console.log(
          `%c✅ Operations %c${message}`,
          'background: #4CAF50; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #4CAF50; font-weight: normal;',
          ...args
        );
      }
    },
    warning: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('operationsLog')) {
        console.warn(
          `%c⚠️ Operations %c${message}`,
          'background: #FF9800; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #FF9800; font-weight: normal;',
          ...args
        );
      }
    },
    error: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('operationsLog')) {
        console.error(
          `%c❌ Operations %c${message}`,
          'background: #f44336; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #f44336; font-weight: normal;',
          ...args
        );
      }
    },
    execute: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('operationsLog')) {
        console.log(
          `%c▶️ Operations-Execute %c${message}`,
          'background: #673AB7; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #673AB7; font-weight: normal;',
          ...args
        );
      }
    },
    cache: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('operationsLog')) {
        console.log(
          `%c💾 Operations-Cache %c${message}`,
          'background: #607D8B; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #607D8B; font-weight: normal;',
          ...args
        );
      }
    },
  },
  waveform: {
    info: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('waveformLog')) {
        console.log(
          `%c🎵 Waveform %c${message}`,
          'background: #9C27B0; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #9C27B0; font-weight: normal;',
          ...args
        );
      }
    },
    success: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('waveformLog')) {
        console.log(
          `%c✅ Waveform %c${message}`,
          'background: #4CAF50; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #4CAF50; font-weight: normal;',
          ...args
        );
      }
    },
    cache: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('waveformLog')) {
        console.log(
          `%c💾 Waveform-Cache %c${message}`,
          'background: #607D8B; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #607D8B; font-weight: normal;',
          ...args
        );
      }
    },
    fetch: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('waveformLog')) {
        console.log(
          `%c🔄 Waveform-Fetch %c${message}`,
          'background: #FF5722; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #FF5722; font-weight: normal;',
          ...args
        );
      }
    },
    batch: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('waveformLog')) {
        console.log(
          `%c📦 Waveform-Batch %c${message}`,
          'background: #3F51B5; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #3F51B5; font-weight: normal;',
          ...args
        );
      }
    },
    operation: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('waveformLog')) {
        console.log(
          `%c⚙️ Waveform-Operation %c${message}`,
          'background: #FF6B35; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #FF6B35; font-weight: normal;',
          ...args
        );
      }
    },
    warning: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('waveformLog')) {
        console.warn(
          `%c⚠️ Waveform %c${message}`,
          'background: #FF9800; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #FF9800; font-weight: normal;',
          ...args
        );
      }
    },
    error: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('waveformLog')) {
        console.error(
          `%c❌ Waveform %c${message}`,
          'background: #f44336; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #f44336; font-weight: normal;',
          ...args
        );
      }
    },
  },
  opPlayback: {
    info: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('opPlaybackLog')) {
        console.log(
          `%c🎬 OpPlayback %c${message}`,
          'background: #00BCD4; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #00BCD4; font-weight: normal;',
          ...args
        );
      }
    },
    success: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('opPlaybackLog')) {
        console.log(
          `%c✅ OpPlayback %c${message}`,
          'background: #4CAF50; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #4CAF50; font-weight: normal;',
          ...args
        );
      }
    },
    warning: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('opPlaybackLog')) {
        console.warn(
          `%c⚠️ OpPlayback %c${message}`,
          'background: #FF9800; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #FF9800; font-weight: normal;',
          ...args
        );
      }
    },
    error: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('opPlaybackLog')) {
        console.error(
          `%c❌ OpPlayback %c${message}`,
          'background: #f44336; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #f44336; font-weight: normal;',
          ...args
        );
      }
    },
    graph: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('opPlaybackLog')) {
        console.log(
          `%c📊 OpPlayback-Graph %c${message}`,
          'background: #3F51B5; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #3F51B5; font-weight: normal;',
          ...args
        );
      }
    },
    seek: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('opPlaybackLog')) {
        console.log(
          `%c⏩ OpPlayback-Seek %c${message}`,
          'background: #FF5722; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #FF5722; font-weight: normal;',
          ...args
        );
      }
    },
  },
  d3timelinemanager: {
    info: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('d3timelinemanagerLog')) {
        console.log(
          `%c📊 D3TimelineManager %c${message}`,
          'background: #3F51B5; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #3F51B5; font-weight: normal;',
          ...args
        );
      }
    },
    success: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('d3timelinemanagerLog')) {
        console.log(
          `%c✅ D3TimelineManager %c${message}`,
          'background: #4CAF50; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #4CAF50; font-weight: normal;',
          ...args
        );
      }
    },
    warning: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('d3timelinemanagerLog')) {
        console.warn(
          `%c⚠️ D3TimelineManager %c${message}`,
          'background: #FF9800; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #FF9800; font-weight: normal;',
          ...args
        );
      }
    },
    error: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('d3timelinemanagerLog')) {
        console.error(
          `%c❌ D3TimelineManager %c${message}`,
          'background: #f44336; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #f44336; font-weight: normal;',
          ...args
        );
      }
    },
    zoom: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('d3timelinemanagerLog')) {
        console.log(
          `%c🔍 D3TimelineManager-Zoom %c${message}`,
          'background: #2196F3; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #2196F3; font-weight: normal;',
          ...args
        );
      }
    },
    axis: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('d3timelinemanagerLog')) {
        console.log(
          `%c📏 D3TimelineManager-Axis %c${message}`,
          'background: #607D8B; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #607D8B; font-weight: normal;',
          ...args
        );
      }
    },
    transform: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('d3timelinemanagerLog')) {
        console.log(
          `%c🔄 D3TimelineManager-Transform %c${message}`,
          'background: #9C27B0; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #9C27B0; font-weight: normal;',
          ...args
        );
      }
    },
    click: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('d3timelinemanagerLog')) {
        console.log(
          `%c👆 D3TimelineManager-Click %c${message}`,
          'background: #FF5722; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #FF5722; font-weight: normal;',
          ...args
        );
      }
    },
    segment: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('d3timelinemanagerLog')) {
        console.log(
          `%c📊 D3TimelineManager-Segment %c${message}`,
          'background: #795548; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #795548; font-weight: normal;',
          ...args
        );
      }
    },
    playhead: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('d3timelinemanagerLog')) {
        console.log(
          `%c▶️ D3TimelineManager-Playhead %c${message}`,
          'background: #E91E63; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #E91E63; font-weight: normal;',
          ...args
        );
      }
    },
    initialization: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('d3timelinemanagerLog')) {
        console.log(
          `%c🚀 D3TimelineManager-Init %c${message}`,
          'background: #00BCD4; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #00BCD4; font-weight: normal;',
          ...args
        );
      }
    },
    scale: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('d3timelinemanagerLog')) {
        console.log(
          `%c📐 D3TimelineManager-Scale %c${message}`,
          'background: #CDDC39; color: black; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #689F38; font-weight: normal;',
          ...args
        );
      }
    },
  },
  timeline: {
    info: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('timelineLog')) {
        console.log(
          `%c⏰ Timeline %c${message}`,
          'background: #3F51B5; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #3F51B5; font-weight: normal;',
          ...args
        );
      }
    },
    success: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('timelineLog')) {
        console.log(
          `%c✅ Timeline %c${message}`,
          'background: #4CAF50; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #4CAF50; font-weight: normal;',
          ...args
        );
      }
    },
    warning: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('timelineLog')) {
        console.warn(
          `%c⚠️ Timeline %c${message}`,
          'background: #FF9800; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #FF9800; font-weight: normal;',
          ...args
        );
      }
    },
    error: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('timelineLog')) {
        console.error(
          `%c❌ Timeline %c${message}`,
          'background: #f44336; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #f44336; font-weight: normal;',
          ...args
        );
      }
    },
    zoom: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('timelineLog')) {
        console.log(
          `%c🔍 Timeline-Zoom %c${message}`,
          'background: #2196F3; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #2196F3; font-weight: normal;',
          ...args
        );
      }
    },
    axis: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('timelineLog')) {
        console.log(
          `%c📏 Timeline-Axis %c${message}`,
          'background: #607D8B; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #607D8B; font-weight: normal;',
          ...args
        );
      }
    },
    transform: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('timelineLog')) {
        console.log(
          `%c🔄 Timeline-Transform %c${message}`,
          'background: #9C27B0; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #9C27B0; font-weight: normal;',
          ...args
        );
      }
    },
    click: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('timelineLog')) {
        console.log(
          `%c👆 Timeline-Click %c${message}`,
          'background: #FF5722; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #FF5722; font-weight: normal;',
          ...args
        );
      }
    },
    segment: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('timelineLog')) {
        console.log(
          `%c📊 Timeline-Segment %c${message}`,
          'background: #795548; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #795548; font-weight: normal;',
          ...args
        );
      }
    },
    playhead: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('timelineLog')) {
        console.log(
          `%c▶️ Timeline-Playhead %c${message}`,
          'background: #E91E63; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #E91E63; font-weight: normal;',
          ...args
        );
      }
    },
    initialization: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('timelineLog')) {
        console.log(
          `%c🚀 Timeline-Init %c${message}`,
          'background: #00BCD4; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #00BCD4; font-weight: normal;',
          ...args
        );
      }
    },
    scale: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('timelineLog')) {
        console.log(
          `%c📐 Timeline-Scale %c${message}`,
          'background: #CDDC39; color: black; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #689F38; font-weight: normal;',
          ...args
        );
      }
    },
  },
  listeners: {
    info: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('listenersLog')) {
        console.log(
          `%c👂 Listeners %c${message}`,
          'background: #00BCD4; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #00BCD4; font-weight: normal;',
          ...args
        );
      }
    },
    attach: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('listenersLog')) {
        console.log(
          `%c🔗 Listeners-Attach %c${message}`,
          'background: #4CAF50; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #4CAF50; font-weight: normal;',
          ...args
        );
      }
    },
    detach: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('listenersLog')) {
        console.log(
          `%c🔓 Listeners-Detach %c${message}`,
          'background: #FF5722; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #FF5722; font-weight: normal;',
          ...args
        );
      }
    },
    event: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('listenersLog')) {
        console.log(
          `%c⚡ Listeners-Event %c${message}`,
          'background: #9C27B0; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #9C27B0; font-weight: normal;',
          ...args
        );
      }
    },
    error: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('listenersLog')) {
        console.error(
          `%c❌ Listeners %c${message}`,
          'background: #f44336; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
          'color: #f44336; font-weight: normal;',
          ...args
        );
      }
    },
  },
  // Template for future logging categories
  // performance: {
  //   info: (message: string, ...args: any[]) => {
  //     if (isLoggingEnabled('performanceLog')) {
  //       console.log(
  //         `%c⚡ Performance %c${message}`,
  //         'background: #FFC107; color: black; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
  //         'color: #F57C00; font-weight: normal;',
  //         ...args
  //       );
  //     }
  //   },
  //   timing: (message: string, ...args: any[]) => {
  //     if (isLoggingEnabled('performanceLog')) {
  //       console.log(
  //         `%c⏱️ Performance-Timing %c${message}`,
  //         'background: #FF8F00; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;',
  //         'color: #FF8F00; font-weight: normal;',
  //         ...args
  //       );
  //     }
  //   },
  // },
};
