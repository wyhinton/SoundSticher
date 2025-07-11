use std::path::Path;

use lofty::file::AudioFile;
use lofty::probe::Probe;
use lofty::read_from_path;
use serde::Deserialize;
use serde::Serialize;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::probe::ProbeResult;
use symphonia::default::get_probe;

use lofty;

use crate::error::Error;

pub fn get_duration(path: &str) -> Option<f32> {
    let file = std::fs::File::open(path).ok()?;
    // let mreader = std::io::BufReader::new(file);
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    let path = Path::new(&path);
    if let Some(extension) = path.extension() {
        hint.with_extension(&extension.to_string_lossy());
    };

    let probed: ProbeResult = get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .ok()?;

    let format = probed.format;
    let track = format.default_track().or_else(|| format.tracks().get(0))?;

    let duration = track.codec_params.n_frames?;
    let sample_rate = track.codec_params.sample_rate?;

    let length = duration as f32 / sample_rate as f32;
    log::info!(
        "Got duration: {}, sample_rate: {}, length: {}",
        duration,
        sample_rate,
        length
    );
    Some(duration as f32 / sample_rate as f32)
}

#[derive(serde::Serialize)]
pub struct FileMetadata {
    pub path: String,
    pub size: Option<u64>,
    pub bitRate: Option<u32>,
    pub channels: Option<u8>,
    pub bitDepth: Option<u8>,
    pub duration: u128,
}

#[tauri::command]
pub fn get_metadata(title: String) -> Result<FileMetadata, Error> {
    let tagged_file = read_from_path(&title);
    let meta = match tagged_file {
        Ok(taggedFile) => {
            let props = taggedFile.properties();
            return Ok(FileMetadata {
                path: title.clone(),
                size: get_file_size(title.clone()),
                bitRate: props.audio_bitrate(),
                channels: props.channels(),
                bitDepth: props.bit_depth(),
                duration: props.duration().as_millis(),
            });
        }
        Err(e) => {
            eprintln!("Error doing metadata: {}", e);
            return Err(Error::InvalidPath);
        }
    };
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
    return None;
}
