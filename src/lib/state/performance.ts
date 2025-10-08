// src/lib/stores/appState.ts
import { Channel, invoke } from '@tauri-apps/api/core';
import { persisted } from 'svelte-persisted-store';
import { writable } from 'svelte/store';
import {
  appState,
  getAllFiles,
  type CombineAudioResult,
  type Section,
  type SectionSend,
} from './state.svelte';
import type { BufferAudioEvent, CombineAudioEvent, ExportAudioEvent } from './events';
import { exportState, type ExportSettings, type ExportState } from './export';
import { CLEAR_COMMAND } from 'tauri-plugin-clipboard-api';

export interface PerformanceMetric {
  time: number;
  timeStamp: number;
}

type PerfMetricName = keyof PerformanceState;

export interface PerformanceState {
  combine_audio_files: PerformanceMetric[];
  get_metadata: PerformanceMetric[];
  get_file_paths_in_folder: PerformanceMetric[];
  pause_sample_preview: PerformanceMetric[];
  play_sample_preview: PerformanceMetric[];
  update_inputs: PerformanceMetric[];
  combine_all_cached_samples: PerformanceMetric[];
  play_timeline_audio: PerformanceMetric[];
  cancel_combine: PerformanceMetric[];
  pause_timeline_audio: PerformanceMetric[];
  clear_audio_files: PerformanceMetric[];
  export_combined_audio_as_wav: PerformanceMetric[];
  get_app_state: PerformanceMetric[];
  test_async: PerformanceMetric[];
  export_audio: PerformanceMetric[];
  open_in_explorer: PerformanceMetric[];
  update_sorting: PerformanceMetric[];
  combine_all_cached_samples_with_custom_order: PerformanceMetric[];
  set_timeline_play_position: PerformanceMetric[];
}

export const performanceStore = persisted<PerformanceState>('performanceState', {
  combine_audio_files: [],
  get_metadata: [],
  get_file_paths_in_folder: [],
  pause_sample_preview: [],
  play_sample_preview: [],
  update_inputs: [],
  combine_all_cached_samples: [],
  play_timeline_audio: [],
  cancel_combine: [],
  pause_timeline_audio: [],
  clear_audio_files: [],
  export_combined_audio_as_wav: [],
  get_app_state: [],
  test_async: [],
  export_audio: [],
  open_in_explorer: [],
  update_sorting: [],
  combine_all_cached_samples_with_custom_order: [],
  set_timeline_play_position: [],
});

export const setPerfMetric = (metric: PerfMetricName, time: number) => {
  performanceStore.update(store => {
    const previous = store[metric] ?? [];

    const updatedMetric = [...previous, { time, timeStamp: Date.now() }].slice(-100); // Keep only the last 100 entries

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

    return cleared;
  });
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

  try {
    const result = await invoke<T>(command, args);
    const end = performance.now();
    setPerfMetric(command, end - start);
    return { ok: true, value: result };
  } catch (err: unknown) {
    const end = performance.now();
    setPerfMetric(command, end - start);

    return { ok: false, error: err as E };
    4;
  }
}

export async function updateInputs(sections: Section[]) {
  const newSends: SectionSend[] = sections.map(s => ({
    folderPath: s.folderPath,
    paths: s.files.map(f => ({ path: f.path })),
  }));
  const onCombineAudioEvent = new Channel<CombineAudioEvent>();

  onCombineAudioEvent.onmessage = message => {
    if (message.event === 'started') {
      appState.update(state => {
        state.isCombiningFile = true;
        state.combinedFileLength = message.data.duration;
        state.timelineItems = [];
        return state;
      });
    }
    if (message.event === 'progress') {
      appState.update(s => {
        const curwaveform = document.getElementById('waveform-path').getAttribute('d');
        s.combinedFile = { svgPath: message.data.svgPath };
        if (curwaveform) {
          s.combinedFile.svgPath = curwaveform + message.data.svgPath;
        }
        let timelineItemToUpdate = s.timelineItems.find(clip => clip.id == message.data.id);
        // console.log(message.data.id)
        if (!timelineItemToUpdate) {
          s.timelineItems.push({ type: 'audio-file', ...message.data });
        } else {
          timelineItemToUpdate = { type: 'audio-file', ...message.data };
        }
        const toGiveId = getAllFiles(s.sections).find(f => f.path === message.data.fileName);
        toGiveId.id = message.data.id;
        s.sections = s.sections;
        return s;
      });
    }
    if (message.event === 'finished') {
      appState.update(s => {
        s.isCombiningFile = false;
        s.combinedFile = { svgPath: message.data.svgPath };
        return s;
      });
    }
  };

  const onBufferAudioEvent = new Channel<BufferAudioEvent>();
  onBufferAudioEvent.onmessage = message => {
    if (message.event === 'finished') {
      console.log(`%cHERE LINE :166 %c`, 'color: brown; font-weight: bold', '');

      invokeWithPerf<CombineAudioResult>('combine_all_cached_samples_with_custom_order', {
        onEvent: onCombineAudioEvent,
      });
    }
  };

  const updateInputsResult = await invokeWithPerf('update_inputs', {
    sections: newSends,
    onEvent: onBufferAudioEvent,
  });
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
    sampleRate: settings.sampleRate,
    format: settings.format,
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
