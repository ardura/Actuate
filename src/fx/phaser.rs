use std::f32::consts::PI;

#[derive(Clone, Copy)]
struct AllpassDelay {
    a1: f32,
    zm1: f32,
}

impl AllpassDelay {
    fn new() -> Self {
        AllpassDelay { a1: 0.0, zm1: 0.0 }
    }

    fn delay(&mut self, delay: f32) {
        self.a1 = (1.0 - delay) / (1.0 + delay);
    }

    fn update(&mut self, in_samp: f32) -> f32 {
        let y = in_samp * -self.a1 + self.zm1;
        self.zm1 = y * self.a1 + in_samp;
        y
    }
}

#[derive(Clone, Copy)]
pub struct StereoPhaser {
    alps: [AllpassDelay; 6],
    dmin: f32,
    dmax: f32,
    fb: f32,
    lfo_phase: f32,
    lfo_inc: f32,
    depth: f32,
    zm1: f32,
    sample_rate: f32,
}

impl StereoPhaser {
    pub fn new() -> Self {
        let mut phaser = StereoPhaser {
            alps: [AllpassDelay::new(); 6],
            dmin: 0.0,
            dmax: 0.0,
            fb: 0.7,
            lfo_phase: 0.0,
            lfo_inc: 0.0,
            depth: 1.0,
            zm1: 0.0,
            sample_rate: 44100.0,
        };
        phaser.range(440.0, 1600.0);
        phaser.set_rate(0.5);
        phaser
    }

    pub fn range(&mut self, f_min: f32, f_max: f32) {
        self.dmin = f_min / (self.sample_rate / 2.0);
        self.dmax = f_max / (self.sample_rate / 2.0);
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
    }

    pub fn set_rate(&mut self, rate: f32) {
        self.lfo_inc = 2.0 * PI * (rate / self.sample_rate);
    }

    pub fn set_feedback(&mut self, fb: f32) {
        self.fb = fb;
    }

    pub fn set_depth(&mut self, depth: f32) {
        self.depth = depth;
    }

    pub fn process(&mut self, left_in: f32, right_in: f32, amount: f32) -> (f32, f32) {
        let d = self.dmin + (self.dmax - self.dmin) * ((self.lfo_phase.sin() + 1.0) / 2.0);
        self.lfo_phase += self.lfo_inc;
        self.lfo_phase %= PI * 2.0;

        for alp in &mut self.alps {
            alp.delay(d);
        }

        let left_out = self
            .alps
            .iter_mut()
            .fold(left_in + self.zm1 * self.fb, |acc, alp| alp.update(acc));

        let right_out = self
            .alps
            .iter_mut()
            .fold(right_in + self.zm1 * self.fb, |acc, alp| alp.update(acc));

        self.zm1 = left_out;

        let output_l = left_out + left_in * self.depth;
        let output_r = right_out + right_in * self.depth;

        (
            output_l * amount + left_in * (1.0 - amount),
            output_r * amount + right_in * (1.0 - amount),
        )
    }
}
