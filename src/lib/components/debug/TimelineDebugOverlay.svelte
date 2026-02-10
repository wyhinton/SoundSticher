<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { TIMELINE_DERIVED, TIMELINE_LAYOUT } from '$lib/config/timelineConfig';

  export let visible = false;

  // Timeline element references and measurements
  let measurements = {
    viewport: {
      width: 0,
      height: 0,
    },
    timelineContainer: {
      height: 0,
      width: 0,
      heightPercent: 0,
    },
    svgParent: {
      height: 0,
      width: 0,
      viewBoxHeight: 0,
      viewBoxWidth: 0,
    },
    scalableContent: {
      height: 0,
      scaleY: 0,
      computedHeight: 0,
    },
    config: {
      topPadding: TIMELINE_LAYOUT.TOP_PADDING,
      axisHeight: TIMELINE_LAYOUT.AXIS_HEIGHT,
      baseContentHeight: TIMELINE_LAYOUT.BASE_CONTENT_HEIGHT,
      defaultHeight: TIMELINE_LAYOUT.DEFAULT_HEIGHT,
      centerY: TIMELINE_DERIVED.CENTER_Y,
    },
  };

  let updateInterval: number | null = null;

  function updateMeasurements() {
    // Get viewport dimensions
    measurements.viewport.width = window.innerWidth;
    measurements.viewport.height = window.innerHeight;

    // Get timeline container (the parent div with timeline-container class)
    const timelineContainer = document.querySelector('.timeline-container') as HTMLElement;
    if (timelineContainer) {
      const rect = timelineContainer.getBoundingClientRect();
      measurements.timelineContainer.height = rect.height;
      measurements.timelineContainer.width = rect.width;
      measurements.timelineContainer.heightPercent =
        (rect.height / measurements.viewport.height) * 100;
    }

    // Get SVG parent element
    const svgParent = document.querySelector('.waveform-svg-parent') as SVGSVGElement;
    if (svgParent) {
      const rect = svgParent.getBoundingClientRect();
      measurements.svgParent.height = rect.height;
      measurements.svgParent.width = rect.width;

      // Get viewBox dimensions
      const viewBox = svgParent.getAttribute('viewBox');
      if (viewBox) {
        const parts = viewBox.split(' ').map(Number);
        if (parts.length === 4) {
          const [, , vbWidth, vbHeight] = parts;
          measurements.svgParent.viewBoxWidth = vbWidth ?? 0;
          measurements.svgParent.viewBoxHeight = vbHeight ?? 0;
        }
      }
    }

    // Get scalable content group
    const scalableContent = document.querySelector('.scalable-content') as SVGGElement;
    if (scalableContent) {
      // Get transform attribute to extract scale
      const transform = scalableContent.getAttribute('transform');
      if (transform) {
        const scaleMatch = transform.match(/scale\([\d.]+,\s*([\d.]+)\)/);
        if (scaleMatch && scaleMatch[1]) {
          measurements.scalableContent.scaleY = parseFloat(scaleMatch[1]);
        }
      }

      // Calculate content height from SVG viewBox and config
      const contentHeight =
        measurements.svgParent.viewBoxHeight -
        TIMELINE_LAYOUT.TOP_PADDING -
        TIMELINE_LAYOUT.AXIS_HEIGHT;
      measurements.scalableContent.height = contentHeight;
      measurements.scalableContent.computedHeight =
        contentHeight * measurements.scalableContent.scaleY;
    }
  }

  onMount(() => {
    if (visible) {
      updateMeasurements();
      updateInterval = setInterval(updateMeasurements, 100) as unknown as number;
    }
  });

  onDestroy(() => {
    if (updateInterval !== null) {
      clearInterval(updateInterval);
      updateInterval = null;
    }
  });

  // Update measurements when visibility changes
  $: if (visible) {
    updateMeasurements();
    if (updateInterval === null) {
      updateInterval = setInterval(updateMeasurements, 100) as unknown as number;
    }
  } else {
    if (updateInterval !== null) {
      clearInterval(updateInterval);
      updateInterval = null;
    }
  }
</script>

