use crate::error::Error;
use crate::logging::{LogSystem, LoggingService};
use crate::send_channel_event;
use crate::state::{AppState, AudioFile};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::File;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::default::{get_codecs, get_probe};
use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter, State}; // Add to Cargo.toml
use uuid::Uuid;

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

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StateChangeEvent {
    pub file_id: String,
    pub field: String,
    pub value: serde_json::Value,
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

#[derive(Clone, Serialize)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "event",
    content = "data"
)]
pub enum BufferAudioEvent {
    Started { content_length: usize },
    Progress { chunk_length: usize },
    Finished,
}

#[tauri::command]
pub async fn update_inputs(
    sections: Vec<Section>,
    state: State<'_, Arc<AppState>>,
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
    app_handle: tauri::AppHandle,
    on_event: Channel<BufferAudioEvent>,
) -> Result<String, Error> {
    let state = state.inner().clone();
    let logging_service = logging_service.inner().clone();
    let _current_token = state.cancel_token.fetch_add(1, Ordering::SeqCst) + 1;

    // Log operation start
    if let Ok(logger) = logging_service.lock() {
        logger.info(
            LogSystem::Combine,
            "Starting input updates",
            Some("update_inputs"),
        );
    } else {
        println!("RUNNING UPDATES");
    }

    // let count = state.combine_process.clone();
    // *count.lock().unwrap() += 1;

    tauri::async_runtime::spawn_blocking(move || {
        let mut audio_files = state.audio_files.lock().unwrap();
        let mut inserted_count = 0;
        let mut removed_count = 0;

        let valid_paths: HashSet<String> = sections
            .iter()
            .flat_map(|section| section.paths.iter().map(|audio| audio.path.clone()))
            .collect();

        send_channel_event!(
            on_event,
            BufferAudioEvent::Started {
                content_length: valid_paths.len(),
            }
        );

        // Collect IDs of files that will be removed
        let removed_ids: Vec<Uuid> = audio_files
            .iter()
            .filter_map(|(path, file)| {
                if !valid_paths.contains(path) {
                    Some(file.id)
                } else {
                    None
                }
            })
            .collect();

        audio_files.retain(|path, _| {
            if valid_paths.contains(path) {
                true
            } else {
                removed_count += 1;
                false
            }
        });

        // Remove corresponding IDs from custom_order
        if !removed_ids.is_empty() {
            let mut custom_order = state.custom_order.lock().unwrap();
            custom_order.retain(|id| !removed_ids.contains(id));
            println!("Removed {} IDs from custom_order", removed_ids.len());
        }

        let mut combined: Vec<i16> = Vec::new();

        // First pass: collect all samples and calculate total length
        let mut new_files: Vec<(String, Vec<i16>)> = Vec::new();
        let mut total_new_samples = 0;

        for path in valid_paths.iter() {
            if !audio_files.contains_key(path) {
                let samples = get_samples(path)?;
                total_new_samples += samples.len();
                new_files.push((path.clone(), samples));
            }
        }

        // Calculate total samples including existing files
        let existing_total_samples: usize = audio_files.values().map(|f| f.samples.len()).sum();
        let grand_total_samples = existing_total_samples + total_new_samples;

        //TODO: DUPLICATE FILES
        for (i, (path, samples)) in new_files.iter().enumerate() {
            combined.extend(samples);

            // Calculate proportional width based on relative length
            let relative_length = samples.len() as f64 / grand_total_samples as f64;
            let segment_width = 1000.0 * relative_length; // Proportional to 1000px total width

            // Generate waveform path when first inserting the sample
            let waveform_path = generate_waveform_path(
                samples,
                segment_width as usize, // Proportional width
                70,                     // Height
                0.0, // No offset, we'll translate this later based on start_offset
            );

            audio_files.insert(
                path.clone(),
                AudioFile {
                    samples: samples.clone(),
                    start_offset: 0.,
                    waveform_path,
                    id: Uuid::new_v4(),
                    path: path.clone(),
                    active: true, // Default to active
                },
            );
            let progress = (i as f32) / ((new_files.len() - 1).max(1) as f32);
            let _ = app_handle.emit("buffering-progress", progress);
            inserted_count += 1;
            println!("➡️📖 INSERTING {} into BTree", path.clone());
        }

        let mut combined_audio = state.combined_audio.lock().unwrap();
        *combined_audio = Some(combined);
        send_channel_event!(on_event, BufferAudioEvent::Finished);

        Ok(format!(
            "Inserted {}, removed {}.",
            inserted_count, removed_count
        ))
    })
    .await?
}

