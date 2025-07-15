<script lang="ts">
  import { onMount } from "svelte";
  import * as d3 from "d3";
  import { appState } from "./state/state.svelte";

  let container: HTMLDivElement;
  let svgEl: SVGSVGElement;
  let axisGroup: SVGGElement;
  let pathGroup: SVGGElement;

  const height = 100;
  const durationSeconds = $appState?.combineFileMeta?.duration;

  const rawPath = $appState?.combinedFile?.svgPath;
  const originalPathWidth = 1000;
  let width = 0;
  let scaleX = 1;

  let xScale: d3.ScaleLinear<number, number>;

  function updateScales() {
    xScale = d3.scaleLinear().domain([0, durationSeconds/1000]).range([0, width]);

    scaleX = width / originalPathWidth;

    renderAxis(xScale);
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
    console.log(ticks);

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
          const t = event.transform;
        //   pathGroupD3.attr("transform", t.toString());
          pathGroupD3.attr("transform", `translate(${t.x}, 0) scale(${t.k}, 1)`);
          console.log(height);
          const newXScale = t.rescaleX(
            d3.scaleLinear().domain([0, durationSeconds/1000]).range([0, width])
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
    console.log(t);
    console.log(ticks);
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

<div class="svg-container ">
  <div bind:this={container} style="width: 100%;">
    <svg bind:this={svgEl} {height} viewBox={`0 0 ${width} ${height}`}>
      <g bind:this={pathGroup}>
        <path
          d={rawPath}
          stroke="steelblue"
          fill="none"
          stroke-width="2"
          transform={`scale(${scaleX}, 1)`}
        />
      </g>
      <rect
        x="0"
        y={80}
        {width}
        height="20"
        fill="var(--bs-dark-bg-subtle);"
      />
      <g bind:this={axisGroup} transform={`translate(0, ${height - 20})`} />
    </svg>
  </div>
</div>

<style>

  .svg-container{
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
