// src/lib/stores/appState.ts
import { Channel, invoke } from '@tauri-apps/api/core';
import { persisted } from 'svelte-persisted-store';
import { get, writable } from 'svelte/store';
import {
  appState,
  getAllFiles,
  type CombineAudioResult,
  type Section,
  type SectionSend,
  bumpRevision,
  callSiteTrackingEnabled,
} from './state.svelte';
import type { BufferAudioEvent, CombineAudioEvent, ExportAudioEvent } from './events';
import { exportState, type ExportSettings, type ExportState } from './export';
import { createTypedEventChannel } from '$lib/utils/channelMaker';

export interface PerformanceMetric {
  time: number;
  timeStamp: number;
  callSite?: string;
  fileName?: string;
  lineNumber?: number;
  columnNumber?: number;
}

type PerfMetricName = keyof PerformanceState;

// Helper function to get call site information
function getCallSiteInfo() {
  const stack = new Error().stack;
  if (!stack) return null;

  const lines = stack.split('\n');
  // Skip the first 3 lines: Error message, getCallSiteInfo, and invokeWithPerf
  const callerLine = lines[3];

  if (!callerLine) return null;

  // Parse the stack trace line to extract file info
  const match = callerLine.match(/at\s+(.+?)\s+\((.+):(\d+):(\d+)\)/);
  if (match) {
    const [, functionName, fileName, lineNumber, columnNumber] = match;
    if (fileName && lineNumber && columnNumber) {
      return {
        callSite: `${functionName} (${fileName.split(/[\\\/]/).pop()}:${lineNumber})`,
        fileName: fileName.split(/[\\\/]/).pop(),
        lineNumber: parseInt(lineNumber),
        columnNumber: parseInt(columnNumber),
      };
    }
  }

  // Fallback for different stack trace formats
  const simpleMatch = callerLine.match(/(\w+\.(?:ts|js|svelte)):(\d+):(\d+)/);
  if (simpleMatch) {
    const [, fileName, lineNumber, columnNumber] = simpleMatch;
    if (fileName && lineNumber && columnNumber) {
      return {
        callSite: `${fileName}:${lineNumber}`,
        fileName,
        lineNumber: parseInt(lineNumber),
        columnNumber: parseInt(columnNumber),
      };
    }
  }

  return { callSite: callerLine.trim() };
}

export interface PerformanceState {
  cancel_combine: PerformanceMetric[];
  clear_audio_files: [];
  clear_waveform_cache: PerformanceMetric[];
  combine_all_cached_samples: PerformanceMetric[];
  combine_all_cached_samples_with_custom_order: PerformanceMetric[];
  export_audio: PerformanceMetric[];
  get_app_state: PerformanceMetric[];
  get_audio_file_active_status: PerformanceMetric[];
  get_current_play_progress: PerformanceMetric[];
  get_custom_order: PerformanceMetric[];
  get_file_paths_in_folder: PerformanceMetric[];
  get_metadata: PerformanceMetric[];
  get_waveform: PerformanceMetric[];
  get_waveform_cache_stats: PerformanceMetric[];
  get_waveforms_batch: PerformanceMetric[];
  get_waveforms_for_operation: PerformanceMetric[];
  invalidate_waveform: PerformanceMetric[];
  op_playback_build_graph: PerformanceMetric[];
  op_playback_clear_graph: PerformanceMetric[];
  op_playback_get_progress: PerformanceMetric[];
  op_playback_pause: PerformanceMetric[];
  op_playback_play: PerformanceMetric[];
  op_playback_resume: PerformanceMetric[];
  op_playback_seek: PerformanceMetric[];
  op_playback_set_loop: PerformanceMetric[];
  op_playback_set_volume: PerformanceMetric[];
  op_playback_stop: PerformanceMetric[];
  open_in_explorer: PerformanceMetric[];
  pause_sample_preview: PerformanceMetric[];
  pause_timeline_audio: PerformanceMetric[];
  play_sample_preview: PerformanceMetric[];
  play_timeline_audio: PerformanceMetric[];
  set_audio_file_active: PerformanceMetric[];
  set_audio_files_active_batch: PerformanceMetric[];
  set_timeline_loop_enabled: PerformanceMetric[];
  set_timeline_play_position: PerformanceMetric[];
  set_volume: PerformanceMetric[];
  stop_timeline_audio: PerformanceMetric[];
  test_async: PerformanceMetric[];
  test_operation: PerformanceMetric[];
  test_operation_with_params: PerformanceMetric[];
  test_scheduler: PerformanceMetric[];
  toggle_audio_file_active: PerformanceMetric[];
  update_inputs: PerformanceMetric[];
  update_sorting: PerformanceMetric[];
}

export const performanceStore = persisted<PerformanceState>('performanceState', {
  cancel_combine: [],
  clear_audio_files: [],
  clear_waveform_cache: [],
  combine_all_cached_samples: [],
  combine_all_cached_samples_with_custom_order: [],
  export_audio: [],
  get_app_state: [],
  get_audio_file_active_status: [],
  get_current_play_progress: [],
  get_custom_order: [],
  get_file_paths_in_folder: [],
  get_metadata: [],
  get_waveform: [],
  get_waveform_cache_stats: [],
  get_waveforms_batch: [],
  get_waveforms_for_operation: [],
  invalidate_waveform: [],
  op_playback_build_graph: [],
  op_playback_clear_graph: [],
  op_playback_get_progress: [],
  op_playback_pause: [],
  op_playback_play: [],
  op_playback_resume: [],
  op_playback_seek: [],
  op_playback_set_loop: [],
  op_playback_set_volume: [],
  op_playback_stop: [],
  open_in_explorer: [],
  pause_sample_preview: [],
  pause_timeline_audio: [],
  play_sample_preview: [],
  play_timeline_audio: [],
  set_audio_file_active: [],
  set_audio_files_active_batch: [],
  set_timeline_loop_enabled: [],
  set_timeline_play_position: [],
  set_volume: [],
  stop_timeline_audio: [],
  test_async: [],
  test_operation: [],
  test_operation_with_params: [],
  test_scheduler: [],
  toggle_audio_file_active: [],
  update_inputs: [],
  update_sorting: [],
});

