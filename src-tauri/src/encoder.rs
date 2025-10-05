use crate::state::AppState;
use crate::Error;
use flacenc::bitsink::BitSink;
use flacenc::bitsink::ByteSink;
use flacenc::component::BitRepr;
use flacenc::config::Encoder as FlacConfig;
use flacenc::encode_with_fixed_block_size;
use flacenc::error::Verify;
use flacenc::source::MemSource;
use hound::WavWriter;
use mp3lame_encoder::{
    max_required_buffer_size, Bitrate, Builder, DualPcm, FlushNoGap, Id3Tag, Quality,
};
use serde::Serialize;
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
        sample_rate: u32,
        channel: Channel<ExportAudioEvent>,
    ) -> Result<Vec<u8>, Error>;
    fn file_extension(&self) -> &'static str;
    fn mime_type(&self) -> &'static str;
    fn write(
        &self,
        samples: &[f32],
        sample_rate: u32,
        path: &str,
        channel: Channel<ExportAudioEvent>,
    ) -> Result<&'static str, Error> {
        let data = self.encode(samples, sample_rate, channel)?;
        let file = File::create(Path::new(path))?;
        let mut writer = BufWriter::new(file);
        writer.write_all(&data)?;
        writer.flush()?;

        Ok(self.file_extension())
    }
}

pub struct WavEncoder;

impl AudioEncoder for WavEncoder {
    fn encode(
        &self,
        samples: &[f32],
        sample_rate: u32,
        channel: Channel<ExportAudioEvent>,
    ) -> Result<Vec<u8>, Error> {
        use hound::{SampleFormat, WavSpec, WavWriter};
        use std::io::Cursor;

        let mut buffer = Cursor::new(Vec::new());
        let spec = WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };

        let mut writer = WavWriter::new(&mut buffer, spec)?;
        for &sample in samples {
            let s = (sample * i16::MAX as f32).clamp(i16::MIN as f32, i16::MAX as f32) as i16;
            writer.write_sample(s)?;
        }
        println!("FINALIZED WAV ");

