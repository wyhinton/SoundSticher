import * as d3 from 'd3';

export interface TimelineItem {
  id: string;
  startOffset: number;
  size: number;
  fileName: string;
  type: string;
  active?: boolean;
}

export interface D3TimelineManagerOptions {
  width: number;
  height: number;
  durationSeconds: number;
  originalPathWidth: number;
  onTransformChange?: (transform: d3.ZoomTransform) => void;
  onAxisUpdate?: (scale: d3.ScaleLinear<number, number>) => void;
}

export class D3TimelineManager {
  private svgElement: SVGSVGElement | null = null;
  private axisGroup: SVGGElement | null = null;
  private pathGroup: SVGGElement | null = null;
  private xScale: d3.ScaleLinear<number, number> | null = null;
  private currentTransform: d3.ZoomTransform = d3.zoomIdentity;
  private zoom: d3.ZoomBehavior<SVGSVGElement, unknown> | null = null;

  private options: D3TimelineManagerOptions;
  private scaleX = 1;

  constructor(options: D3TimelineManagerOptions) {
    this.options = options;
    this.updateScales();
  }

  /**
   * Initialize the D3 manager with SVG elements
   */
  initialize(svgElement: SVGSVGElement, axisGroup: SVGGElement, pathGroup: SVGGElement) {
    this.svgElement = svgElement;
    this.axisGroup = axisGroup;
    this.pathGroup = pathGroup;

    this.setupZoom();
    this.updateScales();
  }

  /**
   * Update the dimensions and duration
   */
  updateOptions(newOptions: Partial<D3TimelineManagerOptions>) {
    this.options = { ...this.options, ...newOptions };
    this.updateScales();
  }

  /**
   * Update scales based on current options
   */
  private updateScales() {
    if (this.options.width <= 0 || this.options.durationSeconds <= 0) return;

    this.xScale = d3
      .scaleLinear()
      .domain([0, this.options.durationSeconds])
      .range([0, this.options.width]);

    this.scaleX = this.options.width / this.options.originalPathWidth;

    if (this.xScale) {
      this.renderAxis(this.xScale);
    }
  }

  /**
   * Render the time axis
   */
  private renderAxis(scale: d3.ScaleLinear<number, number>) {
    if (!this.axisGroup) return;

    const axis = d3
      .axisBottom(scale)
      .ticks(Math.floor(this.options.width / 60))
      .tickFormat((d: number) => {
        const m = Math.floor(d / 60);
        const s = Math.floor(d % 60);
        return `${m}:${s.toString().padStart(2, '0')}`;
      });

    d3.select(this.axisGroup).call(axis);

    // Style the text
    d3.select(this.axisGroup)
      .selectAll('text')
      .style('font-family', 'monospace')
      .style('font-size', '10px');

    const ticks = d3.select(this.axisGroup).selectAll('g.tick');

    // Style first tick
    ticks
      .filter((_, i) => i === 0)
      .select('text')
      .attr('text-anchor', 'start')
      .attr('dx', '0.5em');

    // Style last tick
    ticks
      .filter((_, i, nodes) => i === nodes.length - 1)
      .select('text')
      .attr('text-anchor', 'end')
      .attr('dx', '-0.5em');

    // Style other ticks
    ticks
      .filter((_, i) => i !== 0)
      .select('text')
      .attr('color', 'white');

    // Notify of axis update
    this.options.onAxisUpdate?.(scale);
  }

  /**
   * Setup zoom behavior
   */
  private setupZoom() {
    if (!this.svgElement || !this.pathGroup) return;

    const pathGroupD3 = d3.select(this.pathGroup);

    this.zoom = d3
      .zoom<SVGSVGElement, unknown>()
      .scaleExtent([1, 10])
      .translateExtent([
        [0, 0],
        [this.options.width, 0],
      ])
      .extent([
        [0, 0],
        [this.options.width, 0],
      ])
      .on('zoom', event => {
        this.currentTransform = event.transform;

        // Update path group transform
        pathGroupD3.attr(
          'transform',
          `translate(${event.transform.x}, 0) scale(${event.transform.k}, 1)`
        );

        // Update axis with new scale
        if (this.xScale) {
          const newXScale = this.currentTransform.rescaleX(this.xScale);
          this.renderAxis(newXScale);
        }

        // Notify of transform change
        this.options.onTransformChange?.(this.currentTransform);
      });

    d3.select(this.svgElement).call(this.zoom);
  }

