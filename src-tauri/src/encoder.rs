use crate::state::AppState;
use crate::Error;
use flacenc::bitsink::ByteSink;
use flacenc::component::BitRepr;
use flacenc::config::Encoder as FlacConfig;
use flacenc::encode_with_fixed_block_size;
use flacenc::error::Verify;
use flacenc::source::MemSource;
use mp3lame_encoder::{
    max_required_buffer_size, Bitrate, Builder, DualPcm, FlushNoGap, Id3Tag, Quality,
};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::sync::Arc;
use std::{collections::HashMap, fs::File, io::BufWriter, path::Path};
use tauri::ipc::Channel;
use tauri::State;

pub enum AudioFormat {
    Wav,
    Mp3,
    Flac,
}

pub trait AudioEncoder {
    fn encode(
        &self,
        samples: &[f32],
        settings: &ExportSettings,
        channel: Channel<ExportAudioEvent>,
    ) -> Result<Vec<u8>, Error>;
    fn file_extension(&self) -> &'static str;
    fn mime_type(&self) -> &'static str;
    fn write(
        &self,
        samples: &[f32],
        settings: &ExportSettings,
        path: &str,
        channel: Channel<ExportAudioEvent>,
    ) -> Result<&'static str, Error> {
        let data = self.encode(samples, settings, channel.clone())?;
        let file = File::create(Path::new(path))?;
        let mut writer = BufWriter::new(file);
        writer.write_all(&data)?;
        writer.flush()?;

        // --- Send "finished" event with actual output path ---
        let _ = channel.send(ExportAudioEvent::Finished {
            output_path: path.to_string(),
            message: format!("Successfully exported to {path}"),
        });

        Ok(self.file_extension())
    }
}

pub struct WavEncoder;

impl AudioEncoder for WavEncoder {
    fn encode(
        &self,
        samples: &[f32],
        settings: &ExportSettings,
        channel: Channel<ExportAudioEvent>,
    ) -> Result<Vec<u8>, Error> {
        use hound::{SampleFormat, WavSpec, WavWriter};
        use std::io::Cursor;

        println!("=== WAV ENCODER ===");
        println!("Input samples: {}", samples.len());
        println!("Target sample rate: {}Hz", settings.sample_rate);
        println!(
            "Target bit depth: {}bit (encoding as 16-bit PCM)",
            settings.bit_depth
        );
        println!("Target channels: {} (encoding as mono)", settings.channels);
        println!("==================");

        let _ = channel.send(ExportAudioEvent::Started {
            output_path: "wav-encoding".into(),
            message: format!(
                "WAV encoding: {}Hz, 16-bit PCM, mono ({} samples)",
                settings.sample_rate,
                samples.len()
            ),
        });

        let mut buffer = Cursor::new(Vec::new());
        let spec = WavSpec {
            channels: 1,
            sample_rate: settings.sample_rate,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };

        let mut writer = WavWriter::new(&mut buffer, spec)?;

        let total_samples = samples.len();
        for (i, &sample) in samples.iter().enumerate() {
            let s = (sample * i16::MAX as f32).clamp(i16::MIN as f32, i16::MAX as f32) as i16;
            writer.write_sample(s)?;

            // Send progress updates every 10% or on last sample
            if i % (total_samples / 10 + 1) == 0 || i == total_samples - 1 {
                let progress = (i + 1) as f32 / total_samples as f32;
                let _ = channel.send(ExportAudioEvent::Progress {
                    progress,
                    message: format!("Encoding WAV: {}/{} samples", i + 1, total_samples),
                });
            }
        }

        println!("WAV encoding completed, finalizing file...");
        writer.finalize()?;
        println!("WAV file finalized successfully");

        Ok(buffer.into_inner())
    }

    fn file_extension(&self) -> &'static str {
        "wav"
    }
    fn mime_type(&self) -> &'static str {
        "audio/wav"
    }
}

pub struct FlacEncoder;

