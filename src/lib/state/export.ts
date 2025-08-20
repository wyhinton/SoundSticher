// stores/settings.ts
import { persisted } from "svelte-persisted-store";
import type { Writable } from "svelte/store";

export type ExportSettings = {
  sampleRate: number;
  bitDepth: number;
  channels: number;
  format: string;
  filename: string;
  bitrate?: number;
};

export type ExportProgress = {
  progress: number;
}

export type ExportState = {
  settings?: ExportSettings
  progress: number;
  message?: string;
  error?: string;
}

export const exportState: Writable<ExportState> = persisted<ExportState>(
  "exportSettings",
  {
    settings: {
    sampleRate: 44100,
    bitDepth: 16,
    channels: 2,
    format: "wav",
    filename: "exported_audio",
    bitrate: undefined,
    },
    progress: 0,
    message: undefined,
    error: undefined,
  }
);
