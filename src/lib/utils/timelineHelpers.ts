import type {
  TimelineItem,
  AudioFileTimelineItem,
  SpacerTimelineItem,
} from '../state/state.svelte';
import { formatFileName } from './format';

// Type guards
export function isAudioFileItem(item: TimelineItem): item is AudioFileTimelineItem {
  return item.kind === 'sample' || item.kind === 'merge';
}

export function isSpacerItem(item: TimelineItem): item is SpacerTimelineItem {
  return item.kind === 'spacer';
}

// Common display properties
export function getDisplayName(item: TimelineItem): string {
  switch (item.kind) {
    case 'sample':
    case 'merge':
      return formatFileName((item as AudioFileTimelineItem).fileName);
    case 'spacer':
      return `Spacer (${(item as SpacerTimelineItem).length}s)`;
    default:
      return 'Unknown Item';
  }
}

export function getItemSize(item: TimelineItem): number {
  switch (item.kind) {
    case 'sample':
    case 'merge':
      return (item as AudioFileTimelineItem).size;
    case 'spacer':
      // Convert spacer length to relative size - adjust this logic as needed
      return (item as SpacerTimelineItem).length / 100; // Assuming length is in seconds and we need a normalized value
    default:
      return 0;
  }
}

export function isItemActive(item: TimelineItem): boolean {
  switch (item.kind) {
    case 'sample':
    case 'merge':
      return (item as AudioFileTimelineItem).active ?? true;
    case 'spacer':
      return true; // Spacers are always "active"
    default:
      return false;
  }
}

export function canItemBeDragged(item: TimelineItem): boolean {
  switch (item.kind) {
    case 'sample':
    case 'merge':
      return true; // Audio files can be dragged
    case 'spacer':
      return false; // Spacers might not be draggable
    default:
      return false;
  }
}

export function getItemColor(item: TimelineItem): string {
  switch (item.kind) {
    case 'sample':
      return 'rgb(48, 145, 241)'; // Blue for samples
    case 'merge':
      return 'rgb(255, 200, 100)'; // Orange for merge operations
    case 'spacer':
      return '#666666'; // Gray for spacers
    default:
      return '#cccccc';
  }
}

export function getItemTextColor(item: TimelineItem): string {
  switch (item.kind) {
    case 'sample':
    case 'merge':
      return 'rgba(0, 0, 0, 0.6)'; // Dark text for audio files
    case 'spacer':
      return 'rgba(255, 255, 255, 0.8)'; // Light text for spacers
    default:
      return 'rgba(0, 0, 0, 0.6)';
  }
}

// For operations that only apply to specific types
export function getAudioFilePath(item: TimelineItem): string | null {
  if (isAudioFileItem(item)) {
    return item.fileName;
  }
  return null;
}

export function getSpacerDuration(item: TimelineItem): number | null {
  if (isSpacerItem(item)) {
    return item.length;
  }
  return null;
}

// Helper for getting SVG path (only audio files have this)
export function getItemSvgPath(item: TimelineItem): string | null {
  if (isAudioFileItem(item)) {
    return item.svgPath;
  }
  return null;
}

// Helper for determining if item should show a label
export function shouldShowLabel(item: TimelineItem): boolean {
  switch (item.kind) {
    case 'sample':
    case 'merge':
      return true; // Always show labels for audio files
    case 'spacer':
      return false; // Maybe don't show labels for spacers, or only if they're long enough
    default:
      return false;
  }
}

// Example usage functions demonstrating type-safe operations
export function processTimelineItems(items: TimelineItem[]) {
  items.forEach((item, index) => {
    console.log(`Processing item ${index}: ${item.kind}`);

    // Type-safe operations
    if (isAudioFileItem(item)) {
      // TypeScript knows this is AudioFileTimelineItem
      console.log(`  Audio file: ${item.fileName}, active: ${item.active}`);
      console.log(`  SVG path length: ${item.svgPath.length}`);
    } else if (isSpacerItem(item)) {
      // TypeScript knows this is SpacerTimelineItem
      console.log(`  Spacer duration: ${item.length}s`);
    }
  });
}

export function getActiveAudioFiles(items: TimelineItem[]): string[] {
  return items
    .filter(isAudioFileItem) // TypeScript narrows to AudioFileTimelineItem[]
    .filter(item => item.active)
    .map(item => item.fileName);
}

export function getTotalSpacerTime(items: TimelineItem[]): number {
  return items
    .filter(isSpacerItem) // TypeScript narrows to SpacerTimelineItem[]
    .reduce((total, spacer) => total + spacer.length, 0);
}

export function getItemsOfKind<T extends TimelineItem['kind']>(
  items: TimelineItem[],
  kind: T
): Extract<TimelineItem, { kind: T }>[] {
  return items.filter(item => item.kind === kind) as Extract<TimelineItem, { kind: T }>[];
}
