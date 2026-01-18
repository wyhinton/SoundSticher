<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import {
    appState,
    callSiteTrackingEnabled,
    toggleCallSiteTrackingEnabled,
  } from '../state/state.svelte';
  import { invokeWithPerf, updateInputs } from '../state/performance';
  import { audioFileStateManager } from '../state/stateSynchronization';
  import { loggingState } from '../state/logging';
  import { get } from 'svelte/store';
  import { onMount, onDestroy } from 'svelte';
  import { debugState, customContextMenu } from '../state/debug.svelte';
  import { timelineDebugMode } from '../state/state.svelte';
  import { type DurationResponse } from '$lib/state/durationCache';
  import UndoRedoControls from './UndoRedoControls.svelte';

  // Visibility state
  let isVisible = false;

  // CSS Debug outline state
  let cssOutlineEnabled = false;

  // Duration test state
  let durationTestFilePath = '';
  let durationTestResult: DurationResponse | null = null;
  let durationTestLoading = false;
  let durationTestError: string | null = null;

  // Global keyboard shortcut handler
  function handleGlobalKeydown(event: KeyboardEvent) {
    // Only in development mode
    if (!import.meta.env.DEV) return;

    // Ctrl+Shift+Space to toggle visibility
    if (event.ctrlKey && event.shiftKey && event.code === 'Space') {
      event.preventDefault();
      isVisible = !isVisible;
      console.log('🔧 Debug: Toggled debug toolbar visibility:', isVisible);
    }
  }

  onMount(() => {
    // Add global keyboard listener
    window.addEventListener('keydown', handleGlobalKeydown);
  });

  onDestroy(() => {
    // Remove global keyboard listener
    window.removeEventListener('keydown', handleGlobalKeydown);
  });

  // Debug functions - updated to work with operation-specific sections
  async function triggerNoActiveSamples() {
    try {
      // Note: This function now needs to work with current operation sections
      console.log('🔧 Debug: triggerNoActiveSamples - needs operation context');
      // Implementation depends on having an active operation selected
    } catch (error) {
      console.error('Debug: Failed to trigger no active samples:', error);
    }
  }

  async function reactivateAllFiles() {
    try {
      // Note: This function now needs to work with current operation sections
      console.log('🔧 Debug: reactivateAllFiles - needs operation context');
      // Implementation depends on having an active operation selected
    } catch (error) {
      console.error('Debug: Failed to reactivate all files:', error);
    }
  }

  async function clearAppState() {
    try {
      appState.update(state => ({
        ...state,
        timelineItems: [],
        combinedFile: undefined,
        hasNoActiveSamples: false,
      }));
      console.log('🔧 Debug: Cleared app state (sections now managed per operation)');
    } catch (error) {
      console.error('Debug: Failed to clear app state:', error);
    }
  }

  async function logCurrentState() {
    try {
      const currentState = get(appState);
      console.log('🔧 Debug: Current App State:', currentState);

      // Also log backend state if available
      const customOrder = await invoke('get_custom_order');
      console.log('🔧 Debug: Backend Custom Order:', customOrder);
    } catch (error) {
      console.error('Debug: Failed to log current state:', error);
    }
  }

  async function testCombineFunction() {
    try {
      console.log('🔧 Debug: Testing combine function...');
      const result = await invokeWithPerf('combine_all_cached_samples_with_custom_order');
      console.log('🔧 Debug: Combine result:', result);
    } catch (error) {
      console.error('Debug: Failed to test combine function:', error);
    }
  }

  async function forceStateSync() {
    try {
      console.log('🔧 Debug: Force state sync now requires operation context');
      // Note: updateInputs now needs to be called with operation-specific sections
      // const currentState = get(appState);
      // await updateInputs(currentOperationSections);
      console.log('🔧 Debug: State sync requires active operation');
    } catch (error) {
      console.error('Debug: Failed to force state sync:', error);
    }
  }

  function toggleHasNoActiveSamples() {
    appState.update(state => ({
      ...state,
      hasNoActiveSamples: !state.hasNoActiveSamples,
    }));
    console.log('🔧 Debug: Toggled hasNoActiveSamples to:', get(appState).hasNoActiveSamples);
  }

  async function testEmitNoActiveSamples() {
    try {
      // Manually trigger the no-active-samples event listener
      const event = new CustomEvent('no-active-samples', { detail: {} });
      console.log('🔧 Debug: Manually triggering no-active-samples event');

      // Directly update state to simulate backend event
      appState.update(state => ({
        ...state,
        hasNoActiveSamples: true,
      }));

      console.log('🔧 Debug: hasNoActiveSamples set to true');
    } catch (error) {
      console.error('Debug: Failed to emit no active samples event:', error);
    }
  }

  async function resetNoActiveSamples() {
    try {
      appState.update(state => ({
        ...state,
        hasNoActiveSamples: false,
      }));
      console.log('🔧 Debug: Reset hasNoActiveSamples to false');
    } catch (error) {
      console.error('Debug: Failed to reset no active samples:', error);
    }
  }

  async function resetSortState() {
    try {
      appState.update(state => ({
        ...state,
        sortKey: undefined,
        sortDirection: undefined,
      }));
      console.log('🔧 Debug: Reset sort state - sortKey and sortDirection set to undefined');
    } catch (error) {
      console.error('Debug: Failed to reset sort state:', error);
    }
  }

  // Release management functions
  async function openReleaseGuide() {
    try {
      // Open release guide in default browser or show info
      console.log('🔧 Debug: Opening release guide...');

      // Try to open the RELEASE.md file in VS Code or show instructions
      const releaseInstructions = `
📦 RELEASE INSTRUCTIONS:

🚀 Quick Release Options:
1. Run: scripts/release.bat (Windows) or ./scripts/release.sh (Mac/Linux)
2. Or use: npm run release:[patch|minor|major]

📝 What happens:
- Updates all version files automatically
- Creates git tag and pushes to GitHub  
- Triggers automated builds for Windows, macOS, Linux
- Creates GitHub release with all platform downloads

🎯 Release Types:
- Patch (1.0.0 → 1.0.1): Bug fixes
- Minor (1.0.0 → 1.1.0): New features  
- Major (1.0.0 → 2.0.0): Breaking changes

Monitor progress at: GitHub → Actions tab
      `;

      console.log(releaseInstructions);

      // Also try to show in a simple alert for immediate visibility
      if (typeof window !== 'undefined') {
        alert('Release instructions logged to console. Check DevTools → Console for details.');
      }
    } catch (error) {
      console.error('Debug: Failed to show release guide:', error);
    }
  }

  async function checkReleaseStatus() {
    try {
      console.log('🔧 Debug: Checking release status...');

      // Get current version from package.json context (if available)
      const currentVersion = '0.0.0'; // This would need to be dynamically loaded

      const statusInfo = `
🎵 SOUND STITCH RELEASE STATUS:

Current Version: ${currentVersion}
Project: Sound Stitch (Tauri + SvelteKit)

🏗️ Build Targets:
- Windows x64 (Setup + MSI)
- macOS Universal (DMG)  
- Linux x64 (DEB + AppImage)

🔄 To create new release:
1. Run release script: scripts/release.bat
2. Choose release type
3. Monitor GitHub Actions for build progress
4. Download artifacts from GitHub Releases page

📍 Quick Commands:
- npm run release:patch (bug fixes)
- npm run release:minor (new features)
- npm run release:major (breaking changes)
      `;

      console.log(statusInfo);
    } catch (error) {
      console.error('Debug: Failed to check release status:', error);
    }
  }

  async function simulateReleasePrep() {
    try {
      console.log('🔧 Debug: Simulating release preparation...');

      // This would normally check git status, versions, etc.
      const prepCheck = `
🔍 RELEASE PREPARATION CHECKLIST:

✅ Git repository detected
✅ Version files found:
   - package.json
   - src-tauri/Cargo.toml  
   - src-tauri/tauri.conf.json

✅ GitHub Actions workflow configured
✅ Cross-platform build setup ready

🚀 Ready to release! Run:
   scripts/release.bat (Windows)
   ./scripts/release.sh (Mac/Linux)
      `;

      console.log(prepCheck);

      // Simulate version sync check
      console.log('🔧 Debug: Version sync check completed');
    } catch (error) {
      console.error('Debug: Failed to simulate release prep:', error);
    }
  }

  // Logging toggle functions
  // Dynamic logging categories configuration
  const loggingCategories = [
    {
      key: 'groupsLog' as keyof typeof $loggingState,
      label: 'Groups Log',
      icon: 'fa-layer-group',
      title: 'Toggle Groups system logging',
    },
    {
      key: 'selectionLog' as keyof typeof $loggingState,
      label: 'Selection Log',
      icon: 'fa-mouse-pointer',
      title: 'Toggle Selection system logging',
    },
    {
      key: 'dragdropLog' as keyof typeof $loggingState,
      label: 'DragDrop Log',
      icon: 'fa-arrows-alt',
      title: 'Toggle Drag & Drop logging',
    },
    // Add future categories here:
    // {
    //   key: 'performanceLog' as keyof typeof $loggingState,
    //   label: 'Perf Log',
    //   icon: 'fa-tachometer-alt',
    //   title: 'Toggle Performance logging'
    // },
    // {
    //   key: 'audioLog' as keyof typeof $loggingState,
    //   label: 'Audio Log',
    //   icon: 'fa-volume-up',
    //   title: 'Toggle Audio logging'
    // },
    // {
    //   key: 'uiLog' as keyof typeof $loggingState,
    //   label: 'UI Log',
    //   icon: 'fa-mouse-pointer',
    //   title: 'Toggle UI logging'
    // }
  ];

  function toggleLogging(categoryKey: keyof typeof $loggingState) {
    loggingState.update(state => ({
      ...state,
      [categoryKey]: !state[categoryKey],
    }));
    const newState = get(loggingState);
    console.log(`🔧 Debug: ${categoryKey} toggled to:`, newState[categoryKey]);
  }

  // Toggle call site tracking for performance metrics
  function toggleCallSiteTracking() {
    toggleCallSiteTrackingEnabled();
    const newValue = get(callSiteTrackingEnabled);
    console.log(`🔧 Debug: Call site tracking toggled to:`, newValue);
  }

  // Keep the specific function for backward compatibility, but use the generic one
  function toggleGroupsLogging() {
    toggleLogging('groupsLog');
  }

  // Duration test function
  async function testGetDuration() {
    if (!durationTestFilePath.trim()) {
      durationTestError = 'Please enter a file path';
      return;
    }

    durationTestLoading = true;
    durationTestError = null;
    durationTestResult = null;

    try {
      const result = await invokeWithPerf<DurationResponse>('get_duration', {
        request: {
          path: durationTestFilePath,
        },
      });

      console.log('🔧 Debug: Duration test result:', durationTestResult);
    } catch (error) {
      durationTestError = error instanceof Error ? error.message : String(error);
      durationTestLoading = false;
      console.error('🔧 Debug: Duration test error:', error);
    } finally {
      durationTestLoading = false;
    }
  }

  // Handle Enter key in duration input
  function handleDurationInputKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter') {
      event.preventDefault();
      testGetDuration();
    }
  }

  // MCP Bridge API testing functions
  async function testStartIPCMonitor() {
    try {
      await invoke('plugin:mcp-bridge|start_ipc_monitor');
      console.log('🔧 MCP: IPC monitoring started');
    } catch (error) {
      console.error('MCP: Failed to start IPC monitor:', error);
    }
  }

  async function testStopIPCMonitor() {
    try {
      await invoke('plugin:mcp-bridge|stop_ipc_monitor');
      console.log('🔧 MCP: IPC monitoring stopped');
    } catch (error) {
      console.error('MCP: Failed to stop IPC monitor:', error);
    }
  }

  async function testGetIPCEvents() {
    try {
      const events = await invoke('plugin:mcp-bridge|get_ipc_events');
      console.log('🔧 MCP: IPC Events:', events);
    } catch (error) {
      console.error('MCP: Failed to get IPC events:', error);
    }
  }

  async function testGetWindowInfo() {
    try {
      const windowInfo = await invoke('plugin:mcp-bridge|get_window_info');
      console.log('🔧 MCP: Window Info:', windowInfo);
    } catch (error) {
      console.error('MCP: Failed to get window info:', error);
    }
  }

  async function testGetBackendState() {
    try {
      const state = await invoke('plugin:mcp-bridge|get_backend_state');
      console.log('🔧 MCP: Backend State:', state);
    } catch (error) {
      console.error('MCP: Failed to get backend state:', error);
    }
  }

  async function testEmitCustomEvent() {
    try {
      await invoke('plugin:mcp-bridge|emit_event', {
        eventName: 'debug-custom-event',
        payload: {
          timestamp: new Date().toISOString(),
          message: 'Debug test event from toolbar',
          data: { test: true },
        },
      });
      console.log('🔧 MCP: Custom event emitted: debug-custom-event');
    } catch (error) {
      console.error('MCP: Failed to emit custom event:', error);
    }
  }

  async function testExecuteCommand() {
    try {
      // Test with a simple command that should exist
      const result = await invoke('plugin:mcp-bridge|execute_command', {
        command: 'get_app_state',
      });
      console.log('🔧 MCP: Command execution result:', result);
    } catch (error) {
      console.error('MCP: Failed to execute command:', error);
    }
  }

  async function testListDevices() {
    try {
      const devices = await invoke('plugin:mcp-bridge|list_devices');
      console.log('🔧 MCP: Mobile Devices:', devices);
    } catch (error) {
      console.error('MCP: Failed to list devices:', error);
    }
  }

  async function testWebViewScreenshot() {
    try {
      const screenshot = await invoke('plugin:mcp-bridge|webview_screenshot', {
        format: 'png',
        maxWidth: 800,
      });
      console.log('🔧 MCP: Screenshot captured (base64 length):', screenshot.length);
      // Optionally display or download the screenshot
    } catch (error) {
      console.error('MCP: Failed to take screenshot:', error);
    }
  }

  async function testWebViewExecuteJS() {
    try {
      const result = await invoke('plugin:mcp-bridge|webview_execute_js', {
        script: 'document.title',
      });
      console.log('🔧 MCP: JS execution result (document.title):', result);
    } catch (error) {
      console.error('MCP: Failed to execute JS:', error);
    }
  }

  async function testWebViewFindElement() {
    try {
      const element = await invoke('plugin:mcp-bridge|webview_find_element', {
        selector: 'body',
        strategy: 'css',
      });
      console.log('🔧 MCP: Found element:', element);
    } catch (error) {
      console.error('MCP: Failed to find element:', error);
    }
  }

  async function testReadConsoleLogs() {
    try {
      const logs = await invoke('plugin:mcp-bridge|read_logs', {
        source: 'console',
        lines: 10,
      });
      console.log('🔧 MCP: Console logs:', logs);
    } catch (error) {
      console.error('MCP: Failed to read console logs:', error);
    }
  }

  // Toggle CSS debug outlines
  function toggleCssOutlines() {
    cssOutlineEnabled = !cssOutlineEnabled;

    if (cssOutlineEnabled) {
      // Add the debug outline styles to the document
      const style = document.createElement('style');
      style.id = 'debug-outlines';
      style.textContent = `
        * {
          outline: 4px solid rgba(255,0,0,1) !important;
        }
      `;
      document.head.appendChild(style);
      console.log('🔧 Debug: CSS outlines enabled');
    } else {
      // Remove the debug outline styles
      const existingStyle = document.getElementById('debug-outlines');
      if (existingStyle) {
        existingStyle.remove();
      }
      console.log('🔧 Debug: CSS outlines disabled');
    }
  }
