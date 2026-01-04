import { persisted } from 'svelte-persisted-store';
import { get } from 'svelte/store';

export interface LoggingState {
  groupsLog: boolean;
  selectionLog: boolean;
  dragdropLog: boolean;
  operationsLog: boolean;
  // Add future logging categories here
  // performanceLog?: boolean;
  // audioLog?: boolean;
  // uiLog?: boolean;
}

export const loggingState = persisted<LoggingState>('loggingState', {
  groupsLog: false,
  selectionLog: false,
  dragdropLog: false,
  operationsLog: false,
});

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
