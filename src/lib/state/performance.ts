// src/lib/stores/appState.ts
import { invoke } from '@tauri-apps/api/core';
import { persisted } from 'svelte-persisted-store';
import { writable } from 'svelte/store';


export interface PerformanceMetric{
    time: number;
    timeStamp: number;
}

type PerfMetricName = keyof PerformanceState;

export interface PerformanceState {
  combine_audio_files: PerformanceMetric[];
  get_metadata: PerformanceMetric[];
  get_file_paths_in_folder: PerformanceMetric[];
  pause_song: PerformanceMetric[];
  play_song: PerformanceMetric[];
  update_inputs: PerformanceMetric[];
  combine_all_cached_samples: PerformanceMetric[];
  play_combined_audio: PerformanceMetric[];
  cancel_combine: PerformanceMetric[];

}

export const performanceStore = persisted<PerformanceState>("performanceState",{
  combine_audio_files: [],
  get_metadata: [],
  get_file_paths_in_folder: [],
  pause_song: [],
  play_song: [],
  update_inputs: [],
  combine_all_cached_samples: [],
  play_combined_audio: [],
  cancel_combine: [],
});

export const setPerfMetric = (metric: PerfMetricName, time: number) => {
  performanceStore.update((store) => {
    const updatedMetric = [
      ...(store[metric] ?? []),
      { time, timeStamp: Date.now() }
    ];

    return {
      ...store,
      [metric]: updatedMetric
    };
  });
};
export const resetPerformance = () => {
  performanceStore.update(store => {
    const cleared = Object.keys(store).reduce((acc, key) => {
      acc[key as PerfMetricName] = [];
      return acc;
    }, {} as PerformanceState);

    return cleared;
  });
};


export async function invokeWithPerf<T>(command: PerfMetricName, args?: Record<string, any>): Promise<T> {
  const start = performance.now();
  try {
    const result = await invoke<T>(command, args);
    const end = performance.now();
    setPerfMetric(command, end - start);
    return result;
  } catch (err) {
    const end = performance.now();
    setPerfMetric(command, end - start); // Still log timing even on failure
    throw err;
  }
}
