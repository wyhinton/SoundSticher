<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';

  interface AudioDeviceInfo {
    name: string;
    is_default: boolean;
    sample_rates: number[];
    channels: number;
    sample_format: string;
  }

  let devices: AudioDeviceInfo[] = [];
  let currentDevice: string | null = null;
  let followSystem = true;
  let loading = false;
  let error: string | null = null;

  // Load audio devices on mount
  onMount(async () => {
    await loadDevices();
    await loadCurrentDevice();
    await loadFollowSystemSetting();

    // Listen for device changes from backend
    const unlistenDeviceUpdate = await listen('audio-devices-updated', event => {
      devices = event.payload as AudioDeviceInfo[];
    });

    const unlistenDeviceChanged = await listen('audio-device-changed', event => {
      currentDevice = event.payload as string;
    });

    const unlistenDeviceDisconnected = await listen('audio-device-disconnected', event => {
      const disconnectedDevice = event.payload as string;
      console.warn(`Audio device '${disconnectedDevice}' was disconnected`);
      // Refresh devices to get current state
      loadDevices();
    });

    const unlistenFollowSystemChanged = await listen('follow-system-changed', event => {
      followSystem = event.payload as boolean;
    });

    // Cleanup listeners on component destroy
    return () => {
      unlistenDeviceUpdate();
      unlistenDeviceChanged();
      unlistenDeviceDisconnected();
      unlistenFollowSystemChanged();
    };
  });

  async function loadDevices() {
    try {
      loading = true;
      error = null;
      devices = await invoke('get_audio_devices');
    } catch (e) {
      error = `Failed to load audio devices: ${e}`;
      console.error('Error loading audio devices:', e);
    } finally {
      loading = false;
    }
  }

  async function loadCurrentDevice() {
    try {
      currentDevice = await invoke('get_current_audio_device');
    } catch (e) {
      console.error('Error loading current device:', e);
    }
  }

  async function loadFollowSystemSetting() {
    try {
      followSystem = await invoke('is_following_system_audio');
    } catch (e) {
      console.error('Error loading follow system setting:', e);
    }
  }

  async function refreshDevices() {
    try {
      loading = true;
      error = null;
      devices = await invoke('refresh_audio_devices');
    } catch (e) {
      error = `Failed to refresh audio devices: ${e}`;
      console.error('Error refreshing audio devices:', e);
    } finally {
      loading = false;
    }
  }

  async function changeDevice(deviceName: string) {
    try {
      await invoke('change_audio_device', { deviceName });
      currentDevice = deviceName;

      // If manually selecting a device, disable follow system
      if (followSystem) {
        await setFollowSystem(false);
      }
    } catch (e) {
      error = `Failed to change audio device: ${e}`;
      console.error('Error changing audio device:', e);
    }
  }

  async function setFollowSystem(follow: boolean) {
    try {
      await invoke('set_follow_system_audio', { follow });
      followSystem = follow;
    } catch (e) {
      error = `Failed to set follow system: ${e}`;
      console.error('Error setting follow system:', e);
    }
  }

  function formatSampleRates(rates: number[]): string {
    if (rates.length === 0) return 'Unknown';
    const sortedRates = [...rates].sort((a, b) => b - a);
    return sortedRates
      .slice(0, 3)
      .map(rate => `${rate / 1000}kHz`)
      .join(', ');
  }

  function getDeviceIcon(device: AudioDeviceInfo): string {
    if (device.is_default) return '🔊';
    if (device.name.toLowerCase().includes('speaker')) return '🔈';
    if (device.name.toLowerCase().includes('headphone')) return '🎧';
    if (device.name.toLowerCase().includes('bluetooth')) return '📻';
    if (device.name.toLowerCase().includes('usb')) return '🔌';
    return '🎵';
  }
</script>

