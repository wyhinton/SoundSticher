use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::State;

use crate::error::Error;
use crate::log_debug;
use crate::log_info;
use crate::logging::{LogSystem, LoggingService};

#[tauri::command]
pub fn count_audio_files_in_folders(
    folder_paths: Vec<String>,
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<HashMap<String, u32>, Error> {
    if let Ok(logger) = logging_service.lock() {
        log_info!(
            logger,
            LogSystem::Combine,
            &format!("Counting audio files in {} folder(s)", folder_paths.len())
        );
    }

    let mut file_counts: HashMap<String, u32> = HashMap::new();

    for folder_path in folder_paths {
        let mut count = 0u32;
        let entries = std::fs::read_dir(&folder_path)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(file_name) = path.file_name().and_then(|f| f.to_str()) {
                    if file_name.starts_with("._") {
                        continue;
                    }
                }

                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    let ext = ext.to_lowercase();
                    if [
                        "mp3", "wav", "flac", "ogg", "m4a", "aac", "aiff", "alac", "aif",
                    ]
                    .contains(&ext.as_str())
                    {
                        count += 1;
                    }
                }
            }
        }

        if let Ok(logger) = logging_service.lock() {
            log_debug!(
                logger,
                LogSystem::Combine,
                &format!("Found {} valid audio files in: {}", count, folder_path)
            );
        }

        println!("{}: {} files", folder_path, count);
        file_counts.insert(folder_path, count);
    }

    Ok(file_counts)
}
