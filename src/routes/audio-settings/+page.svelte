<script lang="ts">
  import AudioSettings from '$lib/components/AudioSettings.svelte';
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';

  let showNotification = false;
  let notificationMessage = '';
  let notificationType = 'info';

  onMount(async () => {
    // Listen for audio device events
    const unlistenDeviceDisconnected = await listen('current-audio-device-disconnected', event => {
      const deviceName = event.payload as string;
      showNotification = true;
      notificationMessage = `Audio device "${deviceName}" was disconnected. Please select a new device.`;
      notificationType = 'error';
      setTimeout(() => {
        showNotification = false;
      }, 5000);
    });

    const unlistenDeviceSelectionRequired = await listen('audio-device-selection-required', () => {
      showNotification = true;
      notificationMessage = 'Please select an audio output device from the settings below.';
      notificationType = 'warning';
      setTimeout(() => {
        showNotification = false;
      }, 8000);
    });

    const unlistenDeviceChanged = await listen('audio-device-changed-during-playback', event => {
      const deviceName = event.payload as string;
      showNotification = true;
      notificationMessage = `Audio output switched to "${deviceName}". Playback may resume shortly.`;
      notificationType = 'info';
      setTimeout(() => {
        showNotification = false;
      }, 3000);
    });

    return () => {
      unlistenDeviceDisconnected();
      unlistenDeviceSelectionRequired();
      unlistenDeviceChanged();
    };
  });

  function closeNotification() {
    showNotification = false;
  }
</script>

<svelte:head>
  <title>Audio Settings - Sound Stitch</title>
</svelte:head>

<main class="settings-page">
  <div class="container">
    <header class="page-header">
      <h1>Audio Settings</h1>
      <p class="page-description">
        Configure your audio output devices and playback settings for the best experience.
      </p>
    </header>

    <!-- Notification -->
    {#if showNotification}
      <div class="notification {notificationType}" role="alert">
        <div class="notification-content">
          <span class="notification-icon">
            {#if notificationType === 'error'}❌
            {:else if notificationType === 'warning'}⚠️
            {:else if notificationType === 'info'}ℹ️
            {:else}✅{/if}
          </span>
          <span class="notification-message">{notificationMessage}</span>
          <button class="notification-close" on:click={closeNotification}>×</button>
        </div>
      </div>
    {/if}

    <!-- Audio Settings Component -->
    <AudioSettings />

    <!-- Usage Information -->
    <section class="usage-info">
      <h2>How it works</h2>
      <div class="info-grid">
        <div class="info-card">
          <div class="info-icon">🔄</div>
          <h3>Automatic Device Switching</h3>
          <p>
            When "Follow system default" is enabled, the application will automatically switch to
            your system's default audio device when it changes.
          </p>
        </div>

        <div class="info-card">
          <div class="info-icon">🎯</div>
          <h3>Manual Device Selection</h3>
          <p>
            Choose a specific audio device for precise control over your audio output. This disables
            automatic switching.
          </p>
        </div>

        <div class="info-card">
          <div class="info-icon">🔧</div>
          <h3>Seamless Playback</h3>
          <p>
            Device changes during audio playback are handled gracefully with minimal interruption to
            your listening experience.
          </p>
        </div>

        <div class="info-card">
          <div class="info-icon">📊</div>
          <h3>Device Monitoring</h3>
          <p>
            The application continuously monitors your audio devices and will notify you if your
            selected device becomes unavailable.
          </p>
        </div>
      </div>
    </section>
  </div>
</main>

<style>
  .settings-page {
    min-height: 100vh;
    background: linear-gradient(135deg, #0f0f0f 0%, #1a1a1a 100%);
    color: #ffffff;
    font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
  }

  .container {
    max-width: 1200px;
    margin: 0 auto;
    padding: 2rem;
  }

  .page-header {
    text-align: center;
    margin-bottom: 3rem;
  }

  .page-header h1 {
    font-size: 3rem;
    font-weight: 700;
    margin: 0 0 1rem 0;
    background: linear-gradient(135deg, #ffffff 0%, #cccccc 100%);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
  }

  .page-description {
    font-size: 1.2rem;
    color: #cccccc;
    margin: 0;
    max-width: 600px;
    margin-left: auto;
    margin-right: auto;
    line-height: 1.6;
  }

  .notification {
    margin-bottom: 2rem;
    border-radius: 8px;
    padding: 0;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
    animation: slideIn 0.3s ease-out;
  }

  .notification.info {
    background: #17a2b8;
  }

  .notification.warning {
    background: #ffc107;
  }

  .notification.error {
    background: #dc3545;
  }

  .notification-content {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 16px;
  }

  .notification-icon {
    font-size: 20px;
    flex-shrink: 0;
  }

  .notification-message {
    flex: 1;
    font-weight: 500;
    color: #ffffff;
  }

  .notification-close {
    background: transparent;
    border: none;
    color: #ffffff;
    font-size: 24px;
    cursor: pointer;
    padding: 0;
    width: 30px;
    height: 30px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.2s ease;
  }

  .notification-close:hover {
    background: rgba(255, 255, 255, 0.2);
  }

  .usage-info {
    margin-top: 3rem;
  }

  .usage-info h2 {
    font-size: 2rem;
    font-weight: 600;
    margin-bottom: 2rem;
    text-align: center;
  }

  .info-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: 2rem;
  }

  .info-card {
    background: #2a2a2a;
    border-radius: 12px;
    padding: 2rem;
    text-align: center;
    border: 1px solid #3a3a3a;
    transition:
      transform 0.2s ease,
      box-shadow 0.2s ease;
  }

  .info-card:hover {
    transform: translateY(-4px);
    box-shadow: 0 8px 25px rgba(0, 0, 0, 0.3);
  }

  .info-icon {
    font-size: 3rem;
    margin-bottom: 1rem;
  }

  .info-card h3 {
    font-size: 1.3rem;
    font-weight: 600;
    margin: 0 0 1rem 0;
    color: #ffffff;
  }

  .info-card p {
    color: #cccccc;
    line-height: 1.6;
    margin: 0;
  }

  @keyframes slideIn {
    from {
      opacity: 0;
      transform: translateY(-20px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  /* Responsive design */
  @media (max-width: 768px) {
    .container {
      padding: 1rem;
    }

    .page-header h1 {
      font-size: 2.5rem;
    }

    .info-grid {
      grid-template-columns: 1fr;
      gap: 1.5rem;
    }

    .info-card {
      padding: 1.5rem;
    }
  }
</style>