        writer.finalize()?;
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
        sample_rate: u32,
        channel: Channel<ExportAudioEvent>,
    ) -> Result<Vec<u8>, Error> {
        let num_channels = 2;
        let bits_per_sample = 16;

        if samples.len() % num_channels != 0 {
            return Err(Error::UnevenNumberOfSamples);
        }

        channel.send(ExportAudioEvent::Progress {
            progress: -1.,
            message: "Converting to i32 PCM".into(),
        });
        // Convert to i32 PCM (flacenc expects i32 sample slices)
        let to_i32 = |s: f32| ((s.clamp(-1.0, 1.0)) * i16::MAX as f32).round() as i32;

        let pcm: Vec<i32> = samples.iter().map(|&s| to_i32(s)).collect();

        // --- Send "started" event ---
        let _ = channel.send(ExportAudioEvent::Started {
            output_path: "in-memory".into(),
            message: "FLAC encoding started".into(),
        });

        channel.send(ExportAudioEvent::Progress {
            progress: -1.,
            message: "Building encoder config".into(),
        });
        // Build encoder config
        let config = FlacConfig::default()
            .into_verified()
            .map_err(|_| Error::FlacEncodeError("Invalid config".into()))?;
        channel.send(ExportAudioEvent::Progress {
            progress: -1.,
            message: "Building mem source".into(),
        });
        // Build MemSource
        let source =
            MemSource::from_samples(&pcm, num_channels, bits_per_sample, sample_rate as usize);
        channel.send(ExportAudioEvent::Progress {
            progress: -1.,
            message: "Encoding to flac stream".into(),
        });
        // Encode into a FLAC stream
        let flac_stream = encode_with_fixed_block_size(&config, source, config.block_size)
            .map_err(|_| Error::FlacEncodeError("Flac encode error".into()))?;

        // Write to sink (Vec<u8>)
        let mut sink = ByteSink::new();

        let use_custom = true;

        if use_custom {
            let total_frames = flac_stream.frame_count();
            for i in 0..total_frames {
                flac_stream
                    .frame(i)
                    .unwrap()
                    .write(&mut sink)
                    .map_err(|_| Error::FlacOutputError("Flac frame write error".into()))?;

                // throttle progress updates every 1% or last frame
                if i % (total_frames / 100 + 1) == 0 || i == total_frames - 1 {
                    let progress = (i + 1) as f32 / total_frames as f32;
                    println!("PROGRESS {}", progress);
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

        // --- Send "finished" event ---
        let _ = channel.send(ExportAudioEvent::Finished {
            output_path: "in-memory".into(),
        });

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
        sample_rate: u32,
        channel: Channel<ExportAudioEvent>,
    ) -> Result<Vec<u8>, Error> {
        let num_channels = 2;

        if samples.len() % num_channels != 0 {
            return Err(Error::UnevenNumberOfSamples);
        }
        channel.send(ExportAudioEvent::Progress {
            progress: -1.,
            message: "Converting to u16 PCM".into(),
        });
        let to_u16 = |s: f32| (((s.clamp(-1.0, 1.0) + 1.0) / 2.0) * u16::MAX as f32).round() as u16;

        let mut left = Vec::with_capacity(samples.len() / 2);
        let mut right = Vec::with_capacity(samples.len() / 2);

        channel.send(ExportAudioEvent::Progress {
            progress: -1.,
            message: "Chucking channel samples".into(),
        });
        for chunk in samples.chunks_exact(2) {
            left.push(to_u16(chunk[0]));
            right.push(to_u16(chunk[1]));
        }

        channel.send(ExportAudioEvent::Progress {
            progress: -1.,
            message: "Building encoder".into(),
        });
        // Configure encoder
        let mut builder =
            Builder::new().ok_or_else(|| Error::MP3EncoderError("Failed to build".to_string()))?;
        builder
            .set_num_channels(2)
            .map_err(|e| Error::MP3EncoderError(e.to_string()))?;
        builder
            .set_sample_rate(sample_rate)
            .map_err(|e| Error::MP3EncoderError(e.to_string()))?;
        builder
            .set_brate(Bitrate::Kbps192)
            .map_err(|e| Error::MP3EncoderError(e.to_string()))?;
        builder
            .set_quality(Quality::Best)
            .map_err(|e| Error::MP3EncoderError(e.to_string()))?;
        builder.set_id3_tag(Id3Tag {
            title: b"My title",
            artist: b"My artist",
            album: b"My album",
            year: b"2025",
            comment: b"Exported from Rust",
            album_art: &[],
        });

        let mut encoder = builder
            .build()
            .map_err(|e| Error::MP3EncoderError(e.to_string()))?;

        let mut mp3_out = Vec::new();

        // --- Encode in blocks ---
        let block_size = 4096; // frames per channel per block
        let total_frames = left.len();
        let mut processed = 0;

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
                message: format!("Encoding chunk {}", processed),
            });
        }

        // Flush
        let flushed = encoder
            .flush::<FlushNoGap>(mp3_out.spare_capacity_mut())
            .map_err(|e| Error::MP3EncoderError(e.to_string()))?;
        unsafe { mp3_out.set_len(mp3_out.len() + flushed) };

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
    },
}

#[tauri::command]
pub async fn export_audio(
    sample_rate: u32,
    format: String,
    output_file: String,
    state: State<'_, Arc<AppState>>,
    on_event: Channel<ExportAudioEvent>,
) -> Result<String, Error> {
    let state = state.inner().clone();

    tauri::async_runtime::spawn_blocking(move || {
        // lock audio_files
        let audio_files = state.audio_files.lock().unwrap();
        println!("ENCODING STARTED of {} audio files", audio_files.len());
        if (audio_files.len() == 0) {
            return Err(Error::UnknownEncoderFormat(
                "No audio files to export".into(),
            ));
        }

        // find the total length needed for combined audio
        let total_length: usize = audio_files.values().map(|file| file.samples.len()).sum();

        // allocate combined samples buffer
        let mut combined_samples: Vec<f32> = Vec::with_capacity(total_length);
        on_event
            .send(ExportAudioEvent::Started {
                output_path: output_file.clone(),
                message: format!(
                    "Encoding {} files, with {}",
                    &audio_files.len(),
                    &combined_samples.len()
                ),
            })
            .unwrap();
        // iterate through all audio files and append their samples
        for file in audio_files.values() {
            combined_samples.extend(file.samples.iter().map(|&s| s as f32 / i16::MAX as f32));
        }
        println!("Num Samples: {}", combined_samples.iter().len());
        // set up encoder
        let registry = EncoderRegistry::new();
        let encoder = registry
            .get(&format)
            .ok_or(Error::UnknownEncoderFormat(format))?;
        // write combined samples to file
        encoder.write(&combined_samples, sample_rate, &output_file, on_event)?;
        Ok(format!("Encoded combined audio to {}", &output_file))
    })
    .await?
}
