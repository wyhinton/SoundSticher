use crate::playback::op_playback::{AudioSpec, PlayableOp, PlaybackResult, SampleTime};

pub struct MergePlaybackOp {
    inputs: Vec<Box<dyn PlayableOp>>,
    offsets: Vec<SampleTime>,
    total_duration: SampleTime,
    spec: AudioSpec,
}

impl PlayableOp for MergePlaybackOp {
    fn render_at(
        &mut self,
        t: SampleTime,
        out: &mut [f32],
        spec: &AudioSpec,
    ) -> PlaybackResult<usize> {
        // Clear output buffer
        out.fill(0.0);

        let frames = out.len() / spec.channels as usize;
        let end = t + SampleTime::new(frames as u64);
        let mut samples_written = 0;

        // Create a temporary buffer for mixing
        let mut temp_buffer = vec![0.0f32; out.len()];

        for (i, input) in self.inputs.iter_mut().enumerate() {
            let start = self.offsets[i];

            // Get the duration of this input, skip if it doesn't have one
            let input_duration = match input.duration() {
                Some(dur) => dur,
                None => continue, // Skip infinite sources for now
            };

            let stop = start + input_duration;

            // Skip if this input doesn't overlap with the requested time range
            if end <= start || t >= stop {
                continue;
            }

            // Calculate the local time within this input
            let local_t = if t >= start {
                t - start
            } else {
                SampleTime::new(0) // Start from beginning if we're before the input start
            };

            // Clear temp buffer for this input
            temp_buffer.fill(0.0);

            // Render this input into the temp buffer
            let input_samples = input.render_at(local_t, &mut temp_buffer, spec)?;

            // Mix the input into the output buffer
            for (out_sample, &temp_sample) in out.iter_mut().zip(temp_buffer.iter()) {
                *out_sample += temp_sample;
            }

            samples_written = samples_written.max(input_samples);
        }

        Ok(samples_written)
    }

    fn duration(&self) -> Option<SampleTime> {
        Some(self.total_duration)
    }

    fn spec(&self) -> AudioSpec {
        self.spec
    }
}