export const setPerfMetric = (metric: PerfMetricName, time: number, callSiteInfo?: any) => {
  performanceStore.update(store => {
    const previous = store[metric] ?? [];

    const updatedMetric = [
      ...previous,
      {
        time,
        timeStamp: Date.now(),
        ...callSiteInfo,
      },
    ].slice(-100); // Keep only the last 100 entries

    return {
      ...store,
      [metric]: updatedMetric,
    };
  });
};

export const resetPerformance = () => {
  performanceStore.update(store => {
    const cleared = Object.keys(store).reduce((acc, key) => {
      acc[key as PerfMetricName] = [];
      return acc;
    }, {} as PerformanceState);
    console.log(cleared);
    return cleared;
  });
  console.log(get(performanceStore));
};

type CommandError = {
  kind: string;
  message: string;
};

export type Result<T, E> = { ok: true; value: T } | { ok: false; error: E };

export async function invokeWithPerf<T = string, E = CommandError>(
  command: PerfMetricName,
  args?: Record<string, any>
): Promise<Result<T, E>> {
  const start = performance.now();
  const callSiteInfo = get(callSiteTrackingEnabled) ? getCallSiteInfo() : null;

  try {
    const result = await invoke<T>(command, args);
    const end = performance.now();
    setPerfMetric(command, end - start, callSiteInfo);
    return { ok: true, value: result };
  } catch (err: unknown) {
    const end = performance.now();
    setPerfMetric(command, end - start, callSiteInfo);

    return { ok: false, error: err as E };
  }
}

export async function updateInputs(sections: Section[]) {
  // const newSends: SectionSend[] = sections.map(s => ({
  //   folderPath: s.folderPath,
  //   paths: s.files.map(f => ({ path: f.path })),
  // }));
  // const onCombineAudioEvent = createTypedEventChannel<CombineAudioEvent>({
  //   onStarted: data => {
  //     appState.update(state => {
  //       state.isCombiningFile = true;
  //       state.combinedFileLength = data.duration;
  //       state.timelineItems = [];
  //       return state;
  //     });
  //   },
  //   onProgress: data => {
  //     appState.update(s => {
  //       const curwaveform = document.getElementById('waveform-path')?.getAttribute('d');
  //       s.combinedFile = { svgPath: data.svgPath };
  //       if (curwaveform) {
  //         s.combinedFile.svgPath = curwaveform + data.svgPath;
  //       }
  //       let timelineItemToUpdate = s.timelineItems.find(clip => clip.id == data.id);
  //       if (!timelineItemToUpdate) {
  //         s.timelineItems.push({ type: 'audio-file', ...data });
  //       } else {
  //         timelineItemToUpdate = { type: 'audio-file', ...data };
  //       }
  //       const toGiveId = getAllFiles(s.sections).find(f => f.path === data.fileName);
  //       if (toGiveId) {
  //         toGiveId.id = data.id;
  //       }
  //       s.sections = s.sections;
  //       return s;
  //     });
  //   },
  //   onFinished: data => {
  //     appState.update(s => {
  //       s.isCombiningFile = false;
  //       s.combinedFile = { svgPath: data.svgPath };
  //       s.hasNoActiveSamples = data.empty;
  //       console.log(data.empty);
  //       return s;
  //     });
  //   },
  // });
  // const onBufferAudioEvent = new Channel<BufferAudioEvent>();
  // onBufferAudioEvent.onmessage = message => {
  //   if (message.event === 'finished') {
  //     console.log(`%cHERE LINE :166 %c`, 'color: brown; font-weight: bold', '');
  //     invokeWithPerf<CombineAudioResult>('combine_all_cached_samples_with_custom_order', {
  //       onEvent: onCombineAudioEvent,
  //     });
  //   }
  // };
  // const updateInputsResult = await invokeWithPerf('update_inputs', {
  //   sections: newSends,
  //   onEvent: onBufferAudioEvent,
  // });
  // // Bump the content revision after updateInputs completes
  // if (updateInputsResult.ok) {
  //   bumpRevision();
  // }
  // return updateInputsResult;
}

export async function exportAudio(settings: ExportSettings, outputPath: string) {
  const onExportAudioEvent = new Channel<ExportAudioEvent>();

  onExportAudioEvent.onmessage = message => {
    console.log(message);
    if (message.event === 'started') {
      console.log('STARTED ENCODE');
    }
    if (message.event === 'progress') {
      exportState.update(s => {
        s.progress = message.data.progress;
        s.message = message.data.message;
        return s;
      });
      console.log(message);
    }
    if (message.event === 'finished') {
      console.log('FINISHED ENCODE');
      invokeWithPerf('open_in_explorer', {
        fileToOpen: message.data.outputPath,
      });
    }
  };
  const res = await invokeWithPerf<string, CommandError>('export_audio', {
    settings: settings,
    outputFile: outputPath,
    onEvent: onExportAudioEvent,
  });
  console.log(res);
  if (res.ok === true) {
    exportState.update(s => {
      s.error = undefined;
      return s;
    });
  } else {
    exportState.update(s => {
      s.error = res.error.message;
      return s;
    });
  }
}
