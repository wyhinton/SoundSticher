import { listen } from '@tauri-apps/api/event';
import { appState, type AppState } from './state.svelte';
import { invokeWithPerf, updateInputs } from './performance';
import { get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';

interface StateChangeEvent {
  file_id: string;
  field: string;
  value: any;
}

/**
 * Sets up event listeners for state synchronization between backend and frontend.
 * This ensures that when the backend state changes, the frontend is automatically updated.
 */
export function initializeStateSynchronization() {
  // Listen for audio file state changes from the backend
  listen<StateChangeEvent>('audio_file_state_changed', event => {
    const { file_id, field, value } = event.payload;
    console.log(`State sync: ${field} changed to ${value} for file ${file_id}`);

    updateAudioFileProperty(file_id, field, value);
  });

  console.log('State synchronization initialized');
}

/**
 * Updates a specific property of an audio file in both sections and timeline items
 */
function updateAudioFileProperty(fileId: string, field: string, value: any) {
  appState.update(state => {
    // Update in sections
    const updatedSections = state.sections.map(section => ({
      ...section,
      files: section.files.map(file => (file.id === fileId ? { ...file, [field]: value } : file)),
    }));

    // Update in timeline items if they exist
    const updatedTimelineItems = state.timelineItems?.map(item =>
      item.id === fileId ? { ...item, [field]: value } : item
    );

    return {
      ...state,
      sections: updatedSections,
      timelineItems: updatedTimelineItems || state.timelineItems,
    };
  });
}

/**
 * Audio File State Manager - provides methods for updating audio file state
 * with optimistic updates and automatic synchronization
 */
export class AudioFileStateManager {
  /**
   * Sets multiple files active/inactive with optimistic updates
   */
  async setFilesActive(fileIds: string[], active: boolean): Promise<void> {
    // 1. Optimistic frontend update
    this.updateFrontendFilesActive(fileIds, active);

    try {
      // 2. Backend update (events will be emitted automatically)
      await invokeWithPerf('set_audio_files_active_batch', {
        fileIds,
        active,
      });

      console.log(`Successfully set ${fileIds.length} files to active: ${active}`);
    } catch (error) {
      // 3. Rollback on failure
      console.error('Failed to update files active status:', error);
      this.updateFrontendFilesActive(fileIds, !active);
      throw error;
    }
    updateInputs(get(appState).sections);
  }

  /**
   * Toggles a single file's active status with optimistic updates
   */
  async toggleFileActive(fileId: string): Promise<boolean> {
    // Get current state to determine what the new state should be
    const currentState = get(appState);
    const file = this.findFileInState(currentState, fileId);

    if (!file) {
      throw new Error(`File with ID ${fileId} not found`);
    }

    const newActive = !file.active;

    // 1. Optimistic frontend update
    this.updateFrontendFilesActive([fileId], newActive);

    try {
      // 2. Backend update (events will be emitted automatically)
      const result = await invoke<boolean>('toggle_audio_file_active', {
        fileId,
      });

      console.log(`Successfully toggled file ${fileId} to active: ${newActive}`);
      return result;
    } catch (error) {
      // 3. Rollback on failure
      console.error('Failed to toggle file active status:', error);
      this.updateFrontendFilesActive([fileId], !newActive);
      throw error;
    }
  }

  /**
   * Sets a single file's active status with optimistic updates
   */
  async setFileActive(fileId: string, active: boolean): Promise<void> {
    // 1. Optimistic frontend update
    this.updateFrontendFilesActive([fileId], active);

    try {
      // 2. Backend update (events will be emitted automatically)
      await invoke<void>('set_audio_file_active', {
        fileId,
        active,
      });

      console.log(`Successfully set file ${fileId} to active: ${active}`);
    } catch (error) {
      // 3. Rollback on failure
      console.error('Failed to set file active status:', error);
      this.updateFrontendFilesActive([fileId], !active);
      throw error;
    }
  }

  /**
   * Updates the frontend state for multiple files' active status
   */
  private updateFrontendFilesActive(fileIds: string[], active: boolean): void {
    fileIds.forEach(fileId => {
      updateAudioFileProperty(fileId, 'active', active);
    });
  }

  /**
   * Finds a file in the current app state by ID
   */
  private findFileInState(state: AppState, fileId: string) {
    for (const section of state.sections) {
      const file = section.files.find(f => f.id === fileId);
      if (file) return file;
    }
    return null;
  }
}

// Create a singleton instance for use throughout the app
export const audioFileStateManager = new AudioFileStateManager();
