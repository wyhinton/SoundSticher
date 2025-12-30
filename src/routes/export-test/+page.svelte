<script lang="ts">
  import { exportState, type ExportSettings, calculateEstimatedFileSize } from '$lib/state/export';
  import { exportAudio } from '$lib/state/performance';
  import { onMount } from 'svelte';

  let testDuration = 10; // 10 seconds for testing
  let exporting = false;
  let lastExportResult = '';

  // Test presets for different scenarios
  const presets: Record<string, ExportSettings> = {
    'High Quality WAV': {
      sampleRate: 48000,
      bitDepth: 24,
      channels: 2,
      format: 'wav',
      filename: 'high_quality_test',
    },
    'CD Quality WAV': {
      sampleRate: 44100,
      bitDepth: 16,
      channels: 2,
      format: 'wav',
      filename: 'cd_quality_test',
    },
    'High Quality FLAC': {
      sampleRate: 96000,
      bitDepth: 24,
      channels: 2,
      format: 'flac',
      filename: 'high_quality_test',
    },
    'MP3 320kbps': {
      sampleRate: 48000,
      bitDepth: 16,
      channels: 2,
      format: 'mp3',
      filename: 'high_quality_test',
      bitrate: 320,
    },
    'MP3 192kbps': {
      sampleRate: 44100,
      bitDepth: 16,
      channels: 2,
      format: 'mp3',
      filename: 'standard_quality_test',
      bitrate: 192,
    },
    'MP3 128kbps': {
      sampleRate: 44100,
      bitDepth: 16,
      channels: 2,
      format: 'mp3',
      filename: 'standard_quality_test',
      bitrate: 128,
    },
  };

  // Calculate estimated sizes for all presets
  $: estimatedSizes = Object.entries(presets).map(([name, settings]) => ({
    name,
    settings,
    estimate: calculateEstimatedFileSize(settings, testDuration),
  }));

  async function applyPreset(settings: ExportSettings) {
    exportState.update(state => ({
      ...state,
      settings,
    }));
  }

  async function testExport() {
    if (!$exportState.settings) return;

    exporting = true;
    lastExportResult = '';

    try {
      const outputPath = `C:\\Users\\Primary User\\Desktop\\AUDIO\\test_export.${$exportState.settings.format}`;
      await exportAudio($exportState.settings, outputPath);
      lastExportResult = `Successfully exported to: ${outputPath}`;
    } catch (error) {
      lastExportResult = `Export failed: ${error}`;
      console.error('Export error:', error);
    } finally {
      exporting = false;
    }
  }

  function formatBytes(bytes: number): string {
    const sizes = ['Bytes', 'KB', 'MB', 'GB'];
    if (bytes === 0) return '0 Bytes';
    const i = Math.floor(Math.log(bytes) / Math.log(1024));
    return Math.round((bytes / Math.pow(1024, i)) * 100) / 100 + ' ' + sizes[i];
  }

  function getBitrateText(settings: ExportSettings): string {
    if (settings.bitrate) {
      return ` @ ${settings.bitrate}kbps`;
    }
    return '';
  }
</script>

