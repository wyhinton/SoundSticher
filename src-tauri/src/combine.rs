use crate::error::Error;
use crate::AppState;
use hound::{SampleFormat, WavSpec, WavWriter};
use rodio::buffer::SamplesBuffer;
use rodio::{OutputStream, Sink};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::File;
use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::default::{get_codecs, get_probe};
use tauri::{AppHandle, Emitter, Manager, State}; // Add to Cargo.toml

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CombineAudioResult {
    output: String,
    svg_path: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CachedCombineResult {
    svg_path: String,
    duration: f64,
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
        svg_path: generate_waveform_path(&all_samples, 1000, 70, 0.),
    })
}

pub fn generate_waveform_path(samples: &[i16], width: usize, height: usize, offset: f64) -> String {
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

        let x_pos = x as f32 + offset as f32;

        // Use vertical bars (like Logic Pro / SoundCloud style)
        d.push_str(&format!("M{x_pos:.1},{y1:.1} L{x_pos:.1},{y2:.1} "));
    }

    d
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AudioSend {
    path: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Section {
    folderPath: String,
    paths: Vec<AudioSend>,
}

#[tauri::command]
pub fn update_inputs(
    sections: Vec<Section>,
    state: State<'_, Arc<AppState>>,
    _app_handle: tauri::AppHandle,
) -> Result<String, Error> {
    println!("➡️➡️ SHOULD UPDATE");

    let mut audio_files = state.audio_files.lock().unwrap();
    let mut inserted_count = 0;
    let mut removed_count = 0;
    // Step 1: Collect all valid paths from the incoming sections
    let valid_paths: HashSet<String> = sections
        .iter()
        .flat_map(|section| section.paths.iter().map(|audio| audio.path.clone()))
        .collect();
    println!("p: {:?}", valid_paths);
    // Step 2: Remove any paths from the HashMap that are not in valid_paths
    audio_files.retain(|path, _| {
        if valid_paths.contains(path) {
            true
        } else {
            println!("Removed unused path {}", path);
            removed_count += 1;
            false
        }
    });

    // Step 3: Insert any missing paths
    let mut combined: Vec<i16> = Vec::new();

    for (i, path) in valid_paths.iter().enumerate() {
        if !audio_files.contains_key(path) {
            let samples = get_samples(path)?;
            combined.extend(&samples); // Add to combined audio
            audio_files.insert(path.clone(), samples);
            println!("Inserted path {}", path);
            inserted_count += 1;
            let progress = (i as f32) / (valid_paths.len() as f32);
            let _ = _app_handle.emit("buffering-progress", progress);
        } else {
            println!("Path {} already exists", path);
        }
    }

    // Store combined samples in AppState
    let mut combined_audio = state.combined_audio.lock().unwrap();
    *combined_audio = Some(combined);

    Ok(format!(
        "Successfully inserted {} new path(s), removed {} unused path(s).",
        inserted_count, removed_count
    ))
}

#[tauri::command]
pub fn combine_all_cached_samples(
    state: State<'_, Arc<AppState>>,
    app: AppHandle,
) -> Result<String, Error> {
    let state = Arc::clone(&state); // Clone for thread
    let app = app.clone(); // Clone for thread

    std::thread::spawn(move || {
        state.cancel_flag.store(false, Ordering::Relaxed);

        let audio_files = state.audio_files.lock().unwrap();
        let sample_rate = 44100.0;
        let full_waveform_width = 1000.0;

        let mut combined_samples: Vec<i16> = Vec::new();
        let mut total_samples = 0;

        // First pass: Count total samples
        for (_, samples) in audio_files.iter() {
            if state.cancel_flag.load(Ordering::Relaxed) {
                println!("🛑 Stopped during total sample calculation");
                let _ = app.emit(
                    "combined-cached",
                    CachedCombineResult {
                        svg_path: String::new(),
                        duration: 0.0,
                    },
                );
                return;
            }
            total_samples += samples.len();
        }

        let duration = total_samples as f64 / sample_rate;
        let _ = app.emit("total-length", duration);

        if total_samples == 0 {
            let _ = app.emit(
                "combined-cached",
                CachedCombineResult {
                    svg_path: String::new(),
                    duration: 0.0,
                },
            );
            println!("✅ No samples to combine");
            return;
        }

        let mut current_sample_offset = 0;

        // Second pass: process and emit segments
        for (path, samples) in audio_files.iter() {
            if state.cancel_flag.load(Ordering::Relaxed) {
                println!("🛑 Stopped while adding samples");
                let _ = app.emit(
                    "combined-cached",
                    CachedCombineResult {
                        svg_path: String::new(),
                        duration: 0.0,
                    },
                );
                return;
            }

            println!("Adding {} samples from {}", samples.len(), path);
            combined_samples.extend(samples);

            let duration = combined_samples.len() as f64 / sample_rate;
            let segment_width = full_waveform_width * (samples.len() as f64 / total_samples as f64);
            let x_offset =
                full_waveform_width * (current_sample_offset as f64 / total_samples as f64);

            let svg_path = generate_waveform_path(samples, segment_width as usize, 70, x_offset);

            let _ = app.emit("processed-segment", svg_path);

            current_sample_offset += samples.len();
        }

        println!("✅ Successfully combined all samples");
        let _ = app.emit("combine-complete", ());
    });

    Ok("⏳ Combining started in background thread".to_string())
}

#[tauri::command]
pub fn cancel_combine(state: State<'_, Arc<AppState>>) -> Result<(), Error> {
    state.cancel_flag.store(true, Ordering::Relaxed);
    println!("🚨 Cancellation flag set");
    Ok(())
}

#[tauri::command]
pub fn export_combined_audio_as_wav(
    state: State<'_, Arc<AppState>>,
    outputPath: String,
) -> Result<String, String> {
    // Get a lock on the combined audio
    let combined_audio = state.combined_audio.lock().unwrap();
    let Some(samples) = &*combined_audio else {
        return Err("No combined audio available".to_string());
    };

    if samples.is_empty() {
        return Err("Combined audio is empty".to_string());
    }

    // Define WAV format
    let spec = WavSpec {
        channels: 2,
        sample_rate: 44100,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    // Create file
    let path = Path::new(&outputPath);
    let writer = WavWriter::create(&path, spec).map_err(|e| e.to_string())?;

    // Write samples
    let mut writer = writer;
    for sample in samples {
        writer.write_sample(*sample).map_err(|e| e.to_string())?;
    }

    writer.finalize().map_err(|e| e.to_string())?;

    Ok(format!("WAV file successfully saved to {}", outputPath))
}

#[tauri::command]
pub fn play_combined_audio(
    state: State<'_, Arc<AppState>>,
    app: AppHandle,
    start_seconds: Option<f32>,
) {
    let state = state.inner().clone();

    thread::spawn(move || {
        let mut current_song = state.current_song.lock().unwrap();

        // If already playing or paused, resume instead
        // if let Some(sink) = &*current_song {
        //     if sink.is_paused() {
        //         println!("RESUMING");
        //         sink.play(); // resume
        //         return;
        //     } else if !sink.empty() {
        //         eprintln!("Audio is already playing.");
        //         return;
        //     }
        // }

        // Fetch audio
        let combined_samples = {
            let guard = state.combined_audio.lock().unwrap();
            guard.clone()
        };

        let Some(samples) = combined_samples else {
            eprintln!("No combined audio available.");
            return;
        };

        if samples.is_empty() {
            eprintln!("Combined audio is empty.");
            return;
        }

        let sample_rate = 44100;
        let channels = 2;
        let total_samples = samples.len();
        let start_sample_index =
            (start_seconds.unwrap_or(0.0) * sample_rate as f32 * channels as f32).round() as usize;

        if start_sample_index >= total_samples {
            eprintln!("Start time exceeds audio length.");
            return;
        }

        let trimmed_samples = &samples[start_sample_index..];
        let (_stream, stream_handle) = match OutputStream::try_default() {
            Ok(output) => output,
            Err(e) => {
                eprintln!("Error creating audio output stream: {}", e);
                return;
            }
        };

        let sink = match Sink::try_new(&stream_handle) {
            Ok(sink) => Arc::new(sink),
            Err(e) => {
                eprintln!("Error creating sink: {}", e);
                return;
            }
        };

        let duration = trimmed_samples.len() as f32 / (channels as f32 * sample_rate as f32);
        let start = Instant::now();

        let source = SamplesBuffer::new(channels as u16, sample_rate, trimmed_samples.to_vec());
        sink.append(source);
        sink.set_volume(1.0);
        sink.play();

        *current_song = Some(Arc::clone(&sink));

        let sink_clone = Arc::clone(&sink);
        thread::spawn(move || {
            while !sink_clone.empty() {
                let elapsed = start.elapsed().as_secs_f32();
                let progress = (elapsed / duration).min(1.0);
                let _ = app.emit("combined-progress", progress);
                std::thread::sleep(Duration::from_millis(200));
            }

            let _ = app.emit("combined-progress", 1.0);
        });

        sink.sleep_until_end();
    });
}

#[tauri::command]
pub fn pause_combined_audio(state: State<'_, Arc<AppState>>) {
    println!("PAUSING");
    let current_song = state.current_song.lock().unwrap();
    if let Some(sink) = &*current_song {
        sink.pause();
    }
}

#[tauri::command]
pub fn resume_combined_audio(state: State<'_, Arc<AppState>>) {
    let current_song = state.current_song.lock().unwrap();
    if let Some(sink) = &*current_song {
        sink.play();
    }
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
