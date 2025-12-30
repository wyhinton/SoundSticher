// stores/settings.ts
import { persisted } from 'svelte-persisted-store';
import type { Writable } from 'svelte/store';

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
};

export type ExportState = {
  settings?: ExportSettings;
  progress: number;
  message?: string;
  error?: string;
  outputPath?: string;
};

export type EstimatedFileSize = {
  bytes: number;
  formatted: string;
  breakdown: {
    duration: number;
    sampleRate: number;
    channels: number;
    bitDepth: number;
    samplesPerSecond: number;
    totalSamples: number;
    bytesPerSample: number;
    rawAudioBytes: number;
    formatOverhead: number;
  };
};

export const exportState: Writable<ExportState> = persisted<ExportState>('exportSettings', {
  settings: {
    sampleRate: 44100,
    bitDepth: 16,
    channels: 2,
    format: 'wav',
    filename: 'exported_audio',
    bitrate: undefined,
  },
  progress: 0,
  message: undefined,
  error: undefined,
});

// Utility function to format file size
export function formatFileSize(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

// Utility function to calculate estimated file size
export function calculateEstimatedFileSize(
  settings: ExportSettings | undefined,
  durationSeconds: number
): EstimatedFileSize {
  if (!settings || durationSeconds === 0) {
    return {
      bytes: 0,
      formatted: '0 B',
      breakdown: {
        duration: 0,
        sampleRate: 0,
        channels: 0,
        bitDepth: 0,
        samplesPerSecond: 0,
        totalSamples: 0,
        bytesPerSample: 0,
        rawAudioBytes: 0,
        formatOverhead: 0,
      },
    };
  }

  console.log(durationSeconds);
  const { sampleRate, bitDepth, channels, format } = settings;

  // Calculate raw audio data size
  const samplesPerSecond = sampleRate * channels;
  const totalSamples = samplesPerSecond * durationSeconds;
  const bytesPerSample = bitDepth / 8;
  const rawAudioBytes = totalSamples * bytesPerSample;

  // Add format overhead/compression estimates
  let estimatedBytes = rawAudioBytes;

  switch (format.toLowerCase()) {
    case 'wav':
      // WAV has minimal header overhead (~44 bytes)
      estimatedBytes = rawAudioBytes + 44;
      break;
    case 'flac':
      // FLAC typically achieves 50-70% compression
      estimatedBytes = rawAudioBytes * 0.6 + 1024; // +1KB for metadata
      break;
    case 'mp3':
      // MP3 size depends on bitrate
      if (settings.bitrate) {
        // Convert kbps to bytes per second, then multiply by duration
        estimatedBytes = ((settings.bitrate * 1000) / 8) * durationSeconds;
      } else {
        // Default to ~128kbps equivalent
        estimatedBytes = ((128 * 1000) / 8) * durationSeconds;
      }
      break;
    case 'ogg':
      // OGG Vorbis similar to MP3
      if (settings.bitrate) {
        estimatedBytes = ((settings.bitrate * 1000) / 8) * durationSeconds;
      } else {
        estimatedBytes = ((128 * 1000) / 8) * durationSeconds;
      }
      break;
    default:
      // Default to raw audio size
      estimatedBytes = rawAudioBytes;
  }

  return {
    bytes: Math.round(estimatedBytes),
    formatted: formatFileSize(estimatedBytes),
    breakdown: {
      duration: durationSeconds,
      sampleRate,
      channels,
      bitDepth,
      samplesPerSecond,
      totalSamples,
      bytesPerSample,
      rawAudioBytes,
      formatOverhead: estimatedBytes - rawAudioBytes,
    },
  };
}

// Format-specific high-quality defaults
export function getFormatDefaults(format: string): Partial<ExportSettings> {
  switch (format.toLowerCase()) {
    case 'wav':
      return {
        sampleRate: 48000, // Professional quality
        bitDepth: 24, // High bit depth for dynamic range
        channels: 2, // Stereo
        bitrate: undefined, // Not applicable for WAV
      };
    case 'flac':
      return {
        sampleRate: 96000, // Very high quality for lossless
        bitDepth: 24, // Maximum supported by most systems
        channels: 2, // Stereo
        bitrate: undefined, // Not applicable for FLAC (lossless)
      };
    case 'mp3':
      return {
        sampleRate: 48000, // High quality sample rate
        bitDepth: 16, // Standard for lossy formats
        channels: 2, // Stereo
        bitrate: 320, // Near-transparent quality
      };
    case 'ogg':
      return {
        sampleRate: 48000, // High quality sample rate
        bitDepth: 16, // Standard for lossy formats
        channels: 2, // Stereo
        bitrate: 256, // High quality for Vorbis
      };
    default:
      return {
        sampleRate: 44100,
        bitDepth: 16,
        channels: 2,
        bitrate: undefined,
      };
  }
}

// Apply format defaults while preserving user choices for compatible settings
export function applyFormatDefaults(
  currentSettings: ExportSettings,
  newFormat: string
): ExportSettings {
  const defaults = getFormatDefaults(newFormat);
  const newSettings = {
    ...currentSettings,
    format: newFormat,
    ...defaults,
  };

  // Preserve filename but update extension if it has one
  if (currentSettings.filename) {
    const nameParts = currentSettings.filename.split('.');
    if (nameParts.length > 1) {
      // Replace extension
      newSettings.filename = nameParts.slice(0, -1).join('.') + '.' + newFormat;
    } else {
      // Add extension
      newSettings.filename = currentSettings.filename + '.' + newFormat;
    }
  }

  return newSettings;
}
