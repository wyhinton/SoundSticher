<script lang="ts">
  import { onMount } from "svelte";
  import * as d3 from "d3";
  import { appState } from "./state/state.svelte";
  import { listen } from "@tauri-apps/api/event";
  import { formatFileName } from "./utils/format";
  import TimelineSegment from "./TimelineSegment.svelte";
  import LabelLayer from "./LabelLayer.svelte";

  let container: HTMLDivElement;
  let svgEl: SVGSVGElement;
  let axisGroup: SVGGElement;
  let pathGroup: SVGGElement;
  let labelGroup: SVGGElement

  const height = 120;
  $: durationSeconds = $appState?.combinedFileLength && $appState.sections.length>0 ? $appState.combinedFileLength : 30;
  $: if ($appState?.combinedFileLength && width > 0) {
    updateScales();
  }

  const originalPathWidth = 1000;
  let currentTransform = d3.zoomIdentity;
  let width = 0;
  let scaleX = 1;

  let xScale: d3.ScaleLinear<number, number>;
  let playHeadPosition = 0;
  let playHeadX = 0;
  $: playHeadX = xScale?.(playHeadPosition) ?? 0;

  function updateScales() {
    xScale = d3.scaleLinear().domain([0, durationSeconds]).range([0, width]);
    scaleX = width / originalPathWidth;
    renderAxis(xScale);
  }

  listen<number>("combined-progress", (event) => {
    playHeadPosition = event.payload;
  });

  function handleClick(event: MouseEvent) {
    const rect = container.getBoundingClientRect();
    const relativeX = event.clientX - rect.left;
    console.log(relativeX);
    const clickedTime = currentTransform
      .rescaleX(d3.scaleLinear().domain([0, durationSeconds]).range([0, width]))
      .invert(relativeX);
    console.log(clickedTime);
    playHeadPosition = Math.max(0, Math.min(clickedTime, durationSeconds));
    console.log(playHeadPosition);
    // playHeadX = relativeX;
  }

  function renderAxis(scale: d3.ScaleLinear<number, number>) {
    const axis = d3
      .axisBottom(scale)
      .ticks(Math.floor(width / 60))
      .tickFormat((d: number) => {
        const m = Math.floor(d / 60);
        const s = Math.floor(d % 60);
        return `${m}:${s.toString().padStart(2, "0")}`;
      });

    d3.select(axisGroup).call(axis);

    d3.select(axisGroup)
      .call(axis)
      .selectAll("text")
      .style("font-family", "monospace")
      .style("font-size", "10px"); // optional

    d3.select(axisGroup)
      .call(axis)
      .selectAll("text")
      .style("font-family", "monospace")
      .style("font-size", "10px"); // optional

    const ticks = d3.selectAll("g.tick");

    ticks
      .filter((_, i, nodes) => i === 0)
      .attr("text-anchor", "start")
      .attr("dx", "0.5em");
    //   .attr('color', 'red')

    ticks
      .filter((_, i, nodes) => i === nodes.length - 1)
      .attr("text-anchor", "end")
      .attr("dx", "-0.5em");
    //   .attr('color', 'red')

    ticks.filter((_, i, nodes) => i !== 0).attr("color", "white");
  }

  function setupZoom() {
    const pathGroupD3 = d3.select(pathGroup);

    const labelGroupD3 = d3.select(labelGroup);

    d3.select(svgEl).call(
      d3
        .zoom<SVGSVGElement, unknown>()
        .scaleExtent([1, 10])
        .translateExtent([
          [0, 0],
          [width, 0],
        ])
        .extent([
          [0, 0],
          [width, 0],
        ])
        // .extent([[0, 0], [width, height]])
        .on("zoom", (event) => {
          currentTransform = event.transform;
          pathGroupD3.attr(
            "transform",
            `translate(${event.transform.x}, 0) scale(${event.transform.k}, 1)`
          );
          const newXScale = currentTransform.rescaleX(
            d3.scaleLinear().domain([0, durationSeconds]).range([0, width])
          );
          renderAxis(newXScale);
        })
    );


 
  }

  onMount(() => {
    const resizeObserver = new ResizeObserver(() => {
      width = container.clientWidth;
      updateScales();
      setupZoom();
    });

    const ticks = d3.selectAll(".x-axis .tick text");
    const t = d3.selectAll("g.tick");
    ticks
      .filter((_, i, nodes) => i === 0)
      .attr("text-anchor", "start")
      .attr("dx", "0.5em")
      .attr("color", "red");

    ticks
      .filter((_, i, nodes) => i === nodes.length - 1)
      .attr("text-anchor", "end")
      .attr("dx", "-0.5em");

    d3.selectAll("g.tick")
      .filter(function (d) {
        return d == 50;
      })
      //only ticks that returned true for the filter will be included
      //in the rest of the method calls:
      .select("line") //grab the tick line
      .attr("class", "quadrantBorder") //style with a custom class and CSS
      .style("stroke-width", 5);

    resizeObserver.observe(container);
    return () => resizeObserver.disconnect();
  });
