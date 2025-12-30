# Audio Export File Size Calculation

This document explains how the `calculateEstimatedFileSize` function works and how it's integrated into the dynamic export state system.

## Overview

The `calculateEstimatedFileSize` function provides real-time file size estimates for audio exports based on user settings and audio duration. It calculates both raw audio data size and format-specific overhead/compression to give users accurate file size predictions before exporting.

## Function Signature

```typescript
export function calculateEstimatedFileSize(
  settings: ExportSettings | undefined,
  durationSeconds: number
): EstimatedFileSize
```

## How It Works

### 1. Input Validation

```typescript
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
```

The function first validates inputs and returns a zero-size result if no settings are provided or duration is zero.

### 2. Raw Audio Calculation

The function calculates the raw, uncompressed audio data size using these steps:

```typescript
const { sampleRate, bitDepth, channels, format } = settings;

// Calculate raw audio data size
const samplesPerSecond = sampleRate * channels;
const totalSamples = samplesPerSecond * durationSeconds;
const bytesPerSample = bitDepth / 8;
const rawAudioBytes = totalSamples * bytesPerSample;
```

**Formula breakdown:**
- `samplesPerSecond` = `sampleRate` × `channels` (e.g., 44100 × 2 = 88,200 samples/sec for stereo)
- `totalSamples` = `samplesPerSecond` × `duration` (total audio samples)
- `bytesPerSample` = `bitDepth` ÷ 8 (e.g., 16-bit = 2 bytes per sample)
- `rawAudioBytes` = `totalSamples` × `bytesPerSample` (uncompressed size)

### 3. Format-Specific Size Estimation

Different audio formats have different file sizes due to compression and overhead:

#### WAV Format
```typescript
case 'wav':
  // WAV has minimal header overhead (~44 bytes)
  estimatedBytes = rawAudioBytes + 44;
  break;
```
- **WAV is uncompressed** - file size = raw audio + small header
- **Overhead**: ~44 bytes for WAV header
- **Result**: Largest file size, highest quality

#### FLAC Format
```typescript
case 'flac':
  // FLAC typically achieves 50-70% compression
  estimatedBytes = rawAudioBytes * 0.6 + 1024; // +1KB for metadata
  break;
```
- **FLAC is lossless compressed** - typically 50-70% of original size
- **Compression factor**: 0.6 (60% of original size)
- **Overhead**: 1KB for metadata
- **Result**: Smaller than WAV, same quality

#### MP3 Format
```typescript
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
```
- **MP3 is lossy compressed** - size determined by bitrate, not raw audio size
- **Calculation**: `(bitrate_kbps × 1000 ÷ 8) × duration_seconds`
- **Default bitrate**: 128 kbps if not specified
- **Result**: Much smaller files, quality depends on bitrate

#### OGG Format
```typescript
case 'ogg':
  // OGG Vorbis similar to MP3
  if (settings.bitrate) {
    estimatedBytes = ((settings.bitrate * 1000) / 8) * durationSeconds;
  } else {
    estimatedBytes = ((128 * 1000) / 8) * durationSeconds;
  }
  break;
```
- **OGG Vorbis** behaves similarly to MP3
- **Same calculation** as MP3 format
- **Generally more efficient** than MP3 at same bitrates

## Return Value Structure

The function returns an `EstimatedFileSize` object with:

```typescript
{
  bytes: number;           // Total estimated file size in bytes
  formatted: string;       // Human-readable size (e.g., "3.2 MB")
  breakdown: {
    duration: number;           // Audio duration in seconds
    sampleRate: number;         // Sample rate (Hz)
    channels: number;           // Number of channels
    bitDepth: number;          // Bit depth
    samplesPerSecond: number;   // Total samples per second
    totalSamples: number;       // Total audio samples
    bytesPerSample: number;     // Bytes per sample
    rawAudioBytes: number;      // Uncompressed size
    formatOverhead: number;     // Compression/overhead difference
  };
}
```

## Dynamic Updates in the Application

### 1. Export State Integration

The function is integrated with the reactive export state system:

```typescript
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
```

### 2. Reactive Updates

In Svelte components, the file size calculation updates automatically when:

- **Export settings change** (format, sample rate, bit depth, channels, bitrate)
- **Audio duration changes** (when files are loaded/combined)
- **Any export state property changes**

Example reactive usage:
```typescript
$: estimatedSize = calculateEstimatedFileSize($exportState.settings, audioDuration);
```

### 3. Debug Visualization

In the debug page (`/debug`), the export state is visualized in real-time:

```typescript
// From debug/+page.svelte
$: {
  exportStateJSON = JSON.stringify($exportState, null, 2);
  exportStateHighlighted = Prism.highlight(exportStateJSON, Prism.languages.json, 'json');
  if (appExportState) {
    appExportState.innerHTML = exportStateHighlighted;
  }
}
```

This creates a live JSON view that updates whenever `exportState` changes, including when file size calculations are triggered.

## Example Calculations

### Example 1: 3-minute WAV file (CD quality)
- **Duration**: 180 seconds
- **Sample Rate**: 44,100 Hz
- **Channels**: 2 (stereo)
- **Bit Depth**: 16 bits

**Calculation:**
```
samplesPerSecond = 44,100 × 2 = 88,200
totalSamples = 88,200 × 180 = 15,876,000
bytesPerSample = 16 ÷ 8 = 2
rawAudioBytes = 15,876,000 × 2 = 31,752,000 bytes
estimatedBytes = 31,752,000 + 44 = 31,752,044 bytes ≈ 30.3 MB
```

### Example 2: 3-minute MP3 file (320 kbps)
- **Duration**: 180 seconds
- **Bitrate**: 320 kbps

**Calculation:**
```
estimatedBytes = (320 × 1000 ÷ 8) × 180 = 40,000 × 180 = 7,200,000 bytes ≈ 6.9 MB
```

### Example 3: 3-minute FLAC file
- **Same raw audio** as WAV example
- **Compression**: 60% of original

**Calculation:**
```
rawAudioBytes = 31,752,000 bytes (same as WAV)
estimatedBytes = 31,752,000 × 0.6 + 1,024 = 19,052,224 bytes ≈ 18.2 MB
```

## Performance Considerations

- **Lightweight calculation**: The function performs simple arithmetic operations
- **No I/O operations**: Pure calculation based on parameters
- **Immediate results**: Suitable for real-time UI updates
- **Cached by Svelte**: Reactive calculations are automatically optimized

## Integration Points

### 1. Export Settings UI
The function is used to show real-time file size estimates as users adjust export settings.

### 2. Audio Combining
When audio files are combined, the total duration is used to update file size estimates.

### 3. Format Selection
Changing formats immediately updates size estimates using format-specific calculations.

### 4. Quality Presets
When applying quality presets, the function recalculates estimates with new settings.

## Accuracy Notes

- **WAV files**: Very accurate (±44 bytes for header)
- **FLAC files**: Good estimate (compression varies by content)
- **MP3/OGG files**: Accurate for constant bitrate encoding
- **Variable bitrate**: Estimates may vary from actual files
- **Metadata**: Large metadata (album art, etc.) not included in estimates

## Future Enhancements

Potential improvements to the calculation system:

1. **Content-aware compression**: Analyze audio content to predict FLAC compression ratios
2. **Variable bitrate support**: More sophisticated MP3/OGG size estimation
3. **Metadata overhead**: Include album art and tag size estimates
4. **Historical accuracy**: Learn from previous exports to improve estimates
5. **Format-specific optimizations**: Per-format fine-tuning of calculations
