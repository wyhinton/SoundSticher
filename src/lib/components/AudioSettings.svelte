<script lang="ts">
  import AudioDeviceSelector from './AudioDeviceSelector.svelte';
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  let audioVolume = 1.0;
  let audioEnabled = true;
  let bufferSize = 1024;
  let sampleRate = 44100;
  let showAdvanced = false;

  const bufferSizeOptions = [256, 512, 1024, 2048, 4096];
  const sampleRateOptions = [22050, 44100, 48000, 88200, 96000, 192000];

  onMount(() => {
    loadAudioSettings();
  });

  function loadAudioSettings() {
    // Load saved settings from localStorage or backend
    const savedVolume = localStorage.getItem('audioVolume');
    const savedEnabled = localStorage.getItem('audioEnabled');
    const savedBufferSize = localStorage.getItem('audioBufferSize');
    const savedSampleRate = localStorage.getItem('audioSampleRate');

    if (savedVolume) audioVolume = parseFloat(savedVolume);
    if (savedEnabled) audioEnabled = savedEnabled === 'true';
    if (savedBufferSize) bufferSize = parseInt(savedBufferSize);
    if (savedSampleRate) sampleRate = parseInt(savedSampleRate);
  }

  function saveAudioSettings() {
    localStorage.setItem('audioVolume', audioVolume.toString());
    localStorage.setItem('audioEnabled', audioEnabled.toString());
    localStorage.setItem('audioBufferSize', bufferSize.toString());
    localStorage.setItem('audioSampleRate', sampleRate.toString());
  }

  async function applyVolumeChange() {
    try {
      await invoke('set_volume', { volume: audioVolume });
      saveAudioSettings();
    } catch (e) {
      console.error('Failed to apply volume change:', e);
    }
  }

  function handleVolumeInput(event: Event) {
    const target = event.target as HTMLInputElement;
    audioVolume = parseFloat(target.value);
  }

  function handleVolumeChange() {
    applyVolumeChange();
  }

  function toggleAudio() {
    audioEnabled = !audioEnabled;
    if (!audioEnabled) {
      // Pause audio when disabled
      invoke('pause_timeline_audio').catch(console.error);
    }
    saveAudioSettings();
  }

  function handleBufferSizeChange() {
    saveAudioSettings();
    // Buffer size change would require audio system restart
    console.log('Buffer size changed to:', bufferSize);
  }

  function handleSampleRateChange() {
    saveAudioSettings();
    // Sample rate change would require audio system restart
    console.log('Sample rate changed to:', sampleRate);
  }

  function formatVolume(volume: number): string {
    return `${Math.round(volume * 100)}%`;
  }
</script>

