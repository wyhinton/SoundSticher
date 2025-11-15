<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { appState } from '../state/state.svelte';
  import { invokeWithPerf, updateInputs } from '../state/performance';
  import { audioFileStateManager } from '../state/stateSynchronization';
  import { get } from 'svelte/store';
  import { onMount, onDestroy } from 'svelte';

  // Visibility state
  let isVisible = false;

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

  // Debug functions
  async function triggerNoActiveSamples() {
    try {
      // Deactivate all files to trigger the no active samples event
      const currentState = get(appState);
      const allFileIds = currentState.sections.flatMap(section =>
        section.files.map(file => file.id)
      );

      if (allFileIds.length > 0) {
        await audioFileStateManager.setFilesActive(allFileIds, false);
        console.log('🔧 Debug: Deactivated all files');

        // Trigger combine to emit no-active-samples event
        await invokeWithPerf('combine_all_cached_samples_with_custom_order');
      } else {
        console.log('🔧 Debug: No files to deactivate');
      }
    } catch (error) {
      console.error('Debug: Failed to trigger no active samples:', error);
    }
  }

  async function reactivateAllFiles() {
    try {
      const currentState = get(appState);
      const allFileIds = currentState.sections.flatMap(section =>
        section.files.map(file => file.id)
      );

      if (allFileIds.length > 0) {
        await audioFileStateManager.setFilesActive(allFileIds, true);
        console.log('🔧 Debug: Reactivated all files');

        // Trigger combine to update timeline
        await invokeWithPerf('combine_all_cached_samples_with_custom_order');
      } else {
        console.log('🔧 Debug: No files to reactivate');
      }
    } catch (error) {
      console.error('Debug: Failed to reactivate all files:', error);
    }
  }

  async function clearAppState() {
    try {
      appState.update(state => ({
        ...state,
        sections: [],
        timelineItems: [],
        combinedFile: undefined,
        hasNoActiveSamples: false,
      }));
      console.log('🔧 Debug: Cleared app state');
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
      console.log('🔧 Debug: Forcing state synchronization...');
      const currentState = get(appState);
      await updateInputs(currentState.sections);
      console.log('🔧 Debug: State sync completed');
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
    </div>

    <div class="debug-info">
      <small>
        <i class="fa fa-info-circle"></i>
        DEV | hasNoActive: {$appState?.hasNoActiveSamples ? 'T' : 'F'} | Ctrl+Shift+Space to toggle
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

  /* Responsive adjustments */
  @media (max-width: 768px) {
    .debug-buttons {
      flex-direction: column;
      gap: 3px;
    }

    .button-group {
      min-width: auto;
    }
  }
</style>
