use std::path::Path;
use std::sync::{Arc, Mutex};

use lofty::file::AudioFile;
use lofty::read_from_path;
use serde::Deserialize;
use serde::Serialize;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::probe::ProbeResult;
use symphonia::default::get_probe;
use tauri::State;

use crate::duration_cache::DurationCache;
use crate::error::Error;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileMetadata {
    pub path: String,
    pub size: Option<u64>,
    pub bit_rate: Option<u32>,
    pub channels: Option<u8>,
    pub bit_depth: Option<u8>,
    pub duration: u128,
}

// #[tauri::command]
// pub fn get_metadata(title: String) -> Result<FileMetadata, Error> {
//     let tagged_file = read_from_path(&title);
//     let meta = match tagged_file {
//         Ok(taggedFile) => {
//             let props = taggedFile.properties();
//             log::info!("✅ Successfully retrieved metadata for: {}", title);
//             return Ok(FileMetadata {
//                 path: title.clone(),
//                 size: get_file_size(title.clone()),
//                 bitRate: props.audio_bitrate(),
//                 channels: props.channels(),
//                 bitDepth: props.bit_depth(),
//                 duration: props.duration().as_millis(),
//             });
//         }
//         Err(e) => {
//             eprintln!("Error doing metadata: {}", e);
//             return Err(Error::InvalidPath);
//         }
//     };
// }

#[tauri::command]
pub fn get_metadata(titles: Vec<String>) -> Result<Vec<FileMetadata>, Error> {
    let mut results = Vec::new();

    for title in titles {
        match read_from_path(&title) {
            Ok(tagged_file) => {
                let props = tagged_file.properties();
                // log::info!("✅ Successfully retrieved metadata for: {}", title);
                results.push(FileMetadata {
                    path: title.clone(),
                    size: get_file_size(title.clone()),
                    bit_rate: props.audio_bitrate(),
                    channels: props.channels(),
                    bit_depth: props.bit_depth(),
                    duration: props.duration().as_millis(),
                });
            }
            Err(e) => {
                eprintln!("⚠️ Failed to get metadata for {}: {}", title, e);
                // Optional: skip or return Err here
            }
        }
    }

    Ok(results)
}

/// Lightweight duration-only command for waveform width calculation
/// Uses the duration cache to avoid redundant audio parsing
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FileDuration {
    pub path: String,
    pub duration_seconds: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
struct GetFileSizeResponse {
    file_size: Option<u64>,
}
fn get_file_size(path: String) -> Option<u64> {
    if let Ok(metadata) = std::fs::metadata(path) {
        return Some(metadata.len());
    }
    None
}