<div class="audio-device-manager">
  <div class="header">
    <h3>Audio Output Device</h3>
    <button
      class="refresh-btn"
      on:click={refreshDevices}
      disabled={loading}
      title="Refresh audio devices"
    >
      <span class="refresh-icon" class:spinning={loading}>🔄</span>
    </button>
  </div>

  {#if error}
    <div class="error">
      <span class="error-icon">⚠️</span>
      {error}
      <button class="retry-btn" on:click={loadDevices}>Retry</button>
    </div>
  {/if}

  <div class="follow-system">
    <label class="checkbox-container">
      <input
        type="checkbox"
        bind:checked={followSystem}
        on:change={() => setFollowSystem(followSystem)}
      />
      <span class="checkmark"></span>
      Follow system default output device
    </label>
    <p class="follow-system-desc">
      Automatically switch to the system's default audio output device when it changes.
    </p>
  </div>

  <div class="device-list">
    {#if loading && devices.length === 0}
      <div class="loading">
        <span class="spinner">⏳</span>
        Loading audio devices...
      </div>
    {:else if devices.length === 0}
      <div class="no-devices">
        <span class="no-devices-icon">🔇</span>
        No audio output devices found
      </div>
    {:else}
      {#each devices as device}
        <div
          class="device-item"
          class:selected={currentDevice === device.name}
          class:disabled={followSystem}
        >
          <button
            class="device-button"
            on:click={() => changeDevice(device.name)}
            disabled={followSystem}
            title={followSystem
              ? 'Disable "Follow system default" to manually select devices'
              : `Select ${device.name}`}
          >
            <div class="device-info">
              <div class="device-header">
                <span class="device-icon">{getDeviceIcon(device)}</span>
                <span class="device-name">{device.name}</span>
                {#if device.is_default}
                  <span class="default-badge">Default</span>
                {/if}
                {#if currentDevice === device.name}
                  <span class="current-badge">Current</span>
                {/if}
              </div>
              <div class="device-details">
                <span class="detail">{device.channels} channels</span>
                <span class="detail">{formatSampleRates(device.sample_rates)}</span>
                <span class="detail">{device.sample_format}</span>
              </div>
            </div>
          </button>
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .audio-device-manager {
    background: #2a2a2a;
    border-radius: 8px;
    padding: 16px;
    margin: 8px 0;
    font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
  }

  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 16px;
  }

  .header h3 {
    margin: 0;
    color: #ffffff;
    font-size: 18px;
    font-weight: 600;
  }

  .refresh-btn {
    background: transparent;
    border: 1px solid #555;
    border-radius: 6px;
    padding: 8px;
    cursor: pointer;
    color: #ffffff;
    transition: all 0.2s ease;
  }

  .refresh-btn:hover:not(:disabled) {
    background: #3a3a3a;
    border-color: #777;
  }

  .refresh-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .refresh-icon {
    font-size: 16px;
    display: inline-block;
    transition: transform 0.3s ease;
  }

  .refresh-icon.spinning {
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

  .error {
    background: #ff4444;
    color: #ffffff;
    padding: 12px;
    border-radius: 6px;
    margin-bottom: 16px;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .error-icon {
    font-size: 16px;
  }

  .retry-btn {
    background: transparent;
    border: 1px solid #ffffff;
    color: #ffffff;
    padding: 4px 12px;
    border-radius: 4px;
    cursor: pointer;
    margin-left: auto;
    transition: background 0.2s ease;
  }

  .retry-btn:hover {
    background: rgba(255, 255, 255, 0.1);
  }

  .follow-system {
    margin-bottom: 20px;
  }

  .checkbox-container {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    color: #ffffff;
    font-weight: 500;
  }

  .checkbox-container input[type='checkbox'] {
    width: 16px;
    height: 16px;
    accent-color: #007acc;
    cursor: pointer;
  }

  .follow-system-desc {
    margin: 8px 0 0 24px;
    font-size: 14px;
    color: #cccccc;
    line-height: 1.4;
  }

  .device-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .loading,
  .no-devices {
    text-align: center;
    color: #cccccc;
    padding: 32px;
    font-size: 16px;
  }

  .spinner {
    font-size: 20px;
    margin-right: 8px;
  }

  .no-devices-icon {
    font-size: 24px;
    margin-right: 8px;
  }

  .device-item {
    border-radius: 8px;
    overflow: hidden;
    transition: all 0.2s ease;
  }

  .device-item.selected {
    background: #007acc;
  }

  .device-item.disabled {
    opacity: 0.6;
  }

  .device-button {
    width: 100%;
    background: transparent;
    border: 1px solid #555;
    padding: 16px;
    cursor: pointer;
    text-align: left;
    color: #ffffff;
    border-radius: 8px;
    transition: all 0.2s ease;
  }

  .device-button:hover:not(:disabled) {
    background: #3a3a3a;
    border-color: #777;
  }

  .device-button:disabled {
    cursor: not-allowed;
  }

  .device-item.selected .device-button {
    background: #007acc;
    border-color: #007acc;
  }

  .device-item.selected .device-button:hover:not(:disabled) {
    background: #0066aa;
    border-color: #0066aa;
  }

  .device-info {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .device-header {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .device-icon {
    font-size: 20px;
  }

  .device-name {
    font-weight: 600;
    font-size: 16px;
    flex: 1;
  }

  .default-badge,
  .current-badge {
    background: #28a745;
    color: #ffffff;
    padding: 2px 8px;
    border-radius: 12px;
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
  }

  .current-badge {
    background: #ffc107;
    color: #000000;
  }

  .device-details {
    display: flex;
    gap: 16px;
    flex-wrap: wrap;
  }

  .detail {
    font-size: 14px;
    color: #cccccc;
    background: rgba(255, 255, 255, 0.1);
    padding: 4px 8px;
    border-radius: 4px;
  }

  .device-item.selected .detail {
    color: #e6f3ff;
    background: rgba(255, 255, 255, 0.2);
  }
</style>
