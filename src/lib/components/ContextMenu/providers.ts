import type { ContextMenuProvider } from './types';
import { invokeWithPerf } from '../../state/performance';
import { audioFileStateManager } from '../../state/stateSynchronization';
import { invoke } from '@tauri-apps/api/core';

// Timeline Segment Context Menu
export const timelineSegmentProvider: ContextMenuProvider = context => {
  const target = context.target;
  const segmentElement = target.closest('[data-timeline-segment]');

  if (!segmentElement) return null;

  const segmentIndex = parseInt(segmentElement.getAttribute('data-segment-index') || '0');
  const segmentId = segmentElement.getAttribute('data-segment-id');
  const isActive = segmentElement.getAttribute('data-segment-active') === 'true';
  const isSelected = context.selectedItems?.includes(segmentIndex) || false;

  const selectedCount = context.selectedItems?.length || 0;
  const hasSelection = selectedCount > 1;

  return {
    position: { x: context.event.clientX, y: context.event.clientY },
    items: [
      {
        label: hasSelection ? `Play Selected (${selectedCount})` : 'Play Segment',
        icon: 'fas fa-play',
        action: () => {
          if (hasSelection && context.selectedItems) {
            console.log('Play selected segments:', context.selectedItems);
            // TODO: Implement play selected segments
          } else {
            console.log('Play segment:', segmentId);
            // TODO: Implement play single segment
          }
        },
      },
      { type: 'separator' },
      {
        label: hasSelection
          ? `${isActive ? 'Deactivate' : 'Activate'} Selected (${selectedCount})`
          : `${isActive ? 'Deactivate' : 'Activate'} Segment`,
        icon: isActive ? 'fas fa-eye-slash' : 'fas fa-eye',
        action: async () => {
          const targetIds =
            hasSelection && context.selectedItems
              ? context.selectedItems
                  .map(idx => context.appState.timelineItems?.[idx]?.id)
                  .filter(Boolean)
              : segmentId
                ? [segmentId]
                : [];

          if (targetIds.length > 0) {
            try {
              await audioFileStateManager.setFilesActive(targetIds, !isActive);
              console.log(`${isActive ? 'Deactivated' : 'Activated'} segments:`, targetIds);
            } catch (error) {
              console.error('Failed to update segment active status:', error);
            }
          }
        },
      },
      {
        label: hasSelection ? `Remove Selected (${selectedCount})` : 'Remove Segment',
        icon: 'fas fa-trash',
        variant: 'danger',
        action: () => {
          const targetIds =
            hasSelection && context.selectedItems
              ? context.selectedItems
                  .map(idx => context.appState.timelineItems?.[idx]?.id)
                  .filter(Boolean)
              : segmentId
                ? [segmentId]
                : [];

          if (targetIds.length > 0) {
            console.log('Remove segments:', targetIds);
            // TODO: Implement segment removal
          }
        },
      },
      { type: 'separator' },
      {
        label: 'Properties',
        icon: 'fas fa-info-circle',
        action: () => {
          console.log('Show properties for segment:', segmentId);
          // TODO: Implement properties dialog
        },
      },
    ],
  };
};

// File Table Row Context Menu
export const fileTableRowProvider: ContextMenuProvider = context => {
  const target = context.target;
  const rowElement = target.closest('tr[data-file-id]');

  if (!rowElement) return null;

  const fileId = rowElement.getAttribute('data-file-id');
  const isActive = rowElement.getAttribute('data-file-active') === 'true';
  const filePath = rowElement.getAttribute('data-file-path') || '';
  const fileName = filePath.split(/[/\\]/).pop() || 'Unknown File';

  return {
    position: { x: context.event.clientX, y: context.event.clientY },
    items: [
      {
        label: 'Play Preview',
        icon: 'fas fa-play',
        action: () => {
          try {
            invokeWithPerf('play_sample_preview', { path: filePath });
            console.log('Playing preview for:', fileName);
          } catch (error) {
            console.error('Failed to play preview:', error);
          }
        },
      },
      { type: 'separator' },
      {
        label: isActive ? 'Deactivate' : 'Activate',
        icon: isActive ? 'fas fa-eye-slash' : 'fas fa-eye',
        action: async () => {
          if (fileId) {
            try {
              await audioFileStateManager.setFilesActive([fileId], !isActive);
              console.log(`${isActive ? 'Deactivated' : 'Activated'} file:`, fileName);
            } catch (error) {
              console.error('Failed to update file active status:', error);
            }
          }
        },
      },
      {
        label: 'Remove from Project',
        icon: 'fas fa-times',
        variant: 'danger',
        action: () => {
          console.log('Remove file from project:', fileName);
          // TODO: Implement file removal from project
        },
      },
      { type: 'separator' },
      {
        label: 'Show in Explorer',
        icon: 'fas fa-folder-open',
        action: () => {
          try {
            invoke('show_in_explorer', { path: filePath });
            console.log('Showing in explorer:', filePath);
          } catch (error) {
            console.error('Failed to show in explorer:', error);
          }
        },
      },
      {
        label: 'Copy Path',
        icon: 'fas fa-copy',
        action: () => {
          try {
            navigator.clipboard.writeText(filePath);
            console.log('Copied path to clipboard:', filePath);
          } catch (error) {
            console.error('Failed to copy path:', error);
          }
        },
      },
      {
        label: 'Copy Filename',
        icon: 'fas fa-copy',
        action: () => {
          try {
            navigator.clipboard.writeText(fileName);
            console.log('Copied filename to clipboard:', fileName);
          } catch (error) {
            console.error('Failed to copy filename:', error);
          }
        },
      },
    ],
  };
};

