<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { save as testSave } from "@tauri-apps/plugin-dialog";
  import { invokeWithPerf } from "./state/performance";
  import { get } from "svelte/store";
  import {
    exportState,
    type ExportSettings,
    type ExportState,
  } from "./state/export";
  import { formatPercent } from "./utils/format";
  import { appState, getAllFiles } from "./state/state.svelte";

  const dispatch = createEventDispatcher();

  // local reactive copy (optional)
  let expState: ExportState = get(exportState);

  // whenever you update a field
  const update = (k: keyof ExportSettings, v: any) => {
    console.log(k);
    console.log(v);
    if (k === "format") {
      let newSettings = { ...expState.settings, format: v };
      if (v === "mp3") {
        newSettings.bitrate ??= 192;
      } else {
        delete newSettings.bitrate;
      }
      expState.settings = newSettings;
    } else {
      expState = { ...expState, [k]: v };
    }
    // console.log(exportState)
    // persist immediately
    exportState.set(expState);
    dispatch("exportSettingsChanged", expState);
  };

  const formatFields: Record<string, (keyof ExportSettings)[]> = {
    wav: ["sampleRate", "bitDepth", "channels"],
    flac: ["sampleRate", "bitDepth", "channels"],
    mp3: ["sampleRate", "channels", "bitrate"],
  };
  $: visibleFields = formatFields[expState.settings?.format] ?? [];

  const saveAudio = async () => {
    const path = await testSave({
      filters: [
        {
          name: "Audio Files",
          extensions: [expState.settings.format],
        },
      ],
    });
    if (path) {
      await invokeWithPerf("export_combined_audio_as_wav", {
        outputPath: path,
      });
    }
  };
</script>

<div style="font-size: 0.9rem" class="px-2 mt-1">
  <div class="row">
    <div class="col-2">
      <label
        >Filename <input
          type="text"
          bind:value={expState.settings.filename}
          on:input={() => update("filename", expState.settings.filename)}
        />
      </label>
    </div>
    <div class="col-1">
      <label
        >Format <select
          bind:value={expState.settings.format}
          on:change={() => update("format", expState.settings.format)}
        >
          <option value="wav">WAV</option> <option value="mp3">MP3</option>
          <option value="flac">FLAC</option>
        </select>
      </label>
    </div>
    {#if visibleFields.includes("sampleRate")}
      <div class="col-1">
        <label
          >Sample Rate <select
            bind:value={expState.settings.sampleRate}
            on:change={() => update("sampleRate", expState.settings.sampleRate)}
          >
            <option value={44100}>44100</option>
            <option value={48000}>48000</option>
            <option value={96000}>96000</option>
          </select>
        </label>
      </div>
    {/if}
    {#if visibleFields.includes("bitDepth")}
      <div class="col-1">
        <label
          >Bit Depth <select
            bind:value={expState.settings.bitDepth}
            on:change={() => update("bitDepth", expState.settings.bitDepth)}
          >
            <option value={16}>16</option> <option value={24}>24</option>
            <option value={32}>32</option>
          </select>
        </label>
      </div>
    {/if}
    {#if visibleFields.includes("channels")}
      <div class="col-1">
        <label
          >Channels <select
            bind:value={expState.settings.channels}
            on:change={() => update("channels", expState.settings.channels)}
          >
            <option value={1}>Mono</option> <option value={2}>Stereo</option>
          </select>
        </label>
      </div>
    {/if}
    {#if visibleFields.includes("bitrate")}
      <div class="col-1">
        <label
          >Bitrate (kbps) <select
            bind:value={expState.settings.bitrate}
            on:change={() => update("bitrate", expState.settings.bitrate)}
          >
            <option value={128}>128</option> <option value={192}>192</option>
            <option value={256}>256</option> <option value={320}>320</option>
          </select>
        </label>
      </div>
    {/if}
    <div class="col-1 mt-3">
      <div class="d-flex g-2">
        <button
          class="btn btn-sm btn-success"
          class:disabled={getAllFiles($appState.sections).length === 0}
          on:click={(e) => {
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
        <div class="d-flex text-danger"><i class="fa-solid fa-triangle-exclamation mt-1 me-1"></i>{$exportState.error}</div>
      {/if}
    </div>
  </div>
</div>

<style>
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
    content: "";
    display: inline-block;
    width: 1em;
    text-align: left;
    animation: dots 1s steps(4, end) infinite;
  }

  @keyframes dots {
    0% {
      content: "";
    }
    25% {
      content: ".";
    }
    50% {
      content: "..";
    }
    75% {
      content: "...";
    }
    100% {
      content: "";
    }
  }
</style>
