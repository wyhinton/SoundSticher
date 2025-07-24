<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { save as testSave } from '@tauri-apps/plugin-dialog';
  import { invokeWithPerf } from './state/performance';
  const dispatch = createEventDispatcher();

  export let settings = {
    sampleRate: 44100,
    bitDepth: 16,
    channels: 2,
    format: 'wav',
    filename: 'exported_audio'
  };

  const update = (k: keyof typeof settings, v: any) => {
    settings = { ...settings, [k]: v };
    dispatch('exportSettingsChanged', settings);
  };



  const saveAudio = () =>{
      const z = testSave({
      filters: [
        {
          name: 'Audio Files',
          extensions: ['.wav'],
        },
      ],
    }).then(z=>{
      invokeWithPerf("export_combined_audio_as_wav", {outputPath: z})
      console.log(z)
    });
  }

</script>

<div style="font-size: 0.9rem" class="px-2">
  <div class="row">
      <div class="col-2">
          <label>Filename
                <input type="text" bind:value={settings.filename} on:input={() => update('filename', settings.filename)} />
            </label>
      </div>
      <div class="col-1">
          <label>Sample Rate
            <select bind:value={settings.sampleRate} on:change={() => update('sampleRate', settings.sampleRate)}>
              <option value={44100}>44100</option>
              <option value={48000}>48000</option>
              <option value={96000}>96000</option>
            </select>
          </label>
      </div>
      <div class="col-1">
          <label>Bit Depth
            <select bind:value={settings.bitDepth} on:change={() => update('bitDepth', settings.bitDepth)}>
              <option value={16}>16</option>
              <option value={24}>24</option>
              <option value={32}>32</option>
            </select>
          </label>
      </div>
      <div class="col-1">
          <label>Channels
            <select bind:value={settings.channels} on:change={() => update('channels', settings.channels)}>
              <option value={1}>Mono</option>
              <option value={2}>Stereo</option>
            </select>
          </label>
      </div>
      <div class="col-1">
          <label>Format
            <select bind:value={settings.format} on:change={() => update('format', settings.format)}>
              <option value="wav">WAV</option>
              <option value="mp3">MP3</option>
              <option value="flac">FLAC</option>
            </select>
          </label>
      </div>
      <div class="col-2 mt-3">
        <button class="btn btn-sm btn-success" on:click={(e)=>{saveAudio()}}>Export <i class="fa-solid fa-right-to-bracket"></i> </button>
      </div>
  </div>


</div>

<style>


  select, input {
    font-size: 0.85rem;
    padding: 0.2rem 0.4rem;
    border: 1px solid #ccc;
    border-radius: 3px;
    width: 100%;
    height: 23px;
  }
  label {
    display: flex;
    flex-direction: column;
    color: #3091f1;
    font-size: 12px;
  }
</style>
