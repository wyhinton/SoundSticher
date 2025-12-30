<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { slide } from 'svelte/transition';

  interface AudioDeviceStatus {
    currentDevice: string | null;
    isFollowingSystem: boolean;
    isPlaying: boolean;
    deviceCount: number;
  }

  let status: AudioDeviceStatus = {
    currentDevice: null,
    isFollowingSystem: true,
    isPlaying: false,
    deviceCount: 0,
  };

  let expanded = false;
  let lastUpdate = new Date();

  onMount(async () => {
    await updateStatus();

    // Set up periodic status updates
    const interval = setInterval(updateStatus, 5000);

    // Listen for real-time events
    const unlistenDeviceChanged = await listen('audio-device-changed', async () => {
      await updateStatus();
    });

    const unlistenFollowSystemChanged = await listen('follow-system-changed', async () => {
      await updateStatus();
    });

    const unlistenDevicesUpdated = await listen('audio-devices-updated', async (event: any) => {
      status.deviceCount = event.payload.length;
      lastUpdate = new Date();
    });

    return () => {
      clearInterval(interval);
      unlistenDeviceChanged();
      unlistenFollowSystemChanged();
      unlistenDevicesUpdated();
    };
  });

  async function updateStatus() {
    try {
      const [currentDevice, isFollowing, devices] = await Promise.all([
        invoke<string | null>('get_current_audio_device'),
        invoke<boolean>('is_following_system_audio'),
        invoke<any[]>('get_audio_devices'),
      ]);

      status = {
        currentDevice,
        isFollowingSystem: isFollowing,
        isPlaying: false, // This would need to be tracked from audio playback
        deviceCount: devices.length,
      };

      lastUpdate = new Date();
    } catch (error) {
      console.error('Failed to update audio device status:', error);
    }
  }

  function getStatusColor(): string {
    if (!status.currentDevice) return '#ff4444'; // Red - no device
    if (status.isFollowingSystem) return '#28a745'; // Green - following system
    return '#007acc'; // Blue - manual device selected
  }

  function getStatusText(): string {
    if (!status.currentDevice) return 'No device selected';
    if (status.isFollowingSystem) return 'Following system default';
    return 'Manual device selection';
  }

  function getStatusIcon(): string {
    if (!status.currentDevice) return '🔇';
    if (status.isFollowingSystem) return '🔄';
    return '🎯';
  }

  function formatUpdateTime(): string {
    const now = new Date();
    const diff = now.getTime() - lastUpdate.getTime();

    if (diff < 10000) return 'Just now';
    if (diff < 60000) return `${Math.floor(diff / 1000)}s ago`;
    if (diff < 3600000) return `${Math.floor(diff / 60000)}m ago`;
    return lastUpdate.toLocaleTimeString();
  }
</script>

<div class="device-status" class:expanded>
  <button class="status-header" on:click={() => (expanded = !expanded)} aria-expanded={expanded}>
    <div class="status-indicator" style="--status-color: {getStatusColor()}">
      <span class="status-icon">{getStatusIcon()}</span>
      <div class="status-info">
        <span class="status-title">Audio Device Status</span>
        <span class="status-subtitle">{getStatusText()}</span>
      </div>
    </div>
    <span class="expand-icon" class:rotated={expanded}>▼</span>
  </button>

  {#if expanded}
    <div class="status-details" transition:slide>
      <div class="detail-grid">
        <div class="detail-item">
          <span class="detail-label">Current Device:</span>
          <span class="detail-value" class:null={!status.currentDevice}>
            {status.currentDevice || 'None selected'}
          </span>
        </div>

        <div class="detail-item">
          <span class="detail-label">Follow System:</span>
          <span class="detail-value" class:enabled={status.isFollowingSystem}>
            {status.isFollowingSystem ? 'Enabled' : 'Disabled'}
          </span>
        </div>

        <div class="detail-item">
          <span class="detail-label">Available Devices:</span>
          <span class="detail-value">
            {status.deviceCount} detected
          </span>
        </div>

        <div class="detail-item">
          <span class="detail-label">Last Updated:</span>
          <span class="detail-value">
            {formatUpdateTime()}
          </span>
        </div>
      </div>

      <div class="status-actions">
        <button class="action-btn" on:click={updateStatus}> 🔄 Refresh Status </button>
        <button class="action-btn" on:click={() => invoke('refresh_audio_devices')}>
          📡 Refresh Devices
        </button>
      </div>
    </div>
  {/if}
</div>

<style>
  .device-status {
    background: #2a2a2a;
    border-radius: 8px;
    border: 1px solid #3a3a3a;
    overflow: hidden;
    transition: all 0.2s ease;
    margin: 8px 0;
  }

  .device-status.expanded {
    border-color: var(--status-color);
    box-shadow: 0 0 0 1px var(--status-color);
  }

  .status-header {
    width: 100%;
    background: transparent;
    border: none;
    padding: 16px;
    cursor: pointer;
    color: #ffffff;
    text-align: left;
    display: flex;
    align-items: center;
    justify-content: space-between;
    transition: background 0.2s ease;
  }

  .status-header:hover {
    background: rgba(255, 255, 255, 0.05);
  }

  .status-indicator {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .status-icon {
    font-size: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 40px;
    height: 40px;
    background: var(--status-color);
    border-radius: 50%;
    color: #ffffff;
  }

  .status-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .status-title {
    font-size: 16px;
    font-weight: 600;
    color: #ffffff;
  }

  .status-subtitle {
    font-size: 14px;
    color: #cccccc;
  }

  .expand-icon {
    font-size: 14px;
    color: #888;
    transition: transform 0.2s ease;
  }

  .expand-icon.rotated {
    transform: rotate(180deg);
  }

  .status-details {
    padding: 0 16px 16px 16px;
    border-top: 1px solid #3a3a3a;
    background: #1e1e1e;
  }

  .detail-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 12px;
    margin: 16px 0;
  }

  .detail-item {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 12px;
    background: #2a2a2a;
    border-radius: 6px;
    border: 1px solid #3a3a3a;
  }

  .detail-label {
    font-size: 12px;
    color: #888;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .detail-value {
    font-size: 14px;
    font-weight: 600;
    color: #ffffff;
  }

  .detail-value.null {
    color: #ff4444;
  }

  .detail-value.enabled {
    color: #28a745;
  }

  .status-actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .action-btn {
    background: #3a3a3a;
    border: 1px solid #555;
    color: #ffffff;
    padding: 8px 12px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 12px;
    font-weight: 500;
    transition: all 0.2s ease;
  }

  .action-btn:hover {
    background: #4a4a4a;
    border-color: #777;
    transform: translateY(-1px);
  }

  .action-btn:active {
    transform: translateY(0);
  }

  /* Responsive design */
  @media (max-width: 768px) {
    .detail-grid {
      grid-template-columns: 1fr;
    }

    .status-actions {
      flex-direction: column;
    }

    .action-btn {
      width: 100%;
      justify-content: center;
    }
  }

  /* Slide transition */
  :global(.slide-enter-active),
  :global(.slide-leave-active) {
    transition: all 0.3s ease;
    overflow: hidden;
  }

  :global(.slide-enter-from),
  :global(.slide-leave-to) {
    max-height: 0;
    padding-top: 0;
    padding-bottom: 0;
    opacity: 0;
  }

  :global(.slide-enter-to),
  :global(.slide-leave-from) {
    max-height: 1000px;
    opacity: 1;
  }
</style>
