impl PlayableOp for MergePlaybackNode {
    fn render(&mut self, t: SampleTime, frames: usize, out: &mut [f32]) {
        out.fill(0.0);

        let end = t + frames as u64;

        for (i, input) in self.inputs.iter_mut().enumerate() {
            let start = self.offsets[i];
            let stop = start + input.duration().unwrap();

            if end <= start || t >= stop {
                continue;
            }

            let local_t = t.saturating_sub(start);
            let local_frames = frames.min((stop - t) as usize);

            input.render(local_t, local_frames, out);
        }
    }

    fn duration(&self) -> Option<SampleTime> {
        Some(self.total_duration)
    }

    fn spec(&self) -> AudioSpec {
        self.spec
    }
}
