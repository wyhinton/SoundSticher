use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use tauri::ipc::Channel;
use tauri::State;
use uuid::Uuid;

use crate::state::{get_app_state, AppState, AudioFile};

#[derive(Clone, Serialize)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "event",
    content = "data"
)]
pub enum SortAudioEvent {
    Started {
        content_length: usize,
    },
    Progress {
        progress: f64,
        id: Uuid,
        start_offset: f64,
    },
    Finished,
}

#[derive(Deserialize, Clone)]
pub struct SortUpdate {
    pub id: Uuid,     // UUID of the AudioFile
    pub index: usize, // new order in timeline
}

#[tauri::command]
pub fn update_sorting(
    updates: Vec<SortUpdate>,
    state: State<'_, Arc<AppState>>,
    on_event: Channel<SortAudioEvent>,
) -> Result<Vec<(Uuid, usize)>, String> {
    println!("STARTING SORT");
    let _ = on_event.send(SortAudioEvent::Started {
        content_length: (10),
    });
    let mut audio_files = state.audio_files.lock().map_err(|_| "Lock poisoned")?;

    // Print initial BTreeMap order
    println!("Initial BTreeMap order:");
    for (key, file) in audio_files.iter() {
        println!("{} -> {}", key, file.id);
    }

    // println!("update0{}", updates[0].id);
    // println!("update1{}", updates[1].id);

    // Print order before sorting
    println!("Order before sorting (by input order):");
    for (i, update) in updates.iter().enumerate() {
        println!("  {}: ID {} -> index {}", i, update.id, update.index);
    }

    // Sort updates by index to get the new order
    let mut ordered_updates = updates.clone();
    ordered_updates.sort_by_key(|u| u.index);

    // Print order after sorting
    println!("Order after sorting (by index):");
    for (i, update) in ordered_updates.iter().enumerate() {
        println!("  {}: ID {} -> index {}", i, update.id, update.index);
    }

    // Calculate total samples in the new order
    let total_samples: usize = ordered_updates
        .iter()
        .filter_map(|u| audio_files.values().find(|f| f.id == u.id))
        .map(|file| file.samples.len())
        .sum();

    // if total_samples == 0 {
    //     return Ok(()); // nothing to update
    // }

    let mut current_sample_offset = 0;
    println!("Current AudioFile IDs in BTreeMap:");
    for file in audio_files.values() {
        println!("{}", file.id);
        println!("{}", file.path);
    }
    // Update start_offset for each file according to new order
    for update in &ordered_updates {
        if let Some(file) = audio_files.values_mut().find(|f| f.id == update.id) {
            file.start_offset = current_sample_offset as f64 / total_samples as f64;
            current_sample_offset += file.samples.len();
        }
    }

    // Reorder BTreeMap to reflect new timeline order
    let mut new_map: BTreeMap<String, AudioFile> = BTreeMap::new();
    let num_updates = ordered_updates.len();
    for (i, update) in ordered_updates.iter().enumerate() {
        if let Some(file) = audio_files.values().find(|f| f.id == update.id) {
            new_map.insert(file.path.clone(), file.clone());

            // Send progress as a float between 0.0 and 1.0
            let progress = (i + 1) as f64 / num_updates as f64;
            on_event.send(SortAudioEvent::Progress {
                progress,
                start_offset: file.start_offset,
                id: file.id,
            });

            if let Err(e) = on_event.send(SortAudioEvent::Progress {
                progress,
                start_offset: file.start_offset,
                id: file.id,
            }) {
                eprintln!("⚠️ Failed to send progress event: {}", e);
            }
        } else {
            eprintln!("NOT FOUND");
        }
    }

    *audio_files = new_map;

    // Store the custom order in the app state
    let ordered_ids: Vec<Uuid> = ordered_updates.iter().map(|u| u.id).collect();
    let mut custom_order = state.custom_order.lock().map_err(|_| "Lock poisoned")?;
    *custom_order = ordered_ids.clone();

    // Print the stored custom order
    println!("Stored custom order:");
    for (i, id) in ordered_ids.iter().enumerate() {
        println!("  {}: {}", i, id);
    }

    // Print final BTreeMap order
    println!("Final BTreeMap order:");
    for (key, file) in audio_files.iter() {
        println!("{} -> {}", key, file.id);
    }

    let result: Vec<(Uuid, usize)> = ordered_updates.iter().map(|u| (u.id, u.index)).collect();
    // println!(get_app_state(state)
    Ok(result)
}
