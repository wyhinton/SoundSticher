use rodio::Sink;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tauri::State;

#[derive(Clone)]
pub struct AudioFile {
    pub samples: Vec<i16>,
    pub start_offset: f64,
}

pub struct AppState {
    pub current_song: Mutex<Option<Arc<Sink>>>,
    pub audio_files: Mutex<BTreeMap<String, AudioFile>>,
    pub combined_audio: Mutex<Option<Vec<i16>>>,
    pub cancel_playback: AtomicBool,
    pub buffering_samples: AtomicBool,
    pub svg_path: Mutex<Option<String>>,
    pub cancel_token: AtomicU64,
    pub combine_process: Arc<Mutex<i32>>,
}

#[derive(Serialize)]
pub struct AudioFileDebug {
    samples: usize,
    start_offset: f64,
}

#[derive(Serialize)]
pub struct SerializableAppState {
    pub audio_files: BTreeMap<String, AudioFileDebug>,
    pub combined_audio: String,
    pub buffering_samples: bool,
    pub svg_path: String,
    pub cancel_token: u64,
    pub combine_process: i32,
}

#[tauri::command]
pub fn get_app_state(state: State<'_, Arc<AppState>>) -> SerializableAppState {
    let audio_files = state.audio_files.lock().unwrap();
    let audio_file_debugs: BTreeMap<String, AudioFileDebug> = audio_files
        .iter()
        .map(|(path, audio_file)| {
            (
                path.clone(),
                AudioFileDebug {
                    samples: audio_file.samples.len(),
                    start_offset: audio_file.start_offset,
                },
            )
        })
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
        audio_files: audio_file_debugs,
        combined_audio: combined_audio_string,
        buffering_samples: state.buffering_samples.load(Ordering::Relaxed),
        svg_path: svg_string,
        cancel_token: state.cancel_token.load(Ordering::Relaxed),
        combine_process: *state.combine_process.lock().unwrap(),
    }
}
