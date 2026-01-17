import { get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { appState } from './state.svelte';

export interface Favorite {
  path: string;
  numAudioFiles: number;
}

export async function addToFavorites(folderPath: string) {
  try {
    // Count audio files in the directory
    const counts = await invoke<Record<string, number>>('count_audio_files_in_folders', {
      folderPaths: [folderPath],
    });

    // Get the count for this specific folder
    const numAudioFiles = counts[folderPath] || 0;

    appState.update(state => {
      // Ensure favorites array exists
      if (!Array.isArray(state.favorites)) {
        state.favorites = [];
      }

      // Check if the path is already in favorites
      const alreadyExists = state.favorites.some(fav => fav && fav.path === folderPath);

      if (!alreadyExists) {
        state.favorites.push({ path: folderPath, numAudioFiles });
        console.log(`Added ${folderPath} to favorites with ${numAudioFiles} audio files`);
      } else {
        console.log(`${folderPath} is already in favorites`);
      }

      return state;
    });
  } catch (error) {
    console.error('Error counting audio files when adding to favorites:', error);
    // Fallback: add without counting
    appState.update(state => {
      if (!Array.isArray(state.favorites)) {
        state.favorites = [];
      }

      const alreadyExists = state.favorites.some(fav => fav && fav.path === folderPath);

      if (!alreadyExists) {
        state.favorites.push({ path: folderPath, numAudioFiles: 0 });
        console.log(`Added ${folderPath} to favorites (could not count audio files)`);
      }

      return state;
    });
  }
}

export function removeFromFavorites(folderPath: string) {
  appState.update(state => {
    // Ensure favorites array exists
    if (!Array.isArray(state.favorites)) {
      state.favorites = [];
      return state;
    }

    state.favorites = state.favorites.filter(fav => fav && fav.path !== folderPath);
    console.log(`Removed ${folderPath} from favorites`);
    return state;
  });
}

export function isFavorite(folderPath: string): boolean {
  const currentState = get(appState);
  // Ensure favorites array exists and is valid
  if (!currentState || !Array.isArray(currentState.favorites)) {
    return false;
  }
  return currentState.favorites.some(fav => fav && fav.path === folderPath);
}
