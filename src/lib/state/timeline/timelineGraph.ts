/**
 * Timeline Graph Utilities
 *
 * Provides utilities for flattening the operation graph into a flat array
 * suitable for D3/SVG rendering while preserving hierarchy information.
 *
 * ARCHITECTURE:
 * - The operation graph can have nested MergeOps (MergeOps containing other MergeOps)
 * - For rendering, we flatten everything into a single array
 * - Each item carries metadata about its position in the hierarchy
 * - This enables:
 *   - Visual distinction (depth-based Y offset, different styling)
 *   - Group drag behavior (dragging a MergeOp moves all descendants)
 *   - Selection rules (click MergeOp to select group)
 */

import { logger } from '../logging';
import type { TimelineItem, TimelineItemKind, AudioFileTimelineItem } from '../state.svelte';

// ============================================================================
// TYPES
// ============================================================================

export interface FlattenedTimelineItem
  extends Omit<
    AudioFileTimelineItem,
    'children' | 'parentId' | 'depth' | 'isGroup' | 'operationName'
  > {
  /** IDs of child timeline items (for MergeOps) */
  children: string[];
  /** ID of the parent timeline item (undefined for root items) */
  parentId: string | undefined;
  /** Visual nesting depth (0 = root level) */
  depth: number;
  /** Semantic hint that this item is a group container */
  isGroup: boolean;
  /** The operation name this item came from */
  operationName: string;
  /** ID of the topmost MergeOp ancestor (for group operations) */
  rootGroupId: string | undefined;
}

export interface TimelineHierarchy {
  /** All items flattened for rendering */
  flatItems: FlattenedTimelineItem[];
  /** Map from item ID to its children IDs */
  childrenMap: Map<string, string[]>;
  /** Map from item ID to its parent ID */
  parentMap: Map<string, string>;
  /** Map from item ID to all descendant IDs (recursive) */
  descendantsMap: Map<string, string[]>;
}

// ============================================================================
// HIERARCHY UTILITIES
// ============================================================================

/**
 * Get all descendant IDs of a given item (recursive)
 */
export function getAllDescendants(itemId: string, hierarchy: TimelineHierarchy): string[] {
  const cached = hierarchy.descendantsMap.get(itemId);
  if (cached) return cached;

  const children = hierarchy.childrenMap.get(itemId) || [];
  const descendants: string[] = [];

  for (const childId of children) {
    descendants.push(childId);
    const childDescendants = getAllDescendants(childId, hierarchy);
    descendants.push(...childDescendants);
  }

  hierarchy.descendantsMap.set(itemId, descendants);
  return descendants;
}

/**
 * Get all ancestor IDs of a given item (from immediate parent to root)
 */
export function getAllAncestors(itemId: string, hierarchy: TimelineHierarchy): string[] {
  const ancestors: string[] = [];
  let currentId = hierarchy.parentMap.get(itemId);

  while (currentId) {
    ancestors.push(currentId);
    currentId = hierarchy.parentMap.get(currentId);
  }

  return ancestors;
}

/**
 * Find the root group ID for an item (topmost MergeOp ancestor)
 */
export function findRootGroupId(
  itemId: string,
  hierarchy: TimelineHierarchy,
  items: FlattenedTimelineItem[]
): string | undefined {
  const ancestors = getAllAncestors(itemId, hierarchy);

  // Find the topmost ancestor that is a group
  for (let i = ancestors.length - 1; i >= 0; i--) {
    const ancestorId = ancestors[i];
    if (ancestorId) {
      const ancestor = items.find(item => item.id === ancestorId);
      if (ancestor?.isGroup) {
        return ancestorId;
      }
    }
  }

  return undefined;
}

/**
 * Check if an item is inside a MergeOp (has a parent)
 */
export function isNestedItem(itemId: string, hierarchy: TimelineHierarchy): boolean {
  return hierarchy.parentMap.has(itemId);
}

/**
 * Get items that should move together when dragging
 * - If dragging a MergeOp, include all descendants
 * - If dragging a sample inside a MergeOp, just move that sample
 */
export function getItemsToMoveOnDrag(
  draggedItemId: string,
  items: FlattenedTimelineItem[],
  hierarchy: TimelineHierarchy
): string[] {
  const draggedItem = items.find(item => item.id === draggedItemId);

  if (!draggedItem) {
    return [draggedItemId];
  }

  // If it's a group (MergeOp), include all descendants
  if (draggedItem.isGroup) {
    const descendants = getAllDescendants(draggedItemId, hierarchy);
    return [draggedItemId, ...descendants];
  }

  // Otherwise just move the single item
  return [draggedItemId];
}

