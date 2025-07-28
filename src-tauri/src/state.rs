use rodio::Sink;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tauri::State;

pub struct AppState {
    pub current_song: Mutex<Option<Arc<Sink>>>,
    pub audio_files: Mutex<BTreeMap<String, Vec<i16>>>,
    pub combined_audio: Mutex<Option<Vec<i16>>>,
    pub cancel_flag: AtomicBool,
    pub cancel_playback: AtomicBool,
    pub buffering_samples: AtomicBool,
    pub svg_path: Mutex<Option<String>>,
    pub cancel_token: AtomicU64,
}

#[derive(Serialize)]
pub struct SerializableAppState {
    pub audio_files: BTreeMap<String, usize>,
    pub combined_audio: String,
    pub cancel_flag: bool,
    pub buffering_samples: bool,
    pub svg_path: String,
    pub cancel_token: u64,
}

#[tauri::command]
pub fn get_app_state(state: State<'_, Arc<AppState>>) -> SerializableAppState {
    let audio_files = state.audio_files.lock().unwrap();

    let audio_file_lengths: BTreeMap<String, usize> = audio_files
        .iter()
        .map(|(path, samples)| (path.clone(), samples.len()))
        .collect();

    let combined = state.combined_audio.lock().unwrap();
    let combined_audio_string = match &*combined {
        Some(data) => data.len().to_string(),
        None => "empty".to_string(),
    };
    let svg_path_mutex = state.svg_path.lock().unwrap();
    let svg_string = match &*svg_path_mutex {
        Some(data) => data.clone(),
        None => "no svg path".to_string(),
    };

    SerializableAppState {
        audio_files: audio_file_lengths,
        combined_audio: combined_audio_string,
        cancel_flag: state.cancel_flag.load(Ordering::Relaxed),
        buffering_samples: state.buffering_samples.load(Ordering::Relaxed),
        svg_path: svg_string,
        cancel_token: state.cancel_token.load(Ordering::Relaxed),
    }
}