impl AudioEncoder for FlacEncoder {
    fn encode(
        &self,
        samples: &[f32],
        settings: &ExportSettings,
        channel: Channel<ExportAudioEvent>,
    ) -> Result<Vec<u8>, Error> {
        let num_channels = settings.channels as usize;
        let bits_per_sample = settings.bit_depth as usize;

        println!("=== FLAC ENCODER ===");
        println!("Input samples: {}", samples.len());
        println!("Target sample rate: {}Hz", settings.sample_rate);
        println!("Target bit depth: {}bit", settings.bit_depth);
        println!("Target channels: {}", settings.channels);
        println!("Estimated PCM frames: {}", samples.len() / num_channels);
        println!("===================");

        if samples.len() % num_channels != 0 {
            return Err(Error::UnevenNumberOfSamples);
        }

        channel.send(ExportAudioEvent::Progress {
            progress: -1.,
            message: format!("Converting to i32 PCM ({}bit depth)", bits_per_sample),
        });

        // Convert to i32 PCM (flacenc expects i32 sample slices)
        let max_value = match bits_per_sample {
            8 => 127.0,
            16 => i16::MAX as f32,
            24 => 8388607.0,
            32 => i32::MAX as f32,
            _ => i16::MAX as f32,
        };

        let to_i32 = |s: f32| ((s.clamp(-1.0, 1.0)) * max_value).round() as i32;
        let pcm: Vec<i32> = samples.iter().map(|&s| to_i32(s)).collect();

        println!(
            "Converted {} samples to {}-bit PCM",
            pcm.len(),
            bits_per_sample
        );

        // --- Send "started" event ---
        let _ = channel.send(ExportAudioEvent::Started {
            output_path: "flac-encoding".into(),
            message: format!(
                "FLAC encoding: {}Hz, {}bit, {}ch ({} PCM samples)",
                settings.sample_rate,
                settings.bit_depth,
                settings.channels,
                pcm.len()
            ),
        });

        channel.send(ExportAudioEvent::Progress {
            progress: -1.,
            message: "Building FLAC encoder configuration".into(),
        });

        // Build encoder config
        let config = FlacConfig::default()
            .into_verified()
            .map_err(|_| Error::FlacEncodeError("Invalid config".into()))?;

        channel.send(ExportAudioEvent::Progress {
            progress: -1.,
            message: "Building FLAC memory source".into(),
        });

        // Build MemSource with actual settings
        let source = MemSource::from_samples(
            &pcm,
            num_channels,
            bits_per_sample,
            settings.sample_rate as usize,
        );

        println!(
            "Built FLAC MemSource: {} channels, {} bit depth, {} sample rate",
            num_channels, bits_per_sample, settings.sample_rate
        );

        channel.send(ExportAudioEvent::Progress {
            progress: -1.,
            message: "Encoding to FLAC stream".into(),
        });
        // Encode into a FLAC stream
        let flac_stream = encode_with_fixed_block_size(&config, source, config.block_size)
            .map_err(|_| Error::FlacEncodeError("FLAC encode error".into()))?;

        println!(
            "FLAC stream encoded with {} frames",
            flac_stream.frame_count()
        );

        // Write to sink (Vec<u8>)
        let mut sink = ByteSink::new();

        let use_custom = true;

        if use_custom {
            let total_frames = flac_stream.frame_count();
            println!(
                "Writing {} FLAC frames with progress tracking",
                total_frames
            );

            for i in 0..total_frames {
                flac_stream
                    .frame(i)
                    .unwrap()
                    .write(&mut sink)
                    .map_err(|_| Error::FlacOutputError("FLAC frame write error".into()))?;

                // throttle progress updates every 1% or last frame
                if i % (total_frames / 100 + 1) == 0 || i == total_frames - 1 {
                    let progress = (i + 1) as f32 / total_frames as f32;
                    println!("FLAC encoding progress: {:.1}%", progress * 100.0);
                    let _ = channel.send(ExportAudioEvent::Progress {
                        progress,
                        message: format!("Encoded FLAC frame {}/{}", i + 1, total_frames),
                    });
                }
            }
            // Custom per-frame write with progress
        } else {
            // Use library’s default write method (no per-frame progress)
            flac_stream
                .write(&mut sink)
                .map_err(|_| Error::FlacOutputError("Flac sink error".into()))?;
        }

        flac_stream
            .write(&mut sink)
            .map_err(|_| Error::FlacOutputError("Flac sink error".into()))?;

        Ok(sink.as_slice().to_vec())
    }

    fn file_extension(&self) -> &'static str {
        "flac"
    }

    fn mime_type(&self) -> &'static str {
        "audio/flac"
    }
}

