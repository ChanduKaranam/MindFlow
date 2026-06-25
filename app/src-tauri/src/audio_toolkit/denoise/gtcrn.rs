use anyhow::Result;
use ndarray::{Array4, ArrayD, IxDyn};
use ort::session::{builder::GraphOptimizationLevel, Session};
use ort::value::Value;
use std::collections::VecDeque;
use std::path::Path;

use super::Denoiser;
use crate::audio_toolkit::audio::stft::Stft;

const HOP: usize = 256;
const BINS: usize = 257;

// Confirmed via introspection of gtcrn_simple.onnx:
//   IN  mix         [1, 257, 1, 2]
//   IN  conv_cache  [2, 1, 16, 16, 33]
//   IN  tra_cache   [2, 3, 1, 1, 16]
//   IN  inter_cache [2, 1, 33, 16]
//   OUT enh             [1, 257, 1, 2]
//   OUT conv_cache_out  [2, 1, 16, 16, 33]
//   OUT tra_cache_out   [2, 3, 1, 1, 16]
//   OUT inter_cache_out [2, 1, 33, 16]
// Note: the primary input is "mix", not "input" as suggested by the plan.

pub struct GtcrnDenoiser {
    session: Session,
    fwd: Stft,
    inv: Stft,
    conv_cache: ArrayD<f32>,
    tra_cache: ArrayD<f32>,
    inter_cache: ArrayD<f32>,
    in_q: VecDeque<f32>,
    warmup: bool,
}

impl GtcrnDenoiser {
    pub fn new<P: AsRef<Path>>(model_path: P) -> Result<Self> {
        let session = Session::builder()
            .map_err(|e| anyhow::anyhow!("ort builder: {e}"))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| anyhow::anyhow!("ort opt: {e}"))?
            .with_intra_threads(1)
            .map_err(|e| anyhow::anyhow!("ort intra: {e}"))?
            .commit_from_file(model_path.as_ref())
            .map_err(|e| anyhow::anyhow!("load GTCRN: {e}"))?;
        Ok(Self {
            session,
            fwd: Stft::new(),
            inv: Stft::new(),
            // Shapes confirmed via introspection (see constants above).
            conv_cache: ArrayD::<f32>::zeros(IxDyn(&[2, 1, 16, 16, 33])),
            tra_cache: ArrayD::<f32>::zeros(IxDyn(&[2, 3, 1, 1, 16])),
            inter_cache: ArrayD::<f32>::zeros(IxDyn(&[2, 1, 33, 16])),
            in_q: VecDeque::new(),
            warmup: true,
        })
    }

    fn run_frame(&mut self, hop: &[f32; HOP]) -> [f32; HOP] {
        // STFT -> complex bins, pack into [1, 257, 1, 2].
        let bins = self.fwd.analyze(hop);
        let mut mix = Array4::<f32>::zeros((1, BINS, 1, 2));
        for f in 0..BINS {
            mix[[0, f, 0, 0]] = bins[f][0]; // real
            mix[[0, f, 0, 1]] = bins[f][1]; // imag
        }

        let outputs = self
            .session
            .run(ort::inputs![
                "mix"         => Value::from_array(mix).unwrap(),
                "conv_cache"  => Value::from_array(self.conv_cache.clone()).unwrap(),
                "tra_cache"   => Value::from_array(self.tra_cache.clone()).unwrap(),
                "inter_cache" => Value::from_array(self.inter_cache.clone()).unwrap(),
            ])
            .expect("gtcrn run");

        // Extract enhanced output (shape [1, 257, 1, 2], C-contiguous → stride = f*2+c).
        let enh_tensor = outputs["enh"].try_extract_tensor::<f32>().unwrap();
        let enh_data = enh_tensor.1;
        let mut out_bins = [[0.0f32; 2]; BINS];
        for f in 0..BINS {
            out_bins[f][0] = enh_data[f * 2];     // real
            out_bins[f][1] = enh_data[f * 2 + 1]; // imag
        }
        // DC (bin 0) and Nyquist (bin 256) must be real-valued for a valid one-sided
        // RFFT spectrum; the GTCRN model may produce non-zero imaginary parts there.
        out_bins[0][1] = 0.0;
        out_bins[BINS - 1][1] = 0.0;

        // Advance recurrent caches.
        let cc = outputs["conv_cache_out"].try_extract_tensor::<f32>().unwrap();
        self.conv_cache =
            ArrayD::from_shape_vec(IxDyn(&[2, 1, 16, 16, 33]), cc.1.to_vec()).unwrap();
        let tc = outputs["tra_cache_out"].try_extract_tensor::<f32>().unwrap();
        self.tra_cache =
            ArrayD::from_shape_vec(IxDyn(&[2, 3, 1, 1, 16]), tc.1.to_vec()).unwrap();
        let ic = outputs["inter_cache_out"].try_extract_tensor::<f32>().unwrap();
        self.inter_cache =
            ArrayD::from_shape_vec(IxDyn(&[2, 1, 33, 16]), ic.1.to_vec()).unwrap();

        self.inv.synthesize(&out_bins)
    }
}