#[derive(Clone, Serialize)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "event",
    content = "data"
)]
pub enum CombineAudioEvent {
    Started {
        content_length: usize,
        duration: f64,
    },
    Progress {
        svg_path: String,
        start_offset: f64,
        file_name: String,
        size: f64,
        id: String,
        active: bool,
    },
    Finished {
        svg_path: String,
        empty: bool,
    },
}

#[tauri::command]
pub async fn combine_all_cached_samples(
    state: State<'_, Arc<AppState>>,
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
    app: AppHandle,
    on_event: Channel<CombineAudioEvent>,
    custom_order: Option<Vec<Uuid>>, // Optional custom order
) -> Result<String, Error> {
    let state = Arc::clone(&state); // Clone for thread
    let logging_service = logging_service.inner().clone();
    let app = app.clone(); // Clone for thread

    let count = state.combine_process.clone();
    *count.lock().unwrap() += 1;

    tauri::async_runtime::spawn_blocking(move || {
        let process_count = count.clone();
        let orig = *process_count.lock().unwrap();

        // Log operation start
        if let Ok(logger) = logging_service.lock() {
            logger.info_with_data(
                LogSystem::Combine,
                "Starting cached samples combination",
                Some("combine"),
                serde_json::json!({
                    "origin_count": orig,
                    "current_count": *count.lock().unwrap(),
                    "has_custom_order": custom_order.is_some()
                }),
            );
        } else {
            println!("ORIGIN: {}, COUNT: {}", orig, count.lock().unwrap());
        }

        state.buffering_samples.store(true, Ordering::Relaxed);
        let mut audio_files = state.audio_files.lock().unwrap();

        let sample_rate = 44100.0;
        let _full_waveform_width = 1000.0;

        let mut combined_samples: Vec<i16> = Vec::new();
        let mut total_samples = 0;

        // Collect files in the specified order (custom or default BTreeMap order)
        let ordered_files: Vec<AudioFile> = if let Some(order) = custom_order {
            println!("USING CUSTOM ORDER");
            // Use custom order, but filter by active status
            order
                .iter()
                .filter_map(|id| audio_files.values().find(|f| &f.id == id && f.active))
                .cloned()
                .collect()
        } else {
            println!("USING DEFAULT BTREE ORDER");
            // Use default BTreeMap order, but filter by active status
            audio_files.values().filter(|f| f.active).cloned().collect()
        };

        for audio_file in &ordered_files {
            println!("adding to samples: {}", audio_file.path);
            if *process_count.lock().unwrap() != orig {
                println!("🛑 Stopped while adding samples");
                return Ok("stopped".to_string());
            }
            total_samples += audio_file.samples.len();
        }

        let duration = total_samples as f64 / sample_rate;
        send_channel_event!(
            on_event,
            CombineAudioEvent::Started {
                content_length: ordered_files.len(),
                duration,
            }
        );
        if total_samples == 0 {
            // Emit specific event for no active samples
            send_channel_event!(
                on_event,
                CombineAudioEvent::Finished {
                    svg_path: String::new(),
                    empty: true,
                }
            );

            println!("⚠️ No active samples to combine");
            return Ok("No active samples".to_string());
        }

        let mut current_sample_offset = 0;
        let mut combined_svg_string = String::from("");

        // Process files in the specified order
        for audio_file in ordered_files {
            println!("audio file: {} ", audio_file.path.clone());
            if *process_count.lock().unwrap() != orig {
                println!("🛑 Stopped while adding samples");
                return Ok("stopped".to_string());
            }

            // Update the original file in the BTreeMap with new start_offset (no waveform regeneration)
            if let Some(original_file) = audio_files.values_mut().find(|f| f.id == audio_file.id) {
                original_file.start_offset =
                    (current_sample_offset as f64) / (total_samples as f64);
                combined_samples.extend(&audio_file.samples);

                let relative_length = audio_file.samples.len() as f64 / total_samples as f64;

                if *process_count.lock().unwrap() != orig {
                    println!("🛑 Stopped while adding samples");
                    return Ok("stopped".to_string());
                }

                // Use the pre-calculated waveform path (no regeneration needed)
                let svg_path = &original_file.waveform_path;

                send_channel_event!(
                    on_event,
                    CombineAudioEvent::Progress {
                        file_name: audio_file.path.clone(),
                        svg_path: svg_path.clone(),
                        start_offset: original_file.start_offset,
                        size: relative_length,
                        id: audio_file.id.to_string(),
                        active: audio_file.active,
                    }
                );
                if *process_count.lock().unwrap() != orig {
                    println!("🛑 Stopped while adding samples");
                    return Ok("stopped".to_string());
                }
                // sleep(Duration::from_millis(500)); // slow down 200ms per file
                combined_svg_string.push_str(svg_path);
                current_sample_offset += audio_file.samples.len();
            }
        }

        println!("✅ Successfully combined all samples");
        let _ = app.emit("combine-complete", ());
        state.buffering_samples.store(false, Ordering::Relaxed);

        // Store the combined samples in state
        let mut combined_audio = state.combined_audio.lock().unwrap();
        *combined_audio = Some(combined_samples);

        let mut state_svg_path = state.svg_path.lock().unwrap();
        send_channel_event!(
            on_event,
            CombineAudioEvent::Finished {
                svg_path: combined_svg_string.clone(),
                empty: false,
            }
        );
        *state_svg_path = Some(combined_svg_string);

        Ok("⏳ Combining started in background thread".to_string())
    })
    .await? // <-- This unwraps spawn_blocking Result
}
#[tauri::command]
pub async fn test_async(
    state: State<'_, Arc<AppState>>,
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
    app: AppHandle,
    _on_event: Channel<CombineAudioEvent>,
) -> Result<String, Error> {
    let state = Arc::clone(&state); // Clone for thread
    let logging_service = logging_service.inner().clone();
    let _app = app.clone(); // Clone for thread

    let count = state.combine_process.clone();
    *count.lock().unwrap() += 1;

    tauri::async_runtime::spawn_blocking(move || {
        if let Ok(logger) = logging_service.lock() {
            logger.debug(
                LogSystem::Combine,
                "Test async function called",
                Some("test"),
            );
        }
        sleep(Duration::from_millis(5000));
        Ok("⏳Did it".to_string())
    })
    .await? // <-- This unwraps spawn_blocking Result
}