</script>

<div class="svg-container position-relative">
  <div
    class="position-absolute"
    style="font-size: 10px; color: #9d9d9d !important; bottom:20px"
  >
    <!-- {playHeadX} -->
    <!-- {scaleX} -->
    {currentTransform.k.toFixed(2)}x
  </div>
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <!-- x={playHeadX / (0.5 - currentTransform.k)} -->
  <div
    on:click={(e) => {
      handleClick(e);
    }}
    bind:this={container}
    style="width: 100%;"
  >
    <svg class="waveform-svg-parent" bind:this={svgEl} {height} viewBox={`0 0 ${width} ${height}`}>
      <g transform={`translate(0, ${20})`}>
        <g bind:this={pathGroup} transform={``}>
          <path
            d={$appState?.combinedFile?.svgPath}
            stroke="#3091f1"
            fill="none"
            stroke-width="2"
            transform={`scale(${scaleX}, 1) `}
            pointer-events="none"
            id="waveform-path"
          />
          <rect
            x={playHeadX}
            y={0}
            width={1 / currentTransform.k}
            height="80"
            fill="red"
          />
          <!-- {#if $appState?.timelneItems} -->
          {#if $appState?.timelineItems.length>0}
            {#each $appState?.timelineItems as timelineItem, i}
              <TimelineSegment
                scaleX={scaleX}
                index={i}
                startOffset={timelineItem.startOffset}
                size={timelineItem.size}
                label={formatFileName(timelineItem.fileName)}
                originalPathWidth={originalPathWidth}
                zoomTransform={currentTransform}
              />
              <!-- <text
                x={(timelineItem.startOffset * originalPathWidth) + 4}
                y={40}
                dominant-baseline="middle"
                fill="white"
                font-size="10"
                font-family="monospace"
                pointer-events="none"
              >{formatFileName(timelineItem.fileName)}</text>
              <rect
                x={timelineItem.startOffset * originalPathWidth}
                y={0}
                width={timelineItem.size*originalPathWidth}
                height="80"
                fill="rgba(0, 200, 255, 0.15)"
                stroke="rgba(0, 200, 255, 0.5)"
                stroke-width="0.5"
              /> -->
            {/each}
          {/if}
        
        </g>
      </g>
    {#if $appState?.timelineItems.length > 0}
            <LabelLayer xScale={xScale} scaleX={scaleX} items={$appState?.timelineItems} originalPathWidth={originalPathWidth} currentTransform={currentTransform}></LabelLayer>
    {/if}
      <g>

  </g>
    <!-- TIMELINE BACKGROUND -->
      <rect x="0" y={100} {width} height="20" fill="var(--bs-dark-bg-subtle);" />
      <g bind:this={axisGroup} transform={`translate(0, ${height - 20})`} />
    </svg>
  </div>
</div>

<style>
  .waveform-svg-parent{
    margin-bottom: 6px;
  }
  .svg-container {
    background-color: var(--bs-primary-bg-subtle);
  }
  svg {
    width: 100%;
    height: auto;
  }

  g.axis text {
    font-family: monospace;
    font-size: 10px; /* optional: adjust as needed */
  }
</style>
