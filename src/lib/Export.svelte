<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { save as testSave } from '@tauri-apps/plugin-dialog';
  import { invokeWithPerf } from './state/performance';
  import { get } from 'svelte/store';
  import {
    exportState,
    applyFormatDefaults,
    type ExportSettings,
    type ExportState,
  } from './state/export';
  import { formatPercent } from './utils/format';
  import { appState, getAllFiles, currentOperationSections } from './state/state.svelte';
  import { createTypedEventChannelWithLogging } from './utils/channelMaker';
  import type { ExportAudioEvent } from './state/events';

  const dispatch = createEventDispatcher();

  // local reactive copy (optional)
  let expState: ExportState = get(exportState);

  // whenever you update a field
  const update = (k: keyof ExportSettings, v: any) => {
    if (k === 'format') {
      // Apply format-specific defaults when format changes
      expState.settings = applyFormatDefaults(expState.settings!, v);
    } else {
      // For other settings, just update the specific field
      if (expState.settings) {
        expState.settings = { ...expState.settings, [k]: v };
      }
    }
    // persist immediately
    exportState.set(expState);
    dispatch('exportSettingsChanged', expState);
  };

  const formatFields: Record<string, (keyof ExportSettings)[]> = {
    wav: ['sampleRate', 'bitDepth', 'channels'],
    flac: ['sampleRate', 'bitDepth', 'channels'],
    mp3: ['channels', 'bitrate'],
  };
  $: visibleFields = formatFields[expState.settings?.format] ?? [];

  const openInExplorer = async (filePath: string) => {
    try {
      console.log('📁 Opening file in explorer:', filePath);
      await invokeWithPerf('open_in_explorer', {
        fileToOpen: filePath,
      });
    } catch (error) {
      console.error('❌ Failed to open in explorer:', error);
    }
  };
  const saveAudio = async () => {
    const path = await testSave({
      filters: [
        {
          name: 'Audio Files',
          extensions: [expState.settings.format],
        },
      ],
      title: 'Save audio',
      defaultPath: expState.settings?.filename,
    });
    console.log(path);
    if (path) {
      // Create typed event channel with logging for export progress
      const onEvent = createTypedEventChannelWithLogging<ExportAudioEvent>('Export', {
        onStarted: data => {
          console.log('🎵 Export started:', data);
          exportState.update(state => ({
            ...state,
            progress: 0,
            message: `Starting export to ${data.outputPath}`,
            error: undefined,
          }));
        },
        onProgress: data => {
          console.log('📈 Export progress:', data);
          exportState.update(state => ({
            ...state,
            progress: data.progress,
            message: data.message,
            error: undefined,
          }));
        },
        onFinished: data => {
          console.log('🎉 Export completed:', data);
          exportState.update(state => ({
            ...state,
            progress: 1,
            message: data.message,
            error: undefined,
            outputPath: data.outputPath,
          }));
          // Clear the progress after a short delay
          setTimeout(() => {
            exportState.update(state => ({
              ...state,
              progress: 0,
              message: undefined,
              outputPath: undefined,
            }));
          }, 5000);
        },
      });

      try {
        await invokeWithPerf('export_audio', {
          settings: expState.settings,
          outputFile: path,
          onEvent: onEvent,
        });
      } catch (error) {
        console.error('❌ Export failed:', error);
        exportState.update(state => ({
          ...state,
          progress: 0,
          message: undefined,
          error: `Export failed: ${error}`,
        }));
      }
    }
  };
</script>