// Sources Table Context Menu
export const sourcesTableProvider: ContextMenuProvider = context => {
  const target = context.target;
  const rowElement = target.closest('[data-source-row]');

  if (!rowElement) return null;

  const sectionIndex = parseInt(rowElement.getAttribute('data-section-index') || '0');
  const isSelected = context.selectedItems?.includes(sectionIndex) || false;
  const selectedCount = context.selectedItems?.length || 0;
  const hasSelection = selectedCount > 1;

  return {
    position: { x: context.event.clientX, y: context.event.clientY },
    items: [
      {
        label: hasSelection ? `Refresh Selected (${selectedCount})` : 'Refresh Source',
        icon: 'fas fa-sync',
        action: () => {
          console.log('Refresh sources:', hasSelection ? context.selectedItems : [sectionIndex]);
          // TODO: Implement source refresh
        },
      },
      { type: 'separator' },
      {
        label: hasSelection ? `Remove Selected (${selectedCount})` : 'Remove Source',
        icon: 'fas fa-trash',
        variant: 'danger',
        action: () => {
          console.log('Remove sources:', hasSelection ? context.selectedItems : [sectionIndex]);
          // TODO: Implement source removal
        },
      },
      { type: 'separator' },
      {
        label: 'Add New Source',
        icon: 'fas fa-plus',
        action: () => {
          console.log('Add new source');
          // TODO: Implement add new source
        },
      },
    ],
  };
};

// Export Panel Context Menu
export const exportPanelProvider: ContextMenuProvider = context => {
  const target = context.target;
  const exportPanel = target.closest('.export-panel');

  if (!exportPanel) return null;

  return {
    position: { x: context.event.clientX, y: context.event.clientY },
    items: [
      {
        label: 'Reset Export Settings',
        icon: 'fas fa-undo',
        action: () => {
          console.log('Reset export settings');
          // TODO: Implement reset export settings
        },
      },
      {
        label: 'Save Preset',
        icon: 'fas fa-save',
        action: () => {
          console.log('Save export preset');
          // TODO: Implement save preset
        },
      },
      {
        label: 'Load Preset',
        icon: 'fas fa-folder-open',
        action: () => {
          console.log('Load export preset');
          // TODO: Implement load preset
        },
      },
    ],
  };
};

// General/Fallback Context Menu
export const generalProvider: ContextMenuProvider = context => {
  // This runs last and provides a fallback menu for empty areas
  const target = context.target;

  // Don't show general menu if we're over interactive elements
  if (target.closest('button, input, select, textarea, [role="button"]')) {
    return null;
  }

  return {
    position: { x: context.event.clientX, y: context.event.clientY },
    items: [
      {
        label: 'Add Source',
        icon: 'fas fa-plus',
        action: () => {
          console.log('Add new source from general menu');
          // TODO: Implement add source
        },
      },
      { type: 'separator' },
      {
        label: 'Refresh All',
        icon: 'fas fa-sync',
        action: () => {
          console.log('Refresh all sources');
          // TODO: Implement refresh all
        },
      },
      { type: 'separator' },
      {
        label: 'Settings',
        icon: 'fas fa-cog',
        action: () => {
          console.log('Open settings');
          // TODO: Implement settings dialog
        },
      },
    ],
  };
};