<div class="export-test">
  <h2>Audio Export Settings Test</h2>

  <div class="test-controls">
    <label>
      Test Duration:
      <input type="number" bind:value={testDuration} min="1" max="300" step="1" />
      seconds
    </label>
  </div>

  <div class="presets">
    <h3>Quality Presets</h3>
    <div class="preset-grid">
      {#each estimatedSizes as { name, settings, estimate }}
        <div class="preset-card">
          <button
            class="preset-button"
            on:click={() => applyPreset(settings)}
            class:active={JSON.stringify($exportState.settings) === JSON.stringify(settings)}
          >
            <h4>{name}</h4>
            <div class="preset-details">
              <div>{settings.format.toUpperCase()}</div>
              <div>{settings.sampleRate}Hz, {settings.bitDepth}-bit</div>
              <div>{settings.channels}ch{getBitrateText(settings)}</div>
            </div>
            <div class="size-estimate">
              <strong>{estimate.formatted}</strong>
              <small>({testDuration}s)</small>
            </div>
          </button>
        </div>
      {/each}
    </div>
  </div>

  <div class="current-settings">
    <h3>Current Export Settings</h3>
    {#if $exportState.settings}
      <div class="settings-display">
        <div class="setting-row">
          <label>Format:</label>
          <span>{$exportState.settings.format.toUpperCase()}</span>
        </div>
        <div class="setting-row">
          <label>Sample Rate:</label>
          <span>{$exportState.settings.sampleRate}Hz</span>
        </div>
        <div class="setting-row">
          <label>Bit Depth:</label>
          <span>{$exportState.settings.bitDepth}-bit</span>
        </div>
        <div class="setting-row">
          <label>Channels:</label>
          <span>{$exportState.settings.channels}</span>
        </div>
        {#if $exportState.settings.bitrate}
          <div class="setting-row">
            <label>Bitrate:</label>
            <span>{$exportState.settings.bitrate}kbps</span>
          </div>
        {/if}
        <div class="setting-row">
          <label>Estimated Size:</label>
          <span>{calculateEstimatedFileSize($exportState.settings, testDuration).formatted}</span>
        </div>
      </div>

      <div class="export-actions">
        <button class="export-button" on:click={testExport} disabled={exporting}>
          {#if exporting}
            <span class="spinner">⏳</span> Exporting...
          {:else}
            🎵 Test Export
          {/if}
        </button>
      </div>

      {#if lastExportResult}
        <div class="export-result" class:success={!lastExportResult.includes('failed')}>
          {lastExportResult}
        </div>
      {/if}
    {:else}
      <p>No export settings configured</p>
    {/if}
  </div>

  <div class="comparison">
    <h3>Format Comparison</h3>
    <div class="comparison-table">
      <div class="comparison-header">
        <div>Format</div>
        <div>Quality</div>
        <div>File Size</div>
        <div>Compression</div>
      </div>
      {#each estimatedSizes as { name, settings, estimate }}
        <div class="comparison-row">
          <div class="format-name">{name}</div>
          <div class="quality">
            {settings.sampleRate / 1000}kHz/{settings.bitDepth}bit
            {#if settings.bitrate}
              <br /><small>{settings.bitrate}kbps</small>
            {/if}
          </div>
          <div class="file-size">{estimate.formatted}</div>
          <div class="compression">
            {estimate.breakdown.formatOverhead > 0
              ? `+${formatBytes(estimate.breakdown.formatOverhead)}`
              : estimate.breakdown.formatOverhead < 0
                ? formatBytes(Math.abs(estimate.breakdown.formatOverhead)) + ' saved'
                : 'None'}
          </div>
        </div>
      {/each}
    </div>
  </div>
</div>

<style>
  .export-test {
    padding: 20px;
    max-width: 1200px;
    margin: 0 auto;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  }

  .export-test h2 {
    color: #fff;
    margin-bottom: 30px;
  }

  .export-test h3 {
    color: #fff;
    margin: 20px 0 15px 0;
  }

  .test-controls {
    background: #2a2a2a;
    padding: 15px;
    border-radius: 8px;
    margin-bottom: 25px;
  }

  .test-controls label {
    color: #fff;
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .test-controls input {
    background: #1a1a1a;
    border: 1px solid #555;
    color: #fff;
    padding: 5px 10px;
    border-radius: 4px;
    width: 80px;
  }

  .preset-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: 15px;
    margin-bottom: 30px;
  }

  .preset-card {
    background: #2a2a2a;
    border-radius: 8px;
    overflow: hidden;
  }

  .preset-button {
    width: 100%;
    background: transparent;
    border: 2px solid #555;
    color: #fff;
    padding: 15px;
    cursor: pointer;
    transition: all 0.2s ease;
    text-align: left;
  }

  .preset-button:hover {
    border-color: #007acc;
    background: rgba(0, 122, 204, 0.1);
  }

  .preset-button.active {
    border-color: #007acc;
    background: rgba(0, 122, 204, 0.2);
  }

  .preset-button h4 {
    margin: 0 0 10px 0;
    color: #fff;
    font-size: 16px;
  }

  .preset-details {
    font-size: 14px;
    color: #ccc;
    margin-bottom: 10px;
  }

  .preset-details div {
    margin-bottom: 2px;
  }

  .size-estimate {
    color: #007acc;
    font-weight: 600;
    font-size: 14px;
  }

  .size-estimate small {
    color: #999;
    font-weight: normal;
    margin-left: 5px;
  }

  .current-settings {
    background: #2a2a2a;
    padding: 20px;
    border-radius: 8px;
    margin-bottom: 25px;
  }

  .settings-display {
    margin-bottom: 20px;
  }

  .setting-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 0;
    border-bottom: 1px solid #3a3a3a;
  }

  .setting-row:last-child {
    border-bottom: none;
  }

  .setting-row label {
    color: #ccc;
    font-weight: 500;
  }

  .setting-row span {
    color: #fff;
    font-weight: 600;
  }

  .export-actions {
    text-align: center;
  }

  .export-button {
    background: #007acc;
    color: #fff;
    border: none;
    padding: 12px 24px;
    border-radius: 6px;
    font-size: 16px;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.2s ease;
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }

  .export-button:hover:not(:disabled) {
    background: #005a9a;
  }

  .export-button:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .spinner {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }

  .export-result {
    margin-top: 15px;
    padding: 12px;
    border-radius: 6px;
    font-weight: 500;
  }

  .export-result.success {
    background: rgba(40, 167, 69, 0.2);
    color: #28a745;
    border: 1px solid #28a745;
  }

  .export-result:not(.success) {
    background: rgba(220, 53, 69, 0.2);
    color: #dc3545;
    border: 1px solid #dc3545;
  }

  .comparison {
    background: #2a2a2a;
    padding: 20px;
    border-radius: 8px;
  }

  .comparison-table {
    display: grid;
    grid-template-columns: 2fr 1fr 1fr 1fr;
    gap: 1px;
    background: #555;
  }

  .comparison-header,
  .comparison-row {
    display: contents;
  }

  .comparison-header > div {
    background: #1a1a1a;
    padding: 12px;
    font-weight: 600;
    color: #fff;
    text-align: center;
  }

  .comparison-row > div {
    background: #2a2a2a;
    padding: 12px;
    color: #ccc;
    text-align: center;
  }

  .format-name {
    text-align: left !important;
    font-weight: 500;
    color: #fff !important;
  }

  .file-size {
    font-weight: 600;
    color: #007acc !important;
  }

  .quality {
    font-size: 14px;
  }

  .compression {
    font-size: 14px;
  }
</style>