<div class="audio-settings">
  <div class="settings-header">
    <h2>Audio Settings</h2>
    <button
      class="toggle-advanced"
      on:click={() => (showAdvanced = !showAdvanced)}
      aria-expanded={showAdvanced}
    >
      Advanced {showAdvanced ? '▲' : '▼'}
    </button>
  </div>

  <!-- Audio Device Selection -->
  <div class="settings-section">
    <AudioDeviceSelector />
  </div>

  <!-- Volume Control -->
  <div class="settings-section">
    <div class="setting-group">
      <div class="setting-header">
        <label class="setting-label" for="volume-control">
          <span class="volume-icon"
            >{audioVolume === 0 ? '🔇' : audioVolume < 0.5 ? '🔉' : '🔊'}</span
          >
          Volume: <strong>{formatVolume(audioVolume)}</strong>
        </label>
        <button
          class="mute-btn"
          on:click={toggleAudio}
          class:muted={!audioEnabled}
          title={audioEnabled ? 'Mute audio' : 'Unmute audio'}
        >
          {audioEnabled ? '🔊' : '🔇'}
        </button>
      </div>

      <div class="volume-control">
        <input
          id="volume-control"
          type="range"
          min="0"
          max="1"
          step="0.01"
          value={audioVolume}
          disabled={!audioEnabled}
          on:input={handleVolumeInput}
          on:change={handleVolumeChange}
          class="volume-slider"
        />
        <div class="volume-markers">
          <span>0%</span>
          <span>25%</span>
          <span>50%</span>
          <span>75%</span>
          <span>100%</span>
        </div>
      </div>
    </div>
  </div>

  <!-- Advanced Settings -->
  {#if showAdvanced}
    <div class="settings-section advanced">
      <h3>Advanced Audio Settings</h3>

      <div class="advanced-grid">
        <div class="setting-group">
          <label class="setting-label" for="buffer-size">
            Buffer Size
            <span class="setting-hint">Lower = less latency, higher = more stable</span>
          </label>
          <select
            id="buffer-size"
            bind:value={bufferSize}
            on:change={handleBufferSizeChange}
            class="setting-select"
          >
            {#each bufferSizeOptions as size}
              <option value={size}>{size} samples</option>
            {/each}
          </select>
        </div>

        <div class="setting-group">
          <label class="setting-label" for="sample-rate">
            Sample Rate
            <span class="setting-hint">Higher = better quality, more CPU usage</span>
          </label>
          <select
            id="sample-rate"
            bind:value={sampleRate}
            on:change={handleSampleRateChange}
            class="setting-select"
          >
            {#each sampleRateOptions as rate}
              <option value={rate}>{rate / 1000}kHz</option>
            {/each}
          </select>
        </div>
      </div>

      <div class="performance-info">
        <h4>Current Audio Configuration</h4>
        <div class="config-info">
          <div class="config-item">
            <span class="config-label">Latency (approx.):</span>
            <span class="config-value">{Math.round((bufferSize / sampleRate) * 1000)}ms</span>
          </div>
          <div class="config-item">
            <span class="config-label">Memory per second:</span>
            <span class="config-value">{Math.round((sampleRate * 2 * 4) / 1024)}KB</span>
          </div>
          <div class="config-item">
            <span class="config-label">Audio enabled:</span>
            <span class="config-value" class:enabled={audioEnabled} class:disabled={!audioEnabled}>
              {audioEnabled ? 'Yes' : 'No'}
            </span>
          </div>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .audio-settings {
    background: #1e1e1e;
    border-radius: 12px;
    padding: 20px;
    margin: 16px 0;
    color: #ffffff;
    font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
  }

  .settings-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 24px;
  }

  .settings-header h2 {
    margin: 0;
    font-size: 24px;
    font-weight: 600;
    color: #ffffff;
  }

  .toggle-advanced {
    background: #3a3a3a;
    border: 1px solid #555;
    color: #ffffff;
    padding: 8px 16px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 14px;
    transition: all 0.2s ease;
  }

  .toggle-advanced:hover {
    background: #4a4a4a;
    border-color: #777;
  }

  .settings-section {
    margin-bottom: 24px;
  }

  .settings-section.advanced {
    background: #2a2a2a;
    border-radius: 8px;
    padding: 20px;
    border: 1px solid #3a3a3a;
  }

  .settings-section h3 {
    margin: 0 0 20px 0;
    font-size: 18px;
    font-weight: 600;
    color: #ffffff;
  }

  .setting-group {
    margin-bottom: 20px;
  }

  .setting-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 12px;
  }

  .setting-label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 16px;
    font-weight: 500;
    color: #ffffff;
  }

  .volume-icon {
    font-size: 20px;
    margin-right: 8px;
  }

  .setting-hint {
    font-size: 12px;
    color: #cccccc;
    font-weight: normal;
  }

  .mute-btn {
    background: transparent;
    border: 1px solid #555;
    color: #ffffff;
    padding: 8px 12px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 16px;
    transition: all 0.2s ease;
  }

  .mute-btn:hover {
    background: #3a3a3a;
    border-color: #777;
  }

  .mute-btn.muted {
    background: #ff4444;
    border-color: #ff4444;
  }

  .volume-control {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .volume-slider {
    width: 100%;
    height: 6px;
    background: #3a3a3a;
    border-radius: 3px;
    outline: none;
    transition: background 0.2s ease;
  }

  .volume-slider::-webkit-slider-thumb {
    appearance: none;
    width: 20px;
    height: 20px;
    background: #007acc;
    border-radius: 50%;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .volume-slider::-webkit-slider-thumb:hover {
    background: #0066aa;
    transform: scale(1.1);
  }

  .volume-slider:disabled {
    opacity: 0.5;
  }

  .volume-slider:disabled::-webkit-slider-thumb {
    cursor: not-allowed;
    transform: none;
  }

  .volume-markers {
    display: flex;
    justify-content: space-between;
    font-size: 12px;
    color: #888;
    padding: 0 10px;
  }

  .advanced-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
    gap: 20px;
    margin-bottom: 24px;
  }

  .setting-select {
    width: 100%;
    background: #3a3a3a;
    border: 1px solid #555;
    color: #ffffff;
    padding: 8px 12px;
    border-radius: 6px;
    font-size: 14px;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .setting-select:hover {
    border-color: #777;
  }

  .setting-select:focus {
    outline: none;
    border-color: #007acc;
    box-shadow: 0 0 0 2px rgba(0, 122, 204, 0.2);
  }

  .performance-info {
    background: #1a1a1a;
    border-radius: 8px;
    padding: 16px;
    border: 1px solid #333;
  }

  .performance-info h4 {
    margin: 0 0 12px 0;
    font-size: 16px;
    color: #ffffff;
  }

  .config-info {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 12px;
  }

  .config-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 12px;
    background: #2a2a2a;
    border-radius: 6px;
    border: 1px solid #3a3a3a;
  }

  .config-label {
    font-size: 14px;
    color: #cccccc;
  }

  .config-value {
    font-size: 14px;
    font-weight: 600;
    color: #ffffff;
  }

  .config-value.enabled {
    color: #28a745;
  }

  .config-value.disabled {
    color: #ff4444;
  }

  /* Slide transition for advanced settings */
  :global(.slide-enter-active),
  :global(.slide-leave-active) {
    transition: all 0.3s ease;
  }

  :global(.slide-enter-from),
  :global(.slide-leave-to) {
    opacity: 0;
    transform: translateY(-10px);
  }
</style>
