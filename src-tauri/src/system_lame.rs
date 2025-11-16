use std::io::{self, Write};
use std::path::Path;
use std::process::Command;
use tempfile::NamedTempFile;

pub struct SystemLameEncoder {
    bitrate: u32,
    quality: u8,
}

impl SystemLameEncoder {
    pub fn new(bitrate: u32, quality: u8) -> Self {
        Self { bitrate, quality }
    }

    pub fn encode_to_mp3<P: AsRef<Path>>(
        &self,
        samples: &[f32],
        sample_rate: u32,
        channels: u16,
        output_path: P,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Create temporary WAV file
        let mut temp_wav = NamedTempFile::new()?;

        // Write WAV header and samples to temp file
        self.write_wav_file(&mut temp_wav, samples, sample_rate, channels)?;

        // Get paths
        let temp_wav_path = temp_wav.path();
        let output_path = output_path.as_ref();

        // Use system lame command to encode
        let output = Command::new("lame")
            .arg("--preset")
            .arg("standard")
            .arg("-b")
            .arg(self.bitrate.to_string())
            .arg("-q")
            .arg(self.quality.to_string())
            .arg(temp_wav_path)
            .arg(output_path)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("LAME encoding failed: {}", stderr).into());
        }

        Ok(())
    }

    fn write_wav_file(
        &self,
        file: &mut NamedTempFile,
        samples: &[f32],
        sample_rate: u32,
        channels: u16,
    ) -> io::Result<()> {
        let bytes_per_sample = 2; // 16-bit
        let data_size = samples.len() * bytes_per_sample;
        let file_size = 36 + data_size;

        // WAV header
        file.write_all(b"RIFF")?;
        file.write_all(&(file_size as u32).to_le_bytes())?;
        file.write_all(b"WAVE")?;

        // Format chunk
        file.write_all(b"fmt ")?;
        file.write_all(&16u32.to_le_bytes())?; // chunk size
        file.write_all(&1u16.to_le_bytes())?; // format (PCM)
        file.write_all(&channels.to_le_bytes())?;
        file.write_all(&sample_rate.to_le_bytes())?;
        file.write_all(&(sample_rate * channels as u32 * bytes_per_sample as u32).to_le_bytes())?; // byte rate
        file.write_all(&(channels * bytes_per_sample as u16).to_le_bytes())?; // block align
        file.write_all(&16u16.to_le_bytes())?; // bits per sample

        // Data chunk
        file.write_all(b"data")?;
        file.write_all(&(data_size as u32).to_le_bytes())?;

        // Convert f32 samples to i16 and write
        for &sample in samples {
            let sample_i16 = (sample.clamp(-1.0, 1.0) * 32767.0) as i16;
            file.write_all(&sample_i16.to_le_bytes())?;
        }

        file.flush()?;
        Ok(())
    }
}

// Compatibility types for existing code
pub struct Builder {
    bitrate: u32,
    quality: u8,
}

impl Builder {
    pub fn new() -> Self {
        Self {
            bitrate: 128,
            quality: 2,
        }
    }

    pub fn bitrate(mut self, bitrate: Bitrate) -> Self {
        self.bitrate = match bitrate {
            Bitrate::Kbps32 => 32,
            Bitrate::Kbps40 => 40,
            Bitrate::Kbps48 => 48,
            Bitrate::Kbps56 => 56,
            Bitrate::Kbps64 => 64,
            Bitrate::Kbps80 => 80,
            Bitrate::Kbps96 => 96,
            Bitrate::Kbps112 => 112,
            Bitrate::Kbps128 => 128,
            Bitrate::Kbps160 => 160,
            Bitrate::Kbps192 => 192,
            Bitrate::Kbps224 => 224,
            Bitrate::Kbps256 => 256,
            Bitrate::Kbps320 => 320,
        };
        self
    }

    pub fn quality(mut self, quality: Quality) -> Self {
        self.quality = match quality {
            Quality::Best => 0,
            Quality::High => 2,
            Quality::Good => 5,
            Quality::Fast => 7,
            Quality::Fastest => 9,
        };
        self
    }

    pub fn build(self) -> SystemLameEncoder {
        SystemLameEncoder::new(self.bitrate, self.quality)
    }
}

// Compatibility enums
#[derive(Clone, Copy)]
pub enum Bitrate {
    Kbps32,
    Kbps40,
    Kbps48,
    Kbps56,
    Kbps64,
    Kbps80,
    Kbps96,
    Kbps112,
    Kbps128,
    Kbps160,
    Kbps192,
    Kbps224,
    Kbps256,
    Kbps320,
}

#[derive(Clone, Copy)]
pub enum Quality {
    Best,
    High,
    Good,
    Fast,
    Fastest,
}

// Stub types for compatibility
pub struct DualPcm;
pub struct FlushNoGap;
pub struct Id3Tag;

pub fn max_required_buffer_size(_samples: usize) -> usize {
    // Not needed for system command approach
    0
}