#[tauri::command]
pub fn cancel_combine(_state: State<'_, Arc<AppState>>) -> Result<(), Error> {
    println!("🚨 Cancellation flag set");
    Ok(())
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

#[tauri::command]
pub fn get_custom_order(state: State<'_, Arc<AppState>>) -> Result<Vec<Uuid>, Error> {
    let custom_order = state.custom_order.lock().map_err(|_| Error::LockPoisoned)?;
    Ok(custom_order.clone())
}

#[tauri::command]
pub async fn combine_all_cached_samples_with_custom_order(
    state: State<'_, Arc<AppState>>,
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
    app: AppHandle,
    on_event: Channel<CombineAudioEvent>,
) -> Result<String, Error> {
    // Get the stored custom order
    let custom_order = {
        let order = state.custom_order.lock().map_err(|_| Error::LockPoisoned)?;
        if order.is_empty() {
            None
        } else {
            Some(order.clone())
        }
    };

    // Call the main combine function with the custom order
    combine_all_cached_samples(state, logging_service, app, on_event, custom_order).await
}

#[tauri::command]
pub fn toggle_audio_file_active(
    state: State<'_, Arc<AppState>>,
    app: AppHandle,
    file_id: String,
) -> Result<bool, Error> {
    let uuid = Uuid::parse_str(&file_id).map_err(|_| Error::InvalidPath)?;
    let mut audio_files = state.audio_files.lock().map_err(|_| Error::LockPoisoned)?;

    if let Some(file) = audio_files.values_mut().find(|f| f.id == uuid) {
        file.active = !file.active;
        println!(
            "Toggled file {} active status to: {}",
            file.path, file.active
        );

        // Emit state change event
        let _ = app.emit(
            "audio_file_state_changed",
            StateChangeEvent {
                file_id: uuid.to_string(),
                field: "active".to_string(),
                value: serde_json::Value::Bool(file.active),
            },
        );

        Ok(file.active)
    } else {
        Err(Error::InvalidPath)
    }
}

#[tauri::command]
pub fn set_audio_file_active(
    state: State<'_, Arc<AppState>>,
    app: AppHandle,
    file_id: String,
    active: bool,
) -> Result<(), Error> {
    let uuid = Uuid::parse_str(&file_id).map_err(|_| Error::InvalidPath)?;
    let mut audio_files = state.audio_files.lock().map_err(|_| Error::LockPoisoned)?;

    if let Some(file) = audio_files.values_mut().find(|f| f.id == uuid) {
        file.active = active;
        println!("Set file {} active status to: {}", file.path, active);

        // Emit state change event
        let _ = app.emit(
            "audio_file_state_changed",
            StateChangeEvent {
                file_id: uuid.to_string(),
                field: "active".to_string(),
                value: serde_json::Value::Bool(active),
            },
        );

        Ok(())
    } else {
        Err(Error::InvalidPath)
    }
}

#[tauri::command]
pub fn set_audio_files_active_batch(
    state: State<'_, Arc<AppState>>,
    app: AppHandle,
    file_ids: Vec<String>,
    active: bool,
) -> Result<usize, Error> {
    let mut audio_files = state.audio_files.lock().map_err(|_| Error::LockPoisoned)?;
    let mut updated_count = 0;

    // Parse all UUIDs first to validate them
    let uuids: Result<Vec<Uuid>, Error> = file_ids
        .iter()
        .map(|id| Uuid::parse_str(id).map_err(|_| Error::InvalidPath))
        .collect();

    let uuids = uuids?;

    // Update all files that match the provided IDs
    for uuid in uuids {
        if let Some(file) = audio_files.values_mut().find(|f| f.id == uuid) {
            file.active = active;
            updated_count += 1;
            println!("Set file {} active status to: {}", file.path, active);

            // Emit state change event
            let _ = app.emit(
                "audio_file_state_changed",
                StateChangeEvent {
                    file_id: uuid.to_string(),
                    field: "active".to_string(),
                    value: serde_json::Value::Bool(active),
                },
            );
        }
    }

    if updated_count == 0 {
        println!("Warning: No files were updated. Check if file IDs are valid.");
    } else {
        println!("Updated {} files to active: {}", updated_count, active);
    }

    Ok(updated_count)
}

#[tauri::command]
pub fn get_audio_file_active_status(
    state: State<'_, Arc<AppState>>,
    file_id: String,
) -> Result<bool, Error> {
    let uuid = Uuid::parse_str(&file_id).map_err(|_| Error::InvalidPath)?;
    let audio_files = state.audio_files.lock().map_err(|_| Error::LockPoisoned)?;

    if let Some(file) = audio_files.values().find(|f| f.id == uuid) {
        Ok(file.active)
    } else {
        Err(Error::InvalidPath)
    }
}