<div style="font-size: 0.9rem" class="p-2 export-panel" data-export-panel>
  <div class="row">
    <div class="col-2">
      <label
        >Filename <input
          type="text"
          bind:value={expState.settings.filename}
          on:input={() => update('filename', expState.settings.filename)}
        />
      </label>
    </div>
    <div class="col-1">
      <label
        >Format <select
          bind:value={expState.settings.format}
          on:change={() => update('format', expState.settings.format)}
        >
          <option value="wav">WAV</option> <option value="mp3">MP3</option>
          <option value="flac">FLAC</option>
        </select>
      </label>
    </div>
    {#if visibleFields.includes('sampleRate')}
      <div class="col-1">
        <label
          >Sample Rate <select
            bind:value={expState.settings.sampleRate}
            on:change={() => update('sampleRate', expState.settings.sampleRate)}
          >
            <option value={44100}>44.1 kHz</option>
            <option value={48000}>48 kHz</option>
            <option value={88200}>88.2 kHz</option>
            <option value={96000}>96 kHz</option>
            <option value={192000}>192 kHz</option>
          </select>
        </label>
      </div>
    {/if}
    {#if visibleFields.includes('bitDepth')}
      <div class="col-1">
        <label
          >Bit Depth <select
            bind:value={expState.settings.bitDepth}
            on:change={() => update('bitDepth', expState.settings.bitDepth)}
          >
            <option value={16}>16</option> <option value={24}>24</option>
            <option value={32}>32</option>
          </select>
        </label>
      </div>
    {/if}
    {#if visibleFields.includes('channels')}
      <div class="col-1">
        <label
          >Channels <select
            bind:value={expState.settings.channels}
            on:change={() => update('channels', expState.settings.channels)}
          >
            <option value={1}>Mono</option> <option value={2}>Stereo</option>
          </select>
        </label>
      </div>
    {/if}
    {#if visibleFields.includes('bitrate')}
      <div class="col-1">
        <label
          >Bitrate (kbps) <select
            bind:value={expState.settings.bitrate}
            on:change={() => update('bitrate', expState.settings.bitrate)}
          >
            <option value={128}>128</option> <option value={192}>192</option>
            <option value={256}>256</option> <option value={320}>320</option>
            <option value={500}>500 (VBR)</option>
          </select>
        </label>
      </div>
    {/if}
    <div class="col-1 mt-3">
      <div class="d-flex g-2">
        <button
          class="btn btn-sm btn-success"
          class:disabled={getAllFiles($currentOperationSections).length === 0}
          on:click={e => {
            saveAudio();
          }}
          >Export <i class="fa-solid fa-right-to-bracket"></i>
        </button>
      </div>
    </div>
    <div class="col-4 mt-3">
      {#if $exportState && ($exportState.message || $exportState.progress)}
        {#if $exportState.progress === -1}
          <div>{$exportState.message}<span class="dots"></span></div>
        {:else}
          <div>{formatPercent($exportState.progress)}</div>
        {/if}
      {/if}
      {#if $exportState && $exportState.error}
        <div class="d-flex text-danger">
          <i class="fa-solid fa-triangle-exclamation mt-1 me-1"></i>{$exportState.error}
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .export-panel {
    background-color: #2d3747;
    border: 1px solid #1a252f;
  }
  select,
  input {
    font-size: 0.85rem;
    padding: 0.2rem 0.4rem;
    border: 1px solid #6b6b6b;
    border-radius: 3px;
    width: 100%;
    height: 28px;
    background-color: #101010;
  }
  label {
    display: flex;
    flex-direction: column;
    color: #adadad;
    font-size: 12px;
  }

  .dots::after {
    content: '';
    display: inline-block;
    width: 1em;
    text-align: left;
    animation: dots 1s steps(4, end) infinite;
  }

  @keyframes dots {
    0% {
      content: '';
    }
    25% {
      content: '.';
    }
    50% {
      content: '..';
    }
    75% {
      content: '...';
    }
    100% {
      content: '';
    }
  }

  .export-completed {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .success-text {
    color: #68d391;
    font-weight: 600;
    font-size: 0.85rem;
  }

  .file-path-link {
    background: linear-gradient(135deg, #4a5568, #2d3748);
    border: 1px solid #68d391;
    border-radius: 4px;
    color: #e2e8f0;
    padding: 6px 10px;
    font-size: 0.75rem;
    font-family: 'Courier New', monospace;
    cursor: pointer;
    transition: all 0.2s ease;
    text-align: left;
    max-width: 400px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    display: inline-block;
    user-select: text;
  }

  .file-path-link:hover {
    background: linear-gradient(135deg, #68d391, #4fd1c7);
    color: #1a202c;
    border-color: #9ae6b4;
    transform: translateY(-1px);
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
  }

  .file-path-link:active {
    transform: translateY(0);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.2);
  }

  .file-path-link:focus {
    outline: 2px solid #68d391;
    outline-offset: 2px;
  }
</style>
