// This is a simplified version of Pop2 by Airwindows converted to rust by Ardura

#[derive(Clone, Copy)]
pub(crate) struct Compressor {
    sample_rate: f32,
    // 0 to 1 for each like AW
    amount: f32,
    attack: f32,
    release: f32,
    drive: f32,
    // Data holding variables
    speed_l: f32,
    speed_r: f32,
    coefficient_l: f32,
    coefficient_r: f32,
}

impl Compressor {
    pub fn new(sample_rate: f32, amount: f32, attack: f32, release: f32, drive: f32) -> Self {
        Compressor {
            sample_rate: sample_rate,
            amount: amount,
            attack: attack,
            release: release,
            drive: drive,
            speed_l: 1000.0,
            speed_r: 1000.0,
            coefficient_l: 1.0,
            coefficient_r: 1.0,
        }
    }
    pub fn update(&mut self, sample_rate: f32, amount: f32, attack: f32, release: f32, drive: f32) {
        self.sample_rate = sample_rate;
        let overallscale = self.sample_rate / 44100.0;
        self.amount = amount;
        self.attack = (attack.powi(4) * 100000.0 + 10.0) * overallscale;
        self.release = (release.powi(5) * 2000000.0 + 20.0) * overallscale;
        self.drive = drive;
    }
    pub fn process(&mut self, input_l: f32, input_r: f32) -> (f32, f32) {
        let threshold = 1.0 - ((1.0 - (1.0 - self.amount).powi(2)) * 0.9);
        let max_release = self.release * 4.0;
        let mu_makeup_gain = (1.0 / threshold).sqrt() * self.drive;

        // Start by getting pregain based off threshold
        let pre_gain = 1.0 / threshold;
        let mut output_l = input_l * pre_gain;
        let mut output_r = input_r * pre_gain;

        // Adjust coefficients for L
        if output_l.abs() > threshold {
            let variance = threshold / output_l.abs();
            let mu_attack_l = (self.speed_l.abs()).sqrt();
            self.coefficient_l = self.coefficient_l * (mu_attack_l - 1.0)
                + if variance < threshold {
                    threshold
                } else {
                    variance
                };
            self.coefficient_l = self.coefficient_l / mu_attack_l;
            let mu_new_speed_l = self.speed_l * (self.speed_l - 1.0) + self.release;
            self.speed_l = mu_new_speed_l / self.speed_l;
            self.speed_l = self.speed_l.min(max_release);
        } else {
            self.coefficient_l = self.coefficient_l * (self.speed_l.powi(2) - 1.0) + 1.0;
            self.coefficient_l = self.coefficient_l / (self.speed_l.powi(2));
            let mu_new_speed_l = self.speed_l * (self.speed_l - 1.0) + self.attack;
            self.speed_l = mu_new_speed_l / self.speed_l;
        }

        // Adjust coefficients for R
        if output_r.abs() > threshold {
            let variance = threshold / output_r.abs();
            let mu_attack_r = (self.speed_r.abs()).sqrt();
            self.coefficient_r = self.coefficient_r * (mu_attack_r - 1.0)
                + if variance < threshold {
                    threshold
                } else {
                    variance
                };
            self.coefficient_r = self.coefficient_r / mu_attack_r;
            let mu_new_speed_r = self.speed_r * (self.speed_r - 1.0) + self.release;
            self.speed_r = mu_new_speed_r / self.speed_r;
            self.speed_r = self.speed_r.min(max_release);
        } else {
            self.coefficient_r = self.coefficient_r * (self.speed_r.powi(2) - 1.0) + 1.0;
            self.coefficient_r = self.coefficient_r / (self.speed_r.powi(2));
            let mu_new_speed_r = self.speed_r * (self.speed_r - 1.0) + self.attack;
            self.speed_r = mu_new_speed_r / self.speed_r;
        }

        self.coefficient_l = self.coefficient_l.powi(2);
        self.coefficient_r = self.coefficient_r.powi(2);
        output_l *= mu_makeup_gain;
        output_r *= mu_makeup_gain;
        (output_l, output_r)
    }
}
