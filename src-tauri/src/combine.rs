use crate::error::Error;
use crate::AppState;
use hound::{SampleFormat, WavSpec, WavWriter};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::sync::Arc;
use std::time::Instant;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::default::{get_codecs, get_probe};
use sysinfo::System;
use tauri::{AppHandle, Emitter, Manager, State}; // Add to Cargo.toml

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CombineAudioResult {
    output: String,
    svg_path: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CombineEvent {
    progress: f32,
}

#[tauri::command]
pub fn combine_audio_files(
    input_files: Vec<String>,
    output_path: String,
    state: State<'_, Arc<AppState>>,
    _app_handle: tauri::AppHandle,
) -> Result<CombineAudioResult, Error> {
    let start_total = Instant::now();
    let mut all_samples: Vec<i16> = vec![];

    log::info!(
        "Got request to combine_audio_files for {} files, output to {}",
        input_files.len(),
        output_path
    );

    for (i, file_path) in input_files.iter().enumerate() {
        let start_file = Instant::now();
        println!("Decoding: {}", file_path);

        let samples = get_samples(file_path)?;
        log::info!(
            "Decoded {} samples from {} in {:.2?}",
            samples.len(),
            file_path,
            start_file.elapsed()
        );

        all_samples.extend(&samples);

        let progress_value = ((i + 1) as f32 / input_files.len() as f32);
        let _ = _app_handle.emit("combine-audio-progress", progress_value);
        // Store into AppState if needed
        // if let Ok(mut audio_files) = state.audio_files.lock() {
        //     audio_files.push((file_path.clone(), samples));
        // }
    }

    let start_write = Instant::now();
    log::info!("Writing output to {}", &output_path);

    let spec = WavSpec {
        channels: 2,
        sample_rate: 44100,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    let mut writer = WavWriter::create(&output_path, spec)?;
    for sample in &all_samples {
        writer.write_sample(*sample)?;
    }

    writer.finalize()?;

    log::info!(
        "Wrote {} samples from {} files in {:.2?}",
        all_samples.len(),
        input_files.len(),
        start_write.elapsed()
    );

    log::info!("Total time: {:.2?}", start_total.elapsed());

    Ok(CombineAudioResult {
        output: output_path,
        svg_path: generate_waveform_path(&all_samples, 1000, 70),
    })
}

pub fn generate_waveform_path(samples: &[i16], width: usize, height: usize) -> String {
    let samples_per_pixel = samples.len() / width.max(1);
    let mid_y = height as f32 / 2.0;
    let amplitude_scale = mid_y / i16::MAX as f32;

    let mut d = String::new();
    for x in 0..width {
        let start = x * samples_per_pixel;
        let end = ((x + 1) * samples_per_pixel).min(samples.len());

        let slice = &samples[start..end];
        if slice.is_empty() {
            continue;
        }

        let min = *slice.iter().min().unwrap_or(&0) as f32;
        let max = *slice.iter().max().unwrap_or(&0) as f32;

        let y1 = mid_y - max * amplitude_scale;
        let y2 = mid_y - min * amplitude_scale;

        let x_pos = x as f32;

        // Use vertical bars (like Logic Pro / SoundCloud style)
        d.push_str(&format!("M{x_pos:.1},{y1:.1} L{x_pos:.1},{y2:.1} "));
    }

    d
}

#[tauri::command]
pub fn update_inputs() {
    println!("SHOULD UPDATE")
}

fn get_samples(file_path: &str) -> Result<Vec<i16>, Error> {
    let file = File::open(file_path).map_err(|_| Error::InvalidPath)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let probed = get_probe()
        .format(
            &Default::default(),
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|_| Error::InvalidPath)?;

    let mut format = probed.format;
    let track = format.default_track().ok_or(Error::NoDefaultTrackFound)?;
    let mut decoder = get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|_| Error::InvalidPath)?;

    let mut samples: Vec<i16> = Vec::new();

    while let Ok(packet) = format.next_packet() {
        let decoded = decoder.decode(&packet).map_err(|_| Error::InvalidPath)?;
        let spec = *decoded.spec();
        let mut sample_buf = SampleBuffer::<i16>::new(decoded.capacity() as u64, spec);
        sample_buf.copy_interleaved_ref(decoded);
        samples.extend(sample_buf.samples().iter().copied());
    }

    Ok(samples)
}
