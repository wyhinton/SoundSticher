// src/lib/stores/appState.ts
import { Channel, invoke } from '@tauri-apps/api/core';
import { persisted } from 'svelte-persisted-store';
import { writable } from 'svelte/store';
import { appState, type CombineAudioResult, type Section, type SectionSend } from './state.svelte';
import type { BufferAudioEvent, CombineAudioEvent } from './events';


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
  pause_combined_audio: PerformanceMetric[];
  clear_audio_files: PerformanceMetric[];
  export_combined_audio_as_wav: PerformanceMetric[];
  get_app_state: PerformanceMetric[];

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
  pause_combined_audio: [],
  clear_audio_files: [],
  export_combined_audio_as_wav: [],
  get_app_state: []
});

export const setPerfMetric = (metric: PerfMetricName, time: number) => {
  performanceStore.update((store) => {
    const previous = store[metric] ?? [];
    
    const updatedMetric = [
      ...previous,
      { time, timeStamp: Date.now() }
    ].slice(-100) // Keep only the last 100 entries

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


export async function updateInputs(sections: Section[] ){
  const newSends: SectionSend[] = sections.map((s) => ({
    folderPath: s.folderPath,
    paths: s.files.map((f) => ({ path: f.path })),
  }));
    const onCombineAudioEvent = new Channel<CombineAudioEvent>();
  
    onCombineAudioEvent.onmessage = (message) => {
      console.log(message);
      console.log(`got COMBINE event ${message.event}`);
  
      if (message.event === "started") {
        appState.update((state) => {
          state.isCombiningFile = true;
          state.combinedFileLength = message.data.duration;
          return state;
        });
      }
      if (message.event === "progress") {
            appState.update((s) => {
          s.combinedFile = { svgPath: message.data.svgPath };
          return s;
        });
      }
      if (message.event === "finished") {
        console.log(message);
        appState.update((s) => {
          s.isCombiningFile = false;
          s.combinedFile = { svgPath: message.data.svgPath };
          return s;
        });
        console.log(message.event);
      }
    };
    
    const onBufferAudioEvent = new Channel<BufferAudioEvent>();
    onBufferAudioEvent.onmessage = (message) => {
      console.log(message);
      console.log(`got download event ${message.event}`);
  
      if (message.event === "finished") {
        invokeWithPerf<CombineAudioResult>("combine_all_cached_samples", {
          onEvent: onCombineAudioEvent,
        });
      }
    };
  
    const updateInputsResult = await invokeWithPerf("update_inputs", {
      sections: newSends,
      onEvent: onBufferAudioEvent,
    });

}