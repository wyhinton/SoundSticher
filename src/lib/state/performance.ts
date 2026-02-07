// src/lib/stores/appState.ts
import { Channel, invoke } from '@tauri-apps/api/core';
import { persisted } from 'svelte-persisted-store';
import { get } from 'svelte/store';
import { type Section, callSiteTrackingEnabled } from './state.svelte';
import type { ExportAudioEvent } from './events';
import { exportState, type ExportSettings } from './export';

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
  clear_artifact_registry: PerformanceMetric[];
  clear_artifact_registry_debug: PerformanceMetric[];
  clear_duration_cache: PerformanceMetric[];
  clear_sample_cache: PerformanceMetric[];
  clear_waveform_cache: PerformanceMetric[];
  combine_all_cached_samples: PerformanceMetric[];
  combine_all_cached_samples_with_custom_order: PerformanceMetric[];
  count_audio_files_in_folders: PerformanceMetric[];
  export_audio: PerformanceMetric[];
  get_app_state: PerformanceMetric[];
  get_artifact_debug_info: PerformanceMetric[];
  get_artifact_details_debug: PerformanceMetric[];
  get_artifact_registry_records: PerformanceMetric[];
  get_artifact_registry_stats: PerformanceMetric[];
  get_artifacts_by_operation: PerformanceMetric[];
  get_audio_file_active_status: PerformanceMetric[];
  get_custom_order: PerformanceMetric[];
  get_duration: PerformanceMetric[];
  get_duration_cache_stats: PerformanceMetric[];
  get_durations_batch: PerformanceMetric[];
  get_filtered_artifacts: PerformanceMetric[];
  get_metadata: PerformanceMetric[];
  get_op_playback_state: PerformanceMetric[];
  get_sample_cache_stats: PerformanceMetric[];
  get_waveform: PerformanceMetric[];
  get_waveform_cache_stats: PerformanceMetric[];
  get_waveforms_batch: PerformanceMetric[];
  get_waveforms_for_operation: PerformanceMetric[];
  invalidate_duration: PerformanceMetric[];
  invalidate_sample_cache: PerformanceMetric[];
  invalidate_waveform: PerformanceMetric[];
  op_playback_build_graph: PerformanceMetric[];
  op_playback_build_graph_legacy: PerformanceMetric[];
  op_playback_clear_all_timelines: PerformanceMetric[];
  op_playback_clear_timeline: PerformanceMetric[];
  op_playback_get_progress: PerformanceMetric[];
  op_playback_pause: PerformanceMetric[];
  op_playback_play: PerformanceMetric[];
  op_playback_resume: PerformanceMetric[];
  op_playback_seek: PerformanceMetric[];
  op_playback_set_loop: PerformanceMetric[];
  op_playback_set_volume: PerformanceMetric[];
  op_playback_stop: PerformanceMetric[];
  pause_sample_preview: PerformanceMetric[];
  play_sample_preview: PerformanceMetric[];
  refresh_artifact_registry_status: PerformanceMetric[];
  refresh_artifacts_existence: PerformanceMetric[];
  remove_artifacts_by_operation_debug: PerformanceMetric[];
  render_all_auto_operations: PerformanceMetric[];
  set_audio_file_active: PerformanceMetric[];
  set_audio_files_active_batch: PerformanceMetric[];
  test_async: PerformanceMetric[];
  test_render_single_operation: PerformanceMetric[];
  test_scheduler: PerformanceMetric[];
  timeline_build_from_request: PerformanceMetric[];
  timeline_build_playback: PerformanceMetric[];
  timeline_clear: PerformanceMetric[];
  timeline_clear_all: PerformanceMetric[];
  timeline_get_progress: PerformanceMetric[];
  timeline_pause: PerformanceMetric[];
  timeline_play: PerformanceMetric[];
  timeline_resume: PerformanceMetric[];
  timeline_seek: PerformanceMetric[];
  timeline_set_loop: PerformanceMetric[];
  timeline_set_volume: PerformanceMetric[];
  timeline_stop: PerformanceMetric[];
  toggle_audio_file_active: PerformanceMetric[];
  update_sorting: PerformanceMetric[];
}

export const performanceStore = persisted<PerformanceState>('performanceState', {
  cancel_combine: [],
  clear_artifact_registry: [],
  clear_artifact_registry_debug: [],
  clear_duration_cache: [],
  clear_sample_cache: [],
  clear_waveform_cache: [],
  combine_all_cached_samples: [],
  combine_all_cached_samples_with_custom_order: [],
  count_audio_files_in_folders: [],
  export_audio: [],
  get_app_state: [],
  get_artifact_debug_info: [],
  get_artifact_details_debug: [],
  get_artifact_registry_records: [],
  get_artifact_registry_stats: [],
  get_artifacts_by_operation: [],
  get_audio_file_active_status: [],
  get_custom_order: [],
  get_duration: [],
  get_duration_cache_stats: [],
  get_durations_batch: [],
  get_filtered_artifacts: [],
  get_metadata: [],
  get_op_playback_state: [],
  get_sample_cache_stats: [],
  get_waveform: [],
  get_waveform_cache_stats: [],
  get_waveforms_batch: [],
  get_waveforms_for_operation: [],
  invalidate_duration: [],
  invalidate_sample_cache: [],
  invalidate_waveform: [],
  op_playback_build_graph: [],
  op_playback_build_graph_legacy: [],
  op_playback_clear_all_timelines: [],
  op_playback_clear_timeline: [],
  op_playback_get_progress: [],
  op_playback_pause: [],
  op_playback_play: [],
  op_playback_resume: [],
  op_playback_seek: [],
  op_playback_set_loop: [],
  op_playback_set_volume: [],
  op_playback_stop: [],
  pause_sample_preview: [],
  play_sample_preview: [],
  refresh_artifact_registry_status: [],
  refresh_artifacts_existence: [],
  remove_artifacts_by_operation_debug: [],
  render_all_auto_operations: [],
  set_audio_file_active: [],
  set_audio_files_active_batch: [],
  test_async: [],
  test_render_single_operation: [],
  test_scheduler: [],
  timeline_build_from_request: [],
  timeline_build_playback: [],
  timeline_clear: [],
  timeline_clear_all: [],
  timeline_get_progress: [],
  timeline_pause: [],
  timeline_play: [],
  timeline_resume: [],
  timeline_seek: [],
  timeline_set_loop: [],
  timeline_set_volume: [],
  timeline_stop: [],
  toggle_audio_file_active: [],
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
      // invokeWithPerf('open_in_explorer', {
      //   fileToOpen: message.data.outputPath,
      // });
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