{#if visible}
  <div class="timeline-overlay">
    <div class="overlay-header">
      <i class="fa fa-ruler-vertical"></i>
      Timeline Measurements
    </div>

    <div class="overlay-content">
      <!-- Viewport Section -->
      <div class="section">
        <div class="section-title">
          <i class="fa fa-desktop"></i>
          Viewport
        </div>
        <div class="metrics">
          <div class="metric">
            <span class="label">Width:</span>
            <span class="value">{measurements.viewport.width}px</span>
          </div>
          <div class="metric">
            <span class="label">Height:</span>
            <span class="value">{measurements.viewport.height}px</span>
          </div>
        </div>
      </div>

      <!-- Timeline Container Section -->
      <div class="section">
        <div class="section-title">
          <i class="fa fa-square"></i>
          Timeline Container
        </div>
        <div class="metrics">
          <div class="metric">
            <span class="label">Width:</span>
            <span class="value">{measurements.timelineContainer.width.toFixed(1)}px</span>
          </div>
          <div class="metric">
            <span class="label">Height:</span>
            <span class="value">{measurements.timelineContainer.height.toFixed(1)}px</span>
          </div>
          <div class="metric">
            <span class="label">Height %:</span>
            <span class="value highlight"
              >{measurements.timelineContainer.heightPercent.toFixed(1)}vh</span
            >
          </div>
        </div>
      </div>

      <!-- SVG Parent Section -->
      <div class="section">
        <div class="section-title">
          <i class="fa fa-image"></i>
          SVG Parent
        </div>
        <div class="metrics">
          <div class="metric">
            <span class="label">Actual Width:</span>
            <span class="value">{measurements.svgParent.width.toFixed(1)}px</span>
          </div>
          <div class="metric">
            <span class="label">Actual Height:</span>
            <span class="value">{measurements.svgParent.height.toFixed(1)}px</span>
          </div>
          <div class="metric">
            <span class="label">ViewBox Width:</span>
            <span class="value">{measurements.svgParent.viewBoxWidth}</span>
          </div>
          <div class="metric">
            <span class="label">ViewBox Height:</span>
            <span class="value highlight">{measurements.svgParent.viewBoxHeight}</span>
          </div>
        </div>
      </div>

      <!-- Scalable Content Section -->
      <div class="section">
        <div class="section-title">
          <i class="fa fa-compress-arrows-alt"></i>
          Scalable Content
        </div>
        <div class="metrics">
          <div class="metric">
            <span class="label">Content Height:</span>
            <span class="value">{measurements.scalableContent.height.toFixed(1)}px</span>
          </div>
          <div class="metric">
            <span class="label">Scale Y:</span>
            <span class="value highlight">{measurements.scalableContent.scaleY.toFixed(3)}x</span>
          </div>
          <div class="metric">
            <span class="label">Computed Height:</span>
            <span class="value">{measurements.scalableContent.computedHeight.toFixed(1)}px</span>
          </div>
        </div>
      </div>

      <!-- Config Section -->
      <div class="section">
        <div class="section-title">
          <i class="fa fa-cog"></i>
          Config Constants
        </div>
        <div class="metrics">
          <div class="metric">
            <span class="label">Top Padding:</span>
            <span class="value">{measurements.config.topPadding}px</span>
          </div>
          <div class="metric">
            <span class="label">Axis Height:</span>
            <span class="value">{measurements.config.axisHeight}px</span>
          </div>
          <div class="metric">
            <span class="label">Base Content:</span>
            <span class="value highlight">{measurements.config.baseContentHeight}px</span>
          </div>
          <div class="metric">
            <span class="label">Default Height:</span>
            <span class="value">{measurements.config.defaultHeight}px</span>
          </div>
          <div class="metric">
            <span class="label">Center Y:</span>
            <span class="value">{measurements.config.centerY}px</span>
          </div>
        </div>
      </div>

      <!-- Formula Section -->
      <div class="section formula-section">
        <div class="section-title">
          <i class="fa fa-calculator"></i>
          Formulas
        </div>
        <div class="formulas">
          <div class="formula">
            <span class="formula-label">Content Height =</span>
            <span class="formula-value"
              >ViewBox Height - Top Padding - Axis Height = {measurements.scalableContent.height.toFixed(
                1
              )}px</span
            >
          </div>
          <div class="formula">
            <span class="formula-label">Scale Y =</span>
            <span class="formula-value"
              >Content Height / Base Content Height = {measurements.scalableContent.scaleY.toFixed(
                3
              )}x</span
            >
          </div>
        </div>
      </div>
    </div>

    <div class="overlay-footer">
      <small>Updates every 100ms</small>
    </div>
  </div>
{/if}

<style>
  .timeline-overlay {
    position: fixed;
    top: 50%;
    right: 10px;
    transform: translateY(-50%);
    background: rgba(13, 17, 23, 0.95);
    border: 1px solid var(--bs-primary);
    border-radius: 4px;
    padding: 8px;
    font-size: 10px;
    font-family: 'Courier New', monospace;
    min-width: 280px;
    max-width: 320px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.5);
    z-index: 10000;
    backdrop-filter: blur(10px);
  }

  .overlay-header {
    color: var(--bs-primary);
    font-weight: 600;
    font-size: 11px;
    margin-bottom: 8px;
    padding-bottom: 6px;
    border-bottom: 1px solid var(--bs-primary);
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .overlay-header i {
    font-size: 10px;
  }

  .overlay-content {
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-height: 70vh;
    overflow-y: auto;
  }

  .section {
    background: rgba(0, 0, 0, 0.3);
    border: 1px solid rgba(var(--bs-primary-rgb), 0.3);
    border-radius: 3px;
    padding: 6px;
  }

  .section-title {
    color: var(--bs-info);
    font-weight: 600;
    font-size: 9px;
    margin-bottom: 4px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .section-title i {
    font-size: 8px;
  }

  .metrics {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .metric {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 2px 0;
  }

  .label {
    color: var(--bs-secondary);
    font-size: 9px;
  }

  .value {
    color: var(--bs-light);
    font-weight: 600;
    font-size: 9px;
  }

  .value.highlight {
    color: var(--bs-warning);
  }

  .formula-section {
    background: rgba(13, 110, 253, 0.1);
    border-color: rgba(13, 110, 253, 0.5);
  }

  .formulas {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .formula {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 3px 0;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
  }

  .formula:last-child {
    border-bottom: none;
  }

  .formula-label {
    color: var(--bs-info);
    font-size: 8px;
    font-weight: 600;
  }

  .formula-value {
    color: var(--bs-light);
    font-size: 8px;
  }

  .overlay-footer {
    margin-top: 6px;
    padding-top: 6px;
    border-top: 1px solid rgba(var(--bs-primary-rgb), 0.3);
    text-align: center;
  }

  .overlay-footer small {
    color: var(--bs-secondary);
    font-size: 8px;
    font-style: italic;
  }

  /* Custom scrollbar for overlay content */
  .overlay-content::-webkit-scrollbar {
    width: 4px;
  }

  .overlay-content::-webkit-scrollbar-track {
    background: rgba(0, 0, 0, 0.2);
    border-radius: 2px;
  }

  .overlay-content::-webkit-scrollbar-thumb {
    background: var(--bs-primary);
    border-radius: 2px;
  }

  .overlay-content::-webkit-scrollbar-thumb:hover {
    background: var(--bs-primary-text-emphasis);
  }
</style>