pub struct Mp3Encoder;

impl AudioEncoder for Mp3Encoder {
    fn encode(
        &self,
        samples: &[f32],
        settings: &ExportSettings,
        channel: Channel<ExportAudioEvent>,
    ) -> Result<Vec<u8>, Error> {
        let num_channels = settings.channels as usize;

        println!("🎧 === MP3 ENCODER ===");
        println!("🔢 Input samples: {}", samples.len());
        println!("🔊 Target sample rate: {}Hz", settings.sample_rate);
        println!(
            "🎚️  Target bit depth: {}bit (encoding as stereo 16-bit)",
            settings.bit_depth
        );
        println!(
            "📡 Target channels: {} (encoding as stereo for compatibility)",
            settings.channels
        );
        println!("⚡ Target bitrate: {}kbps", settings.bitrate.unwrap_or(192));
        println!("⭐ Quality: Best");
        println!(
            "🏷️  ID3 Tag: Title='{}', Artist='Sound Stitch'",
            settings.filename
        );
        println!("🎧 ==================");

        if samples.len() % num_channels != 0 {
            return Err(Error::UnevenNumberOfSamples);
        }

        // --- Send "started" event ---
        let _ = channel.send(ExportAudioEvent::Started {
            output_path: "mp3-encoding".into(),
            message: format!(
                "MP3 encoding: {}Hz, stereo, {}kbps, best quality ({} input samples)",
                settings.sample_rate,
                settings.bitrate.unwrap_or(192),
                samples.len()
            ),
        });

        channel.send(ExportAudioEvent::Progress {
            progress: -1.,
            message: "Converting to u16 PCM for MP3 encoding".into(),
        });

        let to_u16 = |s: f32| (((s.clamp(-1.0, 1.0) + 1.0) / 2.0) * u16::MAX as f32).round() as u16;

        let mut left = Vec::with_capacity(samples.len() / num_channels);
        let mut right = Vec::with_capacity(samples.len() / num_channels);

        channel.send(ExportAudioEvent::Progress {
            progress: -1.,
            message: "Separating and processing audio channels for MP3".into(),
        });

        // Handle mono vs stereo
        if num_channels == 1 {
            // Mono: duplicate to both channels
            println!("🔄 Processing mono input: duplicating to stereo channels");
            for sample in samples {
                let converted = to_u16(*sample);
                left.push(converted);
                right.push(converted);
            }
        } else {
            // Stereo or multi-channel: take first two channels
            println!(
                "🔄 Processing {} channel input: using first 2 channels for stereo",
                num_channels
            );
            for chunk in samples.chunks_exact(num_channels) {
                left.push(to_u16(chunk[0]));
                right.push(to_u16(if num_channels > 1 { chunk[1] } else { chunk[0] }));
            }
        }

        println!("✅ Prepared {} stereo frames for MP3 encoding", left.len());

        channel.send(ExportAudioEvent::Progress {
            progress: -1.,
            message: format!(
                "Configuring MP3 encoder: {}kbps, {}Hz, stereo, best quality",
                settings.bitrate.unwrap_or(192),
                settings.sample_rate
            ),
        });

        // Configure encoder with settings
        let mut builder =
            Builder::new().ok_or_else(|| Error::MP3EncoderError("Failed to build".to_string()))?;

        println!("⚙️  Setting MP3 encoder parameters...");

        builder
            .set_num_channels(2) // Always encode as stereo for compatibility
            .map_err(|e| Error::MP3EncoderError(e.to_string()))?;
        println!("  ✅ Channels: 2 (stereo)");

        builder
            .set_sample_rate(settings.sample_rate)
            .map_err(|e| Error::MP3EncoderError(e.to_string()))?;
        println!("  ✅ Sample rate: {}Hz", settings.sample_rate);

        // Use the bitrate from settings
        let bitrate = convert_bitrate(settings.bitrate);
        builder
            .set_brate(bitrate)
            .map_err(|e| Error::MP3EncoderError(e.to_string()))?;
        println!("  ✅ Bitrate: {}kbps", settings.bitrate.unwrap_or(192));

        builder
            .set_quality(Quality::Best)
            .map_err(|e| Error::MP3EncoderError(e.to_string()))?;
        println!("  ✅ Quality: Best");

        // Set ID3 tag with filename if available
        let title = settings.filename.as_bytes();
        builder.set_id3_tag(Id3Tag {
            title,
            artist: b"Sound Stitch",
            album: b"Exported Audio",
            year: b"2025",
            comment: b"Exported from Sound Stitch",
            album_art: &[],
        });
        println!(
            "  ✅ ID3 tags: Title='{}', Artist='Sound Stitch'",
            settings.filename
        );

        let mut encoder = builder
            .build()
            .map_err(|e| Error::MP3EncoderError(e.to_string()))?;

        println!("🚀 MP3 encoder configured successfully. Starting encoding process...");

        let mut mp3_out = Vec::new();

        // --- Encode in blocks ---
        let block_size = 4096; // frames per channel per block
        let total_frames = left.len();
        let mut processed = 0;

        println!(
            "🔄 Encoding {} frames in blocks of {} frames",
            total_frames, block_size
        );
        println!(
            "📊 Estimated output size: ~{} KB",
            (total_frames as u32 * settings.bitrate.unwrap_or(192)) / (8 * 1000)
        );

        for (l_chunk, r_chunk) in left.chunks(block_size).zip(right.chunks(block_size)) {
            let input = DualPcm {
                left: l_chunk,
                right: r_chunk,
            };

            // Reserve enough capacity for this block-
            mp3_out.reserve(max_required_buffer_size(input.left.len()));
            let encoded = encoder
                .encode(input, mp3_out.spare_capacity_mut())
                .map_err(|e| Error::MP3EncoderError(e.to_string()))?;
            unsafe { mp3_out.set_len(mp3_out.len() + encoded) };

            processed += l_chunk.len();
            let progress = processed as f32 / total_frames as f32;

            // --- Send progress update ---
            let _ = channel.send(ExportAudioEvent::Progress {
                progress,
                message: format!(
                    "MP3 encoding: {}/{} frames ({:.1}%)",
                    processed,
                    total_frames,
                    progress * 100.0
                ),
            });

            // Print progress every 25%
            if processed % (total_frames / 4 + 1) == 0 {
                println!(
                    "📈 MP3 encoding progress: {}/{} frames ({:.1}%)",
                    processed,
                    total_frames,
                    progress * 100.0
                );
            }
        }

        // Flush
        println!("🔄 Flushing MP3 encoder...");
        let flushed = encoder
            .flush::<FlushNoGap>(mp3_out.spare_capacity_mut())
            .map_err(|e| Error::MP3EncoderError(e.to_string()))?;
        unsafe { mp3_out.set_len(mp3_out.len() + flushed) };

        println!(
            "🎉 MP3 encoding completed successfully. Output size: {} bytes ({:.2} KB)",
            mp3_out.len(),
            mp3_out.len() as f64 / 1024.0
        );

        Ok(mp3_out)
    }

