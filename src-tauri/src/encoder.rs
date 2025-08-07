use crate::Error;
use hound::WavWriter;
use mp3lame_encoder::{
    max_required_buffer_size, Bitrate, Builder, DualPcm, FlushNoGap, Id3Tag, Quality,
};
use std::io::Write;
use std::{collections::HashMap, fs::File, io::BufWriter, path::Path};

pub enum AudioFormat {
    Wav,
    Mp3,
}

pub trait AudioEncoder {
    fn encode(&self, samples: &[f32], sample_rate: u32) -> Result<Vec<u8>, Error>;
    fn file_extension(&self) -> &'static str;
    fn mime_type(&self) -> &'static str;
    fn write(&self, samples: &[f32], sample_rate: u32, path: &str) -> Result<&'static str, Error> {
        let data = self.encode(samples, sample_rate)?;

        let file = File::create(Path::new(path))?;
        let mut writer = BufWriter::new(file);
        writer.write_all(&data)?;
        writer.flush()?;

        Ok(self.file_extension())
    }
}

pub struct WavEncoder;

impl AudioEncoder for WavEncoder {
    fn encode(&self, samples: &[f32], sample_rate: u32) -> Result<Vec<u8>, Error> {
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

pub struct Mp3Encoder;

impl AudioEncoder for Mp3Encoder {
    fn encode(&self, samples: &[f32], sample_rate: u32) -> Result<Vec<u8>, Error> {
        let num_channels = 2;

        if samples.len() % num_channels != 0 {
            return Err(Error::UnevenNumberOfSamples);
        }

        // Convert f32 samples in [-1.0, 1.0] to u16 PCM in [0, u16::MAX]
        let to_u16 = |s: f32| (((s.clamp(-1.0, 1.0) + 1.0) / 2.0) * u16::MAX as f32).round() as u16;

        let mut left = Vec::with_capacity(samples.len() / 2);
        let mut right = Vec::with_capacity(samples.len() / 2);

        for chunk in samples.chunks_exact(2) {
            left.push(to_u16(chunk[0]));
            right.push(to_u16(chunk[1]));
        }

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

        let input = DualPcm {
            left: &left,
            right: &right,
        };

        let mut mp3_out = Vec::new();
        mp3_out.reserve(max_required_buffer_size(input.left.len()));

        let encoded = encoder
            .encode(input, mp3_out.spare_capacity_mut())
            .map_err(|e| Error::MP3EncoderError(e.to_string()))?;
        unsafe {
            mp3_out.set_len(mp3_out.len() + encoded);
        }

        let flushed = encoder
            .flush::<FlushNoGap>(mp3_out.spare_capacity_mut())
            .map_err(|e| Error::MP3EncoderError(e.to_string()))?;
        unsafe {
            mp3_out.set_len(mp3_out.len() + flushed);
        }

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
        Self { encoders }
    }

    pub fn get(&self, format: &str) -> Option<&Box<dyn AudioEncoder>> {
        self.encoders.get(format)
    }
}

#[tauri::command]
pub fn export_audio(
    samples: Vec<f32>,
    sample_rate: u32,
    format: String,
    output_file: String,
) -> Result<String, Error> {
    let registry = EncoderRegistry::new();
    let encoder = registry
        .get(&format)
        .ok_or(Error::UnknownEncoderFormat(format))?;
    encoder.write(&samples, sample_rate, &output_file)?;
    Ok("Encoded Audio".to_string())
}
