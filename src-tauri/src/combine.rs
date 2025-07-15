use hound::{SampleFormat, WavSpec, WavWriter};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::Path;
use std::time::Instant;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::default::{get_codecs, get_probe};
use sysinfo::System;

use crate::error::Error; // Add to Cargo.toml

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CombineAudioResult {
    output: String,
    svg_path: String,
}

#[tauri::command]
pub fn combine_audio_files(
    input_files: Vec<String>,
    output_path: String,
) -> Result<CombineAudioResult, Error> {
    let start_total = Instant::now();
    let mut all_samples: Vec<i16> = vec![];
    let mut sys = System::new_all();
    log::info!(
        "Got request to combine_audio_files for {} files, out put to {}",
        &input_files.len(),
        &output_path
    );

    for file_path in &input_files {
        let start_file = Instant::now();
        println!("Decoding: {}", file_path);

        let file = File::open(file_path.clone())?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let probed = match get_probe().format(
            &Default::default(),
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        ) {
            Ok(probe) => probe,
            Err(e) => {
                eprintln!("Error doing: {}", e);
                return Err(Error::InvalidPath);
            }
        };

        let mut format = probed.format;
        let track = format.default_track().ok_or(Error::NoDefaultTrackFound);
        let mut decoder =
            get_codecs().make(&track.unwrap().codec_params, &DecoderOptions::default())?;

        let mut sample_count = 0;

        while let Ok(packet) = format.next_packet() {
            let decoded = decoder.decode(&packet)?;
            let spec = *decoded.spec();
            let mut sample_buf = SampleBuffer::<i16>::new(decoded.capacity() as u64, spec);
            sample_buf.copy_interleaved_ref(decoded);
            sample_count += sample_buf.len();
            all_samples.extend(sample_buf.samples().iter());
        }

        log::info!(
            "Decoded {} samples from {} in {:.2?}",
            sample_count,
            file_path,
            start_file.elapsed()
        );
    }

    let start_write = Instant::now();
    log::info!("Writing output to {}", &output_path);

    let spec = WavSpec {
        channels: 2,
        sample_rate: 44100,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    let mut writer = WavWriter::create(output_path.clone(), spec)?;
    for sample in &all_samples {
        writer.write_sample(*sample)?;
    }

    writer.finalize()?;
    log::info!(
        "Wrote {} samples from {} files in {:.2?}",
        all_samples.len(),
        &input_files.len(),
        start_write.elapsed()
    );

    // Optional: Print memory usage
    // sys.refresh_processes();
    // if let Some(process) = sys.process(sysinfo::get_current_pid().unwrap()) {
    //     println!("Memory: {:.2} MB", process.memory() as f64 / 1024.0);
    //     println!("CPU Usage: {:.2}%", process.cpu_usage());
    // }

    log::info!("Total time: {:.2?}", start_total.elapsed());
    return Ok(CombineAudioResult {
        output: output_path,
        svg_path: generate_waveform_path(&all_samples, 1000, 70),
    });
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
