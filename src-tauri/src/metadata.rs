use std::path::Path;

use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::probe::ProbeResult;
use symphonia::default::get_probe;

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
