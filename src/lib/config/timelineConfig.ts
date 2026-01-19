/**
 * Timeline Configuration
 *
 * Centralized configuration for timeline-related constants to ensure consistency
 * across the frontend. These values are used by Timeline.svelte, waveformCache.ts,
 * and other timeline-related components.
 *
 * IMPORTANT: Keep these values in sync across the codebase.
 * - Timeline SVG layout uses these for scalable design
 * - Waveform requests use these for consistent sizing
 * - Any changes here should be reflected in dependent code
 */

/**
 * Timeline SVG Layout Configuration
 */
export const TIMELINE_LAYOUT = {
  /** Top padding for timeline header region (px) */
  TOP_PADDING: 20,

  /** Height of the x-axis footer region (px) */
  AXIS_HEIGHT: 20,

  /** Design height for waveform content region (px)
   * This is the "base" height that content is designed for.
   * The timeline will scale vertically from this base height.
   */
  BASE_CONTENT_HEIGHT: 80,

  /** Default total timeline height when not set by user (px) */
  DEFAULT_HEIGHT: 120,
} as const;

/**
 * Waveform Request Configuration
 */
export const WAVEFORM_CONFIG = {
  /** Default waveform width in pixels (used for initial requests) */
  DEFAULT_WIDTH: 1000,

  /** Default waveform height in pixels
   * MUST match TIMELINE_LAYOUT.BASE_CONTENT_HEIGHT for proper visual alignment
   */
  DEFAULT_HEIGHT: TIMELINE_LAYOUT.BASE_CONTENT_HEIGHT,

  /** Whether to normalize waveform amplitude by default */
  DEFAULT_NORMALIZE: false,

  /** Maximum number of waveforms to cache in memory */
  MAX_CACHE_ENTRIES: 500,
} as const;

/**
 * Derived Timeline Values
 */
export const TIMELINE_DERIVED = {
  /** Center line Y position in design space (half of base content height) */
  get CENTER_Y(): number {
    return TIMELINE_LAYOUT.BASE_CONTENT_HEIGHT / 2;
  },

  /** Total fixed height (header + footer, excluding content) */
  get FIXED_HEIGHT(): number {
    return TIMELINE_LAYOUT.TOP_PADDING + TIMELINE_LAYOUT.AXIS_HEIGHT;
  },

  /** Minimum content height for timeline to be usable */
  get MIN_CONTENT_HEIGHT(): number {
    return 40; // At least 40px for waveforms to be visible
  },
} as const;

/**
 * Timeline Resize Configuration
 */
export const TIMELINE_RESIZE = {
  /** Minimum timeline height as percentage of viewport (10% = very small) */
  MIN_HEIGHT_PERCENT: 10,

  /** Maximum timeline height as percentage of viewport (60% = majority of screen) */
  MAX_HEIGHT_PERCENT: 60,

  /** Default timeline height as percentage of viewport */
  DEFAULT_HEIGHT_PERCENT: 30,
} as const;

/**
 * Type-safe config access
 */
export type TimelineLayoutConfig = typeof TIMELINE_LAYOUT;
export type WaveformConfig = typeof WAVEFORM_CONFIG;
export type TimelineDerivedConfig = typeof TIMELINE_DERIVED;
export type TimelineResizeConfig = typeof TIMELINE_RESIZE;