/**
 * Get indices of items to move (for DragDropManager compatibility)
 */
export function getIndicesToMoveOnDrag(
  draggedIndex: number,
  items: FlattenedTimelineItem[],
  hierarchy: TimelineHierarchy
): number[] {
  const draggedItem = items[draggedIndex];

  if (!draggedItem) {
    return [draggedIndex];
  }

  const idsToMove = getItemsToMoveOnDrag(draggedItem.id, items, hierarchy);

  // Convert IDs to indices
  return idsToMove
    .map(id => items.findIndex(item => item.id === id))
    .filter(index => index >= 0)
    .sort((a, b) => a - b);
}

// ============================================================================
// FLATTEN UTILITIES
// ============================================================================

/**
 * Build hierarchy maps from flattened items
 */
export function buildHierarchyMaps(items: FlattenedTimelineItem[]): TimelineHierarchy {
  const childrenMap = new Map<string, string[]>();
  const parentMap = new Map<string, string>();
  const descendantsMap = new Map<string, string[]>();

  for (const item of items) {
    // Build children map
    if (item.children.length > 0) {
      childrenMap.set(item.id, item.children);
    }

    // Build parent map
    if (item.parentId) {
      parentMap.set(item.id, item.parentId);
    }
  }

  return {
    flatItems: items,
    childrenMap,
    parentMap,
    descendantsMap, // Lazily populated by getAllDescendants
  };
}

/**
 * Create a timeline item for a MergeOp container
 */
export function createMergeOpTimelineItem(
  operationName: string,
  startOffset: number,
  size: number,
  totalDuration: number,
  depth: number,
  parentId: string | undefined,
  childIds: string[]
): FlattenedTimelineItem {
  return {
    kind: 'merge',
    id: `merge:${operationName}`,
    fileName: operationName,
    svgPath: '', // MergeOps don't have waveforms
    startOffset,
    size,
    active: true,
    duration: totalDuration * size,
    children: childIds,
    parentId,
    depth,
    isGroup: true,
    operationName,
    rootGroupId: undefined, // Will be computed after flattening
  };
}

/**
 * Create a timeline item for a SampleOp leaf
 */
export function createSampleOpTimelineItem(
  operationName: string,
  filePath: string,
  startOffset: number,
  size: number,
  duration: number,
  svgPath: string,
  depth: number,
  parentId: string | undefined
): FlattenedTimelineItem {
  return {
    kind: 'sample',
    id: filePath, // Use file path as ID for samples
    fileName: filePath,
    svgPath,
    startOffset,
    size,
    active: true,
    duration,
    children: [],
    parentId,
    depth,
    isGroup: false,
    operationName,
    rootGroupId: undefined, // Will be computed after flattening
  };
}

// ============================================================================
// SELECTION UTILITIES
// ============================================================================

/**
 * Determine what should be selected when clicking an item
 * - If clicking a MergeOp without modifier, select only the group
 * - If clicking with Cmd/Ctrl, allow individual selection
 */
export function resolveSelectionOnClick(
  clickedItemId: string,
  items: FlattenedTimelineItem[],
  hierarchy: TimelineHierarchy,
  isMultiSelect: boolean
): string[] {
  const clickedItem = items.find(item => item.id === clickedItemId);

  if (!clickedItem) {
    return [];
  }

  // If multi-select (Cmd/Ctrl), just select the clicked item
  if (isMultiSelect) {
    return [clickedItemId];
  }

  // If it's a group, select only the group (not children)
  // Children can be selected individually with Cmd/Ctrl
  if (clickedItem.isGroup) {
    return [clickedItemId];
  }

  // For samples, just select the sample
  return [clickedItemId];
}

// ============================================================================
// DEBUG UTILITIES
// ============================================================================

/**
 * Log hierarchy information for debugging
 */
export function logHierarchy(hierarchy: TimelineHierarchy): void {
  logger.timeline?.info?.('Timeline Hierarchy:');
  logger.timeline?.info?.(`  Total items: ${hierarchy.flatItems.length}`);
  logger.timeline?.info?.(`  Groups: ${hierarchy.flatItems.filter(i => i.isGroup).length}`);
  logger.timeline?.info?.(`  Leaf items: ${hierarchy.flatItems.filter(i => !i.isGroup).length}`);

  for (const item of hierarchy.flatItems) {
    const indent = '  '.repeat(item.depth + 1);
    const prefix = item.isGroup ? '📁' : '🎵';
    logger.timeline?.info?.(
      `${indent}${prefix} ${item.operationName} (depth: ${item.depth}, children: ${item.children.length})`
    );
  }
}
