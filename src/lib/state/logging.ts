import { persisted } from 'svelte-persisted-store';
import { get } from 'svelte/store';

export interface LoggingState {
  groupsLog: boolean;
  // Add future logging categories here
  // performanceLog?: boolean;
  // audioLog?: boolean;
  // uiLog?: boolean;
}

export const loggingState = persisted<LoggingState>('loggingState', {
  groupsLog: false,
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
        console.log(`🔷 [GROUPS] ${message}`, ...args);
      }
    },
    success: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('groupsLog')) {
        console.log(`✅ [GROUPS] ${message}`, ...args);
      }
    },
    warning: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('groupsLog')) {
        console.warn(`⚠️ [GROUPS] ${message}`, ...args);
      }
    },
    error: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('groupsLog')) {
        console.error(`❌ [GROUPS] ${message}`, ...args);
      }
    },
    cache: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('groupsLog')) {
        console.log(`💾 [GROUPS-CACHE] ${message}`, ...args);
      }
    },
    eval: (message: string, ...args: any[]) => {
      if (isLoggingEnabled('groupsLog')) {
        console.log(`🧮 [GROUPS-EVAL] ${message}`, ...args);
      }
    },
  },
  // Template for future logging categories
  // performance: {
  //   info: (message: string, ...args: any[]) => {
  //     if (isLoggingEnabled('performanceLog')) {
  //       console.log(`⚡ [PERFORMANCE] ${message}`, ...args);
  //     }
  //   },
  //   timing: (message: string, ...args: any[]) => {
  //     if (isLoggingEnabled('performanceLog')) {
  //       console.log(`⏱️ [PERFORMANCE-TIMING] ${message}`, ...args);
  //     }
  //   },
  // },
};