    fn file_extension(&self) -> &'static str {
        "mp3"
    }

    fn mime_type(&self) -> &'static str {
        "audio/mpeg"
    }
}

impl AudioFormat {
    pub fn get_encoder(&self) -> Box<dyn AudioEncoder> {
        match self {
            AudioFormat::Wav => Box::new(WavEncoder),
            AudioFormat::Mp3 => Box::new(Mp3Encoder),
            AudioFormat::Flac => Box::new(FlacEncoder),
        }
    }
}

pub struct EncoderRegistry {
    encoders: HashMap<&'static str, Box<dyn AudioEncoder>>,
}

impl EncoderRegistry {
    pub fn new() -> Self {
        let mut encoders: HashMap<&'static str, Box<dyn AudioEncoder>> = HashMap::new();
        encoders.insert("wav", Box::new(WavEncoder));
        encoders.insert("mp3", Box::new(Mp3Encoder));
        encoders.insert("flac", Box::new(FlacEncoder));
        Self { encoders }
    }

    pub fn get(&self, format: &str) -> Option<&Box<dyn AudioEncoder>> {
        self.encoders.get(format)
    }
}

#[derive(Clone, Serialize)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "event",
    content = "data"
)]
pub enum ExportAudioEvent {
    Started {
        output_path: String,
        message: String,
    },
    Progress {
        progress: f32,
        message: String,
    },
    Finished {
        output_path: String,
        message: String,
    },
}

