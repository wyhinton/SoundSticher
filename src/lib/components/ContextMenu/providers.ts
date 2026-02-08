import { invoke } from '@tauri-apps/api/core';
import { invokeWithPerf } from '../../state/performance';
import type { ContextMenuProvider } from './types';

//

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
