use hound;

pub struct Recorder {
    buffer: Vec<f32>,
    sample_rate: u32,
    max_samples: usize,
}

impl Recorder {
    pub fn new(sample_rate: f32, max_seconds: u32) -> Self {
        let max_samples = sample_rate as usize * max_seconds as usize * 2_usize;
        let sr_usize = sample_rate as u32;
        Self {
            buffer: Vec::with_capacity(max_samples),
            sample_rate: sr_usize,
            max_samples,
        }
    }

    pub fn reset(&mut self) {
        self.buffer.clear();
    }

    pub fn push(&mut self, left: f32, right: f32) {
        if self.buffer.len() + 2 <= self.max_samples {
            self.buffer.push(left);
            self.buffer.push(right);
        }
    }

    // Export WAV file
    pub fn export(&self, path: &str) -> hound::Result<()> {
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: self.sample_rate,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };

        let mut writer = hound::WavWriter::create(path, spec)?;
        let mut silence_tracker: Vec<f32> = Vec::new();
        for &sample in &self.buffer {
            if sample == 0.0 {
                silence_tracker.push(0.0);
            }
            if silence_tracker.len() > 100 {
                break;
            }
            writer.write_sample(sample)?;
        }
        writer.finalize()?;
        Ok(())
    }
}