</script>

{#if isVisible}
  <div class="debug-toolbar">
    <div class="debug-header">
      <span class="debug-title">
        <i class="fa fa-bug"></i>
        Debug
      </span>
      <button
        class="btn-close"
        on:click={() => (isVisible = false)}
        title="Hide (Ctrl+Shift+Space)"
        aria-label="Hide Debug Toolbar"
      >
        <i class="fa fa-times"></i>
      </button>
    </div>

    <div class="debug-buttons">
      <!-- Undo/Redo Controls -->
      <div class="button-group">
        <span class="group-title">Undo/Redo</span>
        <UndoRedoControls />
      </div>

      <!-- ...existing code... -->
      <div class="button-group">
        <span class="group-title">State</span>
        <button class="btn btn-xs btn-outline-warning" on:click={triggerNoActiveSamples}>
          <i class="fa fa-times"></i>
          No Active
        </button>
        <button class="btn btn-xs btn-outline-success" on:click={reactivateAllFiles}>
          <i class="fa fa-check"></i>
          Reactivate
        </button>
        <button class="btn btn-xs btn-outline-info" on:click={toggleHasNoActiveSamples}>
          <i class="fa fa-toggle-on"></i>
          Toggle
        </button>
        <button class="btn btn-xs btn-outline-warning" on:click={testEmitNoActiveSamples}>
          <i class="fa fa-exclamation-triangle"></i>
          Set No Active
        </button>
        <button class="btn btn-xs btn-outline-success" on:click={resetNoActiveSamples}>
          <i class="fa fa-check-circle"></i>
          Reset
        </button>
        <button class="btn btn-xs btn-outline-secondary" on:click={resetSortState}>
          <i class="fa fa-sort"></i>
          Reset Sort
        </button>
      </div>

      <div class="button-group">
        <span class="group-title">Backend</span>
        <button class="btn btn-xs btn-outline-primary" on:click={testCombineFunction}>
          <i class="fa fa-play"></i>
          Combine
        </button>
        <button class="btn btn-xs btn-outline-secondary" on:click={forceStateSync}>
          <i class="fa fa-sync"></i>
          Sync
        </button>
      </div>

      <div class="button-group">
        <span class="group-title">Debug</span>
        <button class="btn btn-xs btn-outline-info" on:click={logCurrentState}>
          <i class="fa fa-list"></i>
          Log State
        </button>
        <button class="btn btn-xs btn-outline-danger" on:click={clearAppState}>
          <i class="fa fa-trash"></i>
          Clear
        </button>
        <button
          class="btn btn-xs"
          class:btn-outline-secondary={!cssOutlineEnabled}
          class:btn-secondary={cssOutlineEnabled}
          on:click={toggleCssOutlines}
          title="Toggle CSS debug outlines for all elements"
        >
          <i class="fa fa-border-all"></i>
          CSS Outlines
        </button>
      </div>

      <div class="button-group">
        <span class="group-title">Timeline</span>
        <button
          class="btn btn-xs"
          class:btn-outline-success={!$timelineDebugMode}
          class:btn-success={$timelineDebugMode}
          on:click={timelineDebugMode.toggle}
        >
          <i class="fa fa-bug"></i>
          Timeline Debug
        </button>
      </div>

      <div class="button-group">
        <span class="group-title">Context Menu</span>
        <button
          class="btn btn-xs"
          class:btn-outline-primary={!$debugState.useCustomContextMenu}
          class:btn-primary={$debugState.useCustomContextMenu}
          on:click={customContextMenu.toggle}
        >
          <i class="fa fa-mouse-pointer"></i>
          Custom Menu
        </button>
      </div>

      <div class="button-group">
        <span class="group-title">Logging</span>
        {#each loggingCategories as category}
          <button
            class="btn btn-xs"
            class:btn-outline-info={!$loggingState[category.key]}
            class:btn-info={$loggingState[category.key]}
            on:click={() => toggleLogging(category.key)}
            title={category.title}
          >
            <i class="fa {category.icon}"></i>
            {category.label}
          </button>
        {/each}
        <button
          class="btn btn-xs"
          class:btn-outline-warning={!$callSiteTrackingEnabled}
          class:btn-warning={$callSiteTrackingEnabled}
          on:click={toggleCallSiteTracking}
          title="Toggle call site tracking for performance metrics"
        >
          <i class="fa fa-map-marker"></i>
          Call Sites
        </button>
      </div>

      <div class="button-group">
        <span class="group-title">Release</span>
        <button class="btn btn-xs btn-outline-success" on:click={openReleaseGuide}>
          <i class="fa fa-rocket"></i>
          Guide
        </button>
        <button class="btn btn-xs btn-outline-info" on:click={checkReleaseStatus}>
          <i class="fa fa-info-circle"></i>
          Status
        </button>
        <button class="btn btn-xs btn-outline-warning" on:click={simulateReleasePrep}>
          <i class="fa fa-cog"></i>
          Prep Check
        </button>
      </div>

      <div class="button-group">
        <span class="group-title">Duration Test</span>
        <div class="duration-test-input-group">
          <input
            type="text"
            class="duration-test-input"
            placeholder="File path..."
            bind:value={durationTestFilePath}
            on:keydown={handleDurationInputKeydown}
          />
          <button class="btn btn-xs btn-outline-primary" on:click={testGetDuration}>
            <i class="fa fa-clock-o"></i>
            {durationTestLoading ? 'Loading...' : 'Get Duration'}
          </button>
        </div>
        {#if durationTestError}
          <div class="duration-test-error">
            <i class="fa fa-exclamation-circle"></i>
            {durationTestError}
          </div>
        {/if}
        {#if durationTestResult}
          <div class="duration-test-result">
            <div class="result-item">
              <span class="result-label">Path:</span>
              <span class="result-value">{durationTestResult.path}</span>
            </div>
            <div class="result-item">
              <span class="result-label">Duration:</span>
              <span class="result-value">
                {durationTestResult.durationSeconds !== null
                  ? `${durationTestResult.durationSeconds.toFixed(2)}s`
                  : 'N/A'}
              </span>
            </div>
            <div class="result-item">
              <span class="result-label">Cache Hit:</span>
              <span class="result-value" class:cache-hit={durationTestResult.cacheHit}>
                {durationTestResult.cacheHit ? 'Yes ✓' : 'No (computed)'}
              </span>
            </div>
          </div>
        {/if}
      </div>

      <div class="button-group">
        <span class="group-title">MCP APIs</span>

        <!-- IPC Monitoring -->
        <div class="mcp-subgroup">
          <span class="subgroup-title">IPC Monitor</span>
          <div class="subgroup-buttons">
            <button class="btn btn-xs btn-outline-success" on:click={testStartIPCMonitor}>
              <i class="fa fa-play"></i>
              Start
            </button>
            <button class="btn btn-xs btn-outline-danger" on:click={testStopIPCMonitor}>
              <i class="fa fa-stop"></i>
              Stop
            </button>
            <button class="btn btn-xs btn-outline-info" on:click={testGetIPCEvents}>
              <i class="fa fa-list"></i>
              Events
            </button>
          </div>
        </div>

        <!-- Backend State & Window Info -->
        <div class="mcp-subgroup">
          <span class="subgroup-title">State & Window</span>
          <div class="subgroup-buttons">
            <button class="btn btn-xs btn-outline-primary" on:click={testGetBackendState}>
              <i class="fa fa-server"></i>
              Backend
            </button>
            <button class="btn btn-xs btn-outline-secondary" on:click={testGetWindowInfo}>
              <i class="fa fa-window-maximize"></i>
              Window
            </button>
          </div>
        </div>

        <!-- WebView Automation -->
        <div class="mcp-subgroup">
          <span class="subgroup-title">WebView</span>
          <div class="subgroup-buttons">
            <button class="btn btn-xs btn-outline-warning" on:click={testWebViewScreenshot}>
              <i class="fa fa-camera"></i>
              Screenshot
            </button>
            <button class="btn btn-xs btn-outline-info" on:click={testWebViewExecuteJS}>
              <i class="fa fa-code"></i>
              Exec JS
            </button>
            <button class="btn btn-xs btn-outline-secondary" on:click={testWebViewFindElement}>
              <i class="fa fa-search"></i>
              Find El
            </button>
          </div>
        </div>

        <!-- Events & Commands -->
        <div class="mcp-subgroup">
          <span class="subgroup-title">Events & Commands</span>
          <div class="subgroup-buttons">
            <button class="btn btn-xs btn-outline-success" on:click={testEmitCustomEvent}>
              <i class="fa fa-broadcast-tower"></i>
              Emit
            </button>
            <button class="btn btn-xs btn-outline-primary" on:click={testExecuteCommand}>
              <i class="fa fa-terminal"></i>
              Command
            </button>
          </div>
        </div>

        <!-- Mobile & Logs -->
        <div class="mcp-subgroup">
          <span class="subgroup-title">Mobile & Logs</span>
          <div class="subgroup-buttons">
            <button class="btn btn-xs btn-outline-info" on:click={testListDevices}>
              <i class="fa fa-mobile"></i>
              Devices
            </button>
            <button class="btn btn-xs btn-outline-secondary" on:click={testReadConsoleLogs}>
              <i class="fa fa-file-text"></i>
              Logs
            </button>
          </div>
        </div>
      </div>
    </div>

    <div class="debug-info">
      <small>
        <i class="fa fa-info-circle"></i>
        DEV | hasNoActive: {$appState?.hasNoActiveSamples ? 'T' : 'F'} | Timeline Debug: {$timelineDebugMode
          ? 'ON'
          : 'OFF'} | Custom Menu: {$debugState.useCustomContextMenu ? 'ON' : 'OFF'} | Call Sites: {$callSiteTrackingEnabled
          ? 'ON'
          : 'OFF'} | CSS Outlines: {cssOutlineEnabled ? 'ON' : 'OFF'} |
        {#each loggingCategories as category}
          {category.label}: {$loggingState[category.key] ? 'ON' : 'OFF'} |
        {/each}
        Ctrl+Shift+Space to toggle
      </small>
    </div>
  </div>
{:else}
  <!-- Hidden toolbar - show toggle button -->
  <div class="debug-toggle-hidden">
    <button
      class="btn-toggle"
      on:click={() => (isVisible = true)}
      title="Show Debug Toolbar (Ctrl+Shift+Space)"
      aria-label="Show Debug Toolbar"
    >
      <i class="fa fa-bug"></i>
    </button>
  </div>
{/if}

<style>
  .debug-toolbar {
    background: var(--bs-dark);
    border: 1px solid var(--bs-warning);
    border-radius: 4px;
    padding: 4px 6px;
    margin: 2px 0;
    box-shadow: 0 1px 3px rgba(255, 193, 7, 0.2);
    font-size: 10px;
  }

  .debug-header .debug-title {
    color: var(--bs-warning);
    font-weight: 600;
    font-size: 11px;
    margin: 0;
  }

  .debug-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 4px;
  }

  .btn-close {
    background: none;
    border: none;
    color: var(--bs-secondary);
    padding: 0;
    width: 14px;
    height: 14px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 2px;
    transition: all 0.15s ease;
    cursor: pointer;
  }

  .btn-close:hover {
    color: var(--bs-warning);
    background: rgba(255, 193, 7, 0.1);
  }

  .btn-close i {
    font-size: 8px;
  }

  .debug-toggle-hidden {
    position: fixed;
    bottom: 10px;
    right: 10px;
    z-index: 1000;
  }

  .btn-toggle {
    background: var(--bs-dark);
    border: 1px solid var(--bs-warning);
    color: var(--bs-warning);
    width: 24px;
    height: 24px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: all 0.2s ease;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
  }

  .btn-toggle:hover {
    transform: scale(1.1);
    box-shadow: 0 4px 12px rgba(255, 193, 7, 0.3);
  }

  .btn-toggle i {
    font-size: 10px;
  }

  .debug-buttons {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin: 4px 0;
  }

  .button-group {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 80px;
  }

  .group-title {
    color: var(--bs-light);
    font-size: 9px;
    font-weight: 600;
    margin: 0 0 2px 0;
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }

  .btn-xs {
    font-size: 9px;
    padding: 1px 4px;
    border-radius: 2px;
    transition: all 0.15s ease;
    line-height: 1.2;
    min-height: 16px;
    white-space: nowrap;
  }

  .btn-xs i {
    font-size: 8px;
    margin-right: 2px;
  }

  .btn-xs:hover {
    transform: translateY(-0.5px);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.15);
  }

  .debug-info {
    border-top: 1px solid var(--bs-secondary);
    padding-top: 3px;
    margin-top: 4px;
  }

  .debug-info small {
    font-size: 9px;
    color: var(--bs-secondary);
  }

  .debug-info i {
    font-size: 8px;
    margin-right: 2px;
  }

  /* Duration test styles */
  .duration-test-input-group {
    display: flex;
    gap: 2px;
    align-items: center;
  }

  .duration-test-input {
    flex: 1;
    font-size: 9px;
    padding: 2px 4px;
    border: 1px solid var(--bs-secondary);
    border-radius: 2px;
    background: var(--bs-dark);
    color: var(--bs-light);
    min-width: 150px;
  }

  .duration-test-input:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .duration-test-input::placeholder {
    color: var(--bs-secondary);
  }

  .duration-test-error {
    background: rgba(220, 53, 69, 0.1);
    border: 1px solid rgba(220, 53, 69, 0.5);
    color: #ff6b6b;
    padding: 3px 4px;
    border-radius: 2px;
    font-size: 9px;
    margin-top: 2px;
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .duration-test-error i {
    font-size: 8px;
  }

  .duration-test-result {
    background: rgba(40, 167, 69, 0.1);
    border: 1px solid rgba(40, 167, 69, 0.5);
    padding: 3px 4px;
    border-radius: 2px;
    font-size: 9px;
    margin-top: 2px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .result-item {
    display: flex;
    gap: 4px;
    align-items: center;
  }

  .result-label {
    color: var(--bs-secondary);
    font-weight: 600;
    min-width: 80px;
  }

  .result-value {
    color: var(--bs-light);
    font-family: 'Courier New', monospace;
  }

  .result-value.cache-hit {
    color: #51cf66;
  }

  /* MCP API styles */
  .mcp-subgroup {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin-bottom: 3px;
  }

  .subgroup-title {
    color: var(--bs-info);
    font-size: 8px;
    font-weight: 600;
    margin: 0 0 1px 0;
    text-transform: uppercase;
    letter-spacing: 0.2px;
    opacity: 0.8;
  }

  .subgroup-buttons {
    display: flex;
    gap: 1px;
    flex-wrap: wrap;
  }

  .subgroup-buttons .btn-xs {
    font-size: 8px;
    padding: 1px 3px;
    min-height: 14px;
    flex: 1;
    min-width: 45px;
  }

  .subgroup-buttons .btn-xs i {
    font-size: 7px;
    margin-right: 1px;
  }

  /* Responsive adjustments */
  @media (max-width: 768px) {
    .debug-buttons {
      flex-direction: column;
      gap: 3px;
    }

    .button-group {
      min-width: auto;
    }

    .duration-test-input-group {
      flex-direction: column;
    }

    .duration-test-input {
      min-width: auto;
      width: 100%;
    }
  }
</style>