  /**
   * Convert timeline coordinates to screen coordinates
   */
  timelineToScreen(timelineX: number): number {
    return timelineX * this.currentTransform.k + this.currentTransform.x;
  }

  /**
   * Convert screen coordinates to timeline coordinates
   */
  screenToTimeline(screenX: number): number {
    return this.currentTransform.invert
      ? this.currentTransform.invertX(screenX)
      : screenX / this.currentTransform.k - this.currentTransform.x / this.currentTransform.k;
  }

  /**
   * Convert a click position to timeline time
   */
  clickToTime(relativeX: number): number {
    if (!this.xScale) return 0;

    const clickedTime = this.currentTransform.rescaleX(this.xScale).invert(relativeX);

    return Math.max(0, Math.min(clickedTime, this.options.durationSeconds));
  }

  /**
   * Calculate playhead X position based on time
   */
  getPlayheadX(playHeadPosition: number): number {
    return this.xScale?.(playHeadPosition) ?? 0;
  }

  /**
   * Check if a click position intersects with any timeline segments
   */
  findClickedSegment(relativeX: number, timelineItems: TimelineItem[]): number | null {
    for (let i = 0; i < timelineItems.length; i++) {
      const item = timelineItems[i];
      const itemStartX =
        item.startOffset * this.options.originalPathWidth * this.scaleX * this.currentTransform.k +
        this.currentTransform.x;
      const itemEndX =
        itemStartX +
        item.size * this.options.originalPathWidth * this.scaleX * this.currentTransform.k;

      if (relativeX >= itemStartX && relativeX <= itemEndX) {
        return i;
      }
    }
    return null;
  }

  /**
   * Calculate drop position for drag and drop operations
   */
  calculateDropPosition(
    mouseX: number,
    timelineItems: TimelineItem[]
  ): { index: number; x: number } {
    const timelineX = this.screenToTimeline(mouseX);

    let targetIndex = -1;
    let targetX = 0;

    for (let i = 0; i < timelineItems.length; i++) {
      const itemStartX =
        timelineItems[i].startOffset * this.options.originalPathWidth * this.scaleX;
      const itemEndX =
        itemStartX + timelineItems[i].size * this.options.originalPathWidth * this.scaleX;

      if (timelineX >= itemStartX && timelineX <= itemEndX) {
        const midPoint = itemStartX + (itemEndX - itemStartX) / 2;

        if (timelineX < midPoint) {
          targetIndex = i;
          targetX = itemStartX;
        } else {
          targetIndex = i + 1;
          targetX = itemEndX;
        }
        break;
      }
    }

    // If no segment found, place at the end
    if (targetIndex === -1 && timelineItems.length > 0) {
      targetIndex = timelineItems.length;
      const lastItem = timelineItems[timelineItems.length - 1];
      targetX =
        (lastItem.startOffset + lastItem.size) * this.options.originalPathWidth * this.scaleX;
    }

    return {
      index: targetIndex,
      x: this.timelineToScreen(targetX),
    };
  }

  /**
   * Get current transform
   */
  getCurrentTransform(): d3.ZoomTransform {
    return this.currentTransform;
  }

  /**
   * Get current scale X
   */
  getScaleX(): number {
    return this.scaleX;
  }

  /**
   * Get current X scale
   */
  getXScale(): d3.ScaleLinear<number, number> | null {
    return this.xScale;
  }

  /**
   * Cleanup resources
   */
  destroy() {
    if (this.svgElement && this.zoom) {
      d3.select(this.svgElement).on('.zoom', null);
    }
    this.svgElement = null;
    this.axisGroup = null;
    this.pathGroup = null;
    this.zoom = null;
  }
}