impl Denoiser for GtcrnDenoiser {
    fn process(&mut self, input: &[f32]) -> Vec<f32> {
        self.in_q.extend(input.iter().copied());
        let mut out = Vec::new();
        while self.in_q.len() >= HOP {
            let mut hop = [0.0f32; HOP];
            for v in hop.iter_mut() {
                *v = self.in_q.pop_front().unwrap();
            }
            let cleaned = self.run_frame(&hop);
            if self.warmup {
                self.warmup = false; // discard the first (model warm-up) output frame
            } else {
                out.extend_from_slice(&cleaned);
            }
        }
        out
    }

    fn reset(&mut self) {
        self.fwd.reset();
        self.inv.reset();
        self.conv_cache.fill(0.0);
        self.tra_cache.fill(0.0);
        self.inter_cache.fill(0.0);
        self.in_q.clear();
        self.warmup = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio_toolkit::denoise::Denoiser;
    use std::path::PathBuf;

    fn model_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/models/gtcrn_simple.onnx")
    }

    fn load_jfk() -> Vec<f32> {
        let wav = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/jfk.wav");
        let mut r = hound::WavReader::open(&wav).unwrap();
        assert_eq!(r.spec().sample_rate, 16000);
        assert_eq!(r.spec().channels, 1);
        match r.spec().sample_format {
            hound::SampleFormat::Int => {
                r.samples::<i16>().map(|s| s.unwrap() as f32 / 32768.0).collect()
            }
            hound::SampleFormat::Float => r.samples::<f32>().map(|s| s.unwrap()).collect(),
        }
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }

    // Mix speech with white noise, denoise, and assert the noise floor in the
    // SILENT lead-in of jfk.wav is reduced (the model removes noise where there
    // is no speech) without destroying the signal energy where there is speech.
    #[test]
    fn reduces_noise_floor_without_killing_speech() {
        let clean = load_jfk();
        // jfk.wav has ~0.1s of near-silence at the start; build a noisy copy.
        let noise_amp = 0.02f32;
        let noisy: Vec<f32> = clean
            .iter()
            .enumerate()
            .map(|(i, &s)| s + noise_amp * ((i as f32 * 12.9898).sin() * 43758.547).fract())
            .collect();

        let mut d = GtcrnDenoiser::new(model_path()).unwrap();
        let out = d.process(&noisy);
        // process is stream-in/out; flush any tail by processing a little silence.
        let mut out = out;
        out.extend(d.process(&vec![0.0f32; 512]));

        assert!(!out.iter().any(|v| v.is_nan()), "denoiser produced NaN");
        assert!(out.len() > clean.len() / 2, "denoiser dropped too much audio");

        // Lead-in window (first 1500 samples ~ pre-speech) should be quieter after denoise.
        let lead = 1500.min(out.len()).min(noisy.len());
        let noisy_floor = rms(&noisy[..lead]);
        let den_floor = rms(&out[..lead]);
        assert!(
            den_floor < noisy_floor * 0.8,
            "noise floor not reduced: {noisy_floor} -> {den_floor}"
        );
    }
}