#[derive(Clone, Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ExportSettings {
    pub sample_rate: u32,
    pub bit_depth: u16,
    pub channels: u16,
    pub format: String,
    pub filename: String,
    pub bitrate: Option<u32>,
}

// Helper function to convert u32 bitrate to LAME Bitrate enum
fn convert_bitrate(bitrate: Option<u32>) -> Bitrate {
    match bitrate {
        Some(32) => Bitrate::Kbps32,
        Some(40) => Bitrate::Kbps40,
        Some(48) => Bitrate::Kbps48,
        Some(64) => Bitrate::Kbps64,
        Some(80) => Bitrate::Kbps80,
        Some(96) => Bitrate::Kbps96,
        Some(112) => Bitrate::Kbps112,
        Some(128) => Bitrate::Kbps128,
        Some(160) => Bitrate::Kbps160,
        Some(192) => Bitrate::Kbps192,
        Some(224) => Bitrate::Kbps224,
        Some(256) => Bitrate::Kbps256,
        Some(320) => Bitrate::Kbps320,
        _ => Bitrate::Kbps192, // Default fallback
    }
}

#[tauri::command]
pub async fn export_audio(
    settings: ExportSettings,
    output_file: String,
    state: State<'_, Arc<AppState>>,
    on_event: Channel<ExportAudioEvent>,
) -> Result<String, Error> {
    let state = state.inner().clone();

    tauri::async_runtime::spawn_blocking(move || {
        // lock audio_files
        let audio_files = state.audio_files.lock().unwrap();
        println!("🎵 ENCODING STARTED of {} audio files", audio_files.len());

        if audio_files.len() == 0 {
            return Err(Error::UnknownEncoderFormat(
                "No audio files to export".into(),
            ));
        }

        // find the total length needed for combined audio
        let total_length: usize = audio_files.values().map(|file| file.samples.len()).sum();

        // allocate combined samples buffer
        let mut combined_samples: Vec<f32> = Vec::with_capacity(total_length);

        // Print comprehensive export settings information
        println!("🎵 === EXPORT SETTINGS ===");
        println!("📁 Format: {}", settings.format.to_uppercase());
        println!("🏷️  Filename: {}", settings.filename);
        println!("🔊 Sample Rate: {}Hz", settings.sample_rate);
        println!("🎚️  Bit Depth: {}bit", settings.bit_depth);
        println!("📡 Channels: {}", settings.channels);
        if let Some(bitrate) = settings.bitrate {
            println!("⚡ Bitrate: {}kbps", bitrate);
        }
        println!("💾 Output File: {}", output_file);
        println!("📂 Input Files: {} audio files", audio_files.len());
        println!("🎵 =======================");

        on_event
            .send(ExportAudioEvent::Started {
                output_path: output_file.clone(),
                message: format!(
                    "Encoding {} files to {} ({}Hz, {}bit, {}ch{}) → {}",
                    audio_files.len(),
                    settings.format.to_uppercase(),
                    settings.sample_rate,
                    settings.bit_depth,
                    settings.channels,
                    if let Some(bitrate) = settings.bitrate {
                        format!(", {}kbps", bitrate)
                    } else {
                        String::new()
                    },
                    settings.filename
                ),
            })
            .unwrap();

        // iterate through all audio files and append their samples
        for file in audio_files.values() {
            combined_samples.extend(file.samples.iter().map(|&s| s as f32 / i16::MAX as f32));
        }

        println!(
            "📊 Num Samples: {}, Target format: {}",
            combined_samples.len(),
            settings.format
        );

        // set up encoder
        let registry = EncoderRegistry::new();
        let encoder = registry
            .get(&settings.format)
            .ok_or(Error::UnknownEncoderFormat(settings.format.clone()))?;

        // write combined samples to file using settings
        encoder.write(&combined_samples, &settings, &output_file, on_event)?;

        Ok(format!(
            "Encoded combined audio to {} with settings: {}Hz, {}bit, {}ch{}",
            output_file,
            settings.sample_rate,
            settings.bit_depth,
            settings.channels,
            if let Some(bitrate) = settings.bitrate {
                format!(", {}kbps", bitrate)
            } else {
                String::new()
            }
        ))
    })
    .await?
}
