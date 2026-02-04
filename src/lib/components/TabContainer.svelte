<script lang="ts">
  export let activeTab: string = '';
  export let tabs: Array<{
    id: string;
    label: string;
    icon?: string;
  }> = [];
  export let onTabChange: (tabId: string) => void = () => {};
  export let contentHeight: number = 500;
  export let resizable: boolean = false;
  export let onHeightChange: (height: number) => void = () => {};
  export let minHeight: number = 80;
  export let maxHeight: number = 800;

  let isResizing: boolean = false;

  function handleTabClick(tabId: string) {
    activeTab = tabId;
    onTabChange(tabId);
  }

  function handleResizeStart(event: MouseEvent) {
    if (!resizable) return;

    event.preventDefault();
    isResizing = true;

    const startY = event.clientY;
    const startHeight = contentHeight;

    function handleMouseMove(e: MouseEvent) {
      if (!isResizing) return;

      const deltaY = e.clientY - startY;
      const newHeight = Math.max(minHeight, Math.min(maxHeight, startHeight + deltaY));
      contentHeight = newHeight;
      onHeightChange(newHeight);
    }

    function handleMouseUp() {
      isResizing = false;
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
    }

    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);
  }
</script>

<div class="tab-container">
  <!-- Tab navigation -->
  <div class="tab-navigation" role="tablist" aria-label="Tab navigation">
    {#each tabs as tab (tab.id)}
      <button
        class="tab"
        class:active={activeTab === tab.id}
        onclick={() => handleTabClick(tab.id)}
        role="tab"
        aria-selected={activeTab === tab.id}
        aria-controls="{tab.id}-tab-panel"
      >
        {#if tab.icon}
          <i class="fa {tab.icon}"></i>
        {/if}
        {tab.label}
      </button>
    {/each}
  </div>

  <!-- Tab content -->
  <div class="tab-content" style="height: {contentHeight}px;">
    <div
      class="tab-panel"
      id="{activeTab}-tab-panel"
      role="tabpanel"
      aria-labelledby="{activeTab}-tab"
    >
      {#if activeTab === 'frontend'}
        <slot name="frontend" />
      {:else if activeTab === 'backend'}
        <slot name="backend" />
      {:else if activeTab === 'performance'}
        <slot name="performance" />
      {:else if activeTab === 'export'}
        <slot name="export" />
      {:else if activeTab === 'debug'}
        <slot name="debug" />
      {:else if activeTab === 'logging'}
        <slot name="logging" />
      {:else if activeTab === 'listeners'}
        <slot name="listeners" />
      {:else if activeTab === 'invoke-history'}
        <slot name="invoke-history" />
      {:else if activeTab === 'selection'}
        <slot name="selection" />
      {:else if activeTab === 'artifacts'}
        <slot name="artifacts" />
      {:else if activeTab === 'undo-redo'}
        <slot name="undo-redo" />
      {:else if activeTab === 'timeline-stores'}
        <slot name="timeline-stores" />
      {:else}
        <slot {activeTab} />
      {/if}
    </div>
  </div>

  <!-- Resize handle -->
  {#if resizable}
    <button
      class="resize-handle"
      class:resizing={isResizing}
      onmousedown={handleResizeStart}
      aria-label="Resize tab content"
      type="button"
    >
      <div class="resize-indicator"></div>
    </button>
  {/if}
</div>

<style>
  .tab-container {
    background-color: rgba(0, 0, 0, 0.3);
    border-radius: 8px;
    overflow: hidden;
    border: 1px solid rgba(255, 255, 255, 0.1);
  }

  /* Tab navigation */
  .tab-navigation {
    display: flex;
    flex-direction: row;
    flex-wrap: wrap;
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
    gap: 2px;
    padding: 0 2px;
    align-items: flex-end;
  }

  .tab {
    padding: 4px 8px;
    cursor: pointer;
    position: relative;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-bottom: none;
    color: rgba(255, 255, 255, 0.7);
    font-size: 12px;
    font-weight: 500;
    transition: all 0.2s ease;
    border-radius: 6px 6px 0 0;
    margin-top: 4px;
    min-width: 80px;
    max-width: 200px;
    display: flex;
    align-items: center;
    gap: 6px;
    justify-content: center;
    flex-shrink: 0;
    white-space: nowrap;
  }

  .tab:hover {
    background-color: rgba(255, 255, 255, 0.1);
    color: rgba(255, 255, 255, 0.9);
    border-color: rgba(255, 255, 255, 0.2);
  }

  .tab.active {
    color: #f59e0b;
    font-weight: 600;
    background-color: rgba(245, 158, 11, 0.2);
    border-color: rgba(245, 158, 11, 0.5);
    margin-top: 0;
    padding-top: 12px;
    z-index: 1;
    position: relative;
  }

  .tab.active::after {
    content: '';
    position: absolute;
    bottom: -1px;
    left: 0;
    right: 0;
    height: 1px;
    background: rgba(245, 158, 11, 0.2);
  }

  .tab i {
    font-size: 11px;
  }

  /* Tab content */
  .tab-content {
    background-color: rgba(0, 0, 0, 0.3);
    border-top: none;
    border-radius: 0 0 8px 8px;
    overflow-y: auto;
    min-height: 80px;
  }

  .tab-panel {
    color: rgba(255, 255, 255, 0.8);
    font-size: 12px;
    line-height: 1.4;
    height: 100%;
    padding: 20px;
  }

  /* Resize handle */
  .resize-handle {
    height: 8px;
    background-color: rgba(255, 255, 255, 0.05);
    cursor: ns-resize;
    border-top: 1px solid rgba(255, 255, 255, 0.1);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background-color 0.2s ease;
  }

  .resize-handle:hover {
    background-color: rgba(255, 255, 255, 0.1);
  }

  .resize-handle.resizing {
    background-color: rgba(255, 255, 255, 0.15);
  }

  .resize-indicator {
    width: 40px;
    height: 2px;
    background-color: rgba(255, 255, 255, 0.3);
    border-radius: 1px;
    position: relative;
  }

  .resize-indicator::before {
    content: '';
    position: absolute;
    top: -2px;
    left: 0;
    right: 0;
    height: 2px;
    background-color: rgba(255, 255, 255, 0.3);
    border-radius: 1px;
  }

  .resize-indicator::after {
    content: '';
    position: absolute;
    top: 2px;
    left: 0;
    right: 0;
    height: 2px;
    background-color: rgba(255, 255, 255, 0.3);
    border-radius: 1px;
  }

  /* Responsive Design */
</style>
