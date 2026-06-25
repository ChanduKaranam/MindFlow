use anyhow::Result;
use ndarray::{Array0, Array2, Array3};
use ort::session::{builder::GraphOptimizationLevel, Session};
use ort::value::Value;
use std::path::Path;

use super::{VadFrame, VoiceActivityDetector};
use crate::audio_toolkit::audio::gain;

/// Silero v6 requires exactly 512 samples (32 ms) per inference at 16 kHz.
const V6_FRAME_SAMPLES: usize = 512;
/// Sample rate expected by the model.
const V6_SAMPLE_RATE: i64 = 16000;

// Detection-only gain defaults (see audio_toolkit::audio::gain).
const GAIN_TARGET_DBFS: f32 = -20.0;
const GAIN_NOISE_GATE_DBFS: f32 = -50.0;
const GAIN_MAX: f32 = 8.0;

pub struct SileroV6Vad {
    session: Session,
    state: Array3<f32>, // [2, 1, 128]
    threshold: f32,
}

impl SileroV6Vad {
    pub fn new<P: AsRef<Path>>(model_path: P, threshold: f32) -> Result<Self> {
        if !(0.0..=1.0).contains(&threshold) {
            anyhow::bail!("threshold must be between 0.0 and 1.0");
        }
        // ort::Error<SessionBuilder> is not Send+Sync, so we cannot use `?` directly
        // with anyhow::Result. Following vad-rs reference: map_err via ort::Error to
        // strip the generic parameter before the `?` conversion to anyhow::Error.
        let session = Session::builder()
            .map_err(|e| anyhow::anyhow!("ort session builder: {e}"))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| anyhow::anyhow!("ort opt level: {e}"))?
            .with_intra_threads(1)
            .map_err(|e| anyhow::anyhow!("ort intra threads: {e}"))?
            .with_inter_threads(1)
            .map_err(|e| anyhow::anyhow!("ort inter threads: {e}"))?
            .commit_from_file(model_path.as_ref())
            .map_err(|e| anyhow::anyhow!("Failed to load Silero v6 model: {e}"))?;

        Ok(Self {
            session,
            state: Array3::<f32>::zeros((2, 1, 128)),
            threshold,
        })
    }

    /// Run one 512-sample frame; returns speech probability and advances LSTM state.
    pub fn probability(&mut self, frame: &[f32]) -> Result<f32> {
        // Boost soft speech for detection only (the original `frame` is what the
        // caller keeps for transcription).
        let boosted = gain::rms_normalized(frame, GAIN_TARGET_DBFS, GAIN_NOISE_GATE_DBFS, GAIN_MAX);

        let input = Array2::from_shape_vec((1, boosted.len()), boosted)?;
        let input_value = Value::from_array(input)?;
        let state_value = Value::from_array(self.state.clone())?;
        // v6.0 model requires a scalar int64 `sr` input (sample rate).
        let sr_value = Value::from_array(Array0::from_elem((), V6_SAMPLE_RATE))?;

        let outputs = self
            .session
            .run(ort::inputs![
                "input" => input_value,
                "state" => state_value,
                "sr" => sr_value,
            ])
            .map_err(|e| anyhow::anyhow!("Silero v6 inference error: {e}"))?;

        // Advance LSTM state from `stateN`.
        let state_out = outputs
            .get("stateN")
            .ok_or_else(|| anyhow::anyhow!("model output 'stateN' missing"))?
            .try_extract_tensor::<f32>()
            .map_err(|e| anyhow::anyhow!("extract stateN: {e}"))?;
        self.state = Array3::from_shape_vec((2, 1, 128), state_out.1.to_vec())?;

        let prob_out = outputs
            .get("output")
            .ok_or_else(|| anyhow::anyhow!("model output 'output' missing"))?
            .try_extract_tensor::<f32>()
            .map_err(|e| anyhow::anyhow!("extract output: {e}"))?;
        Ok(prob_out.1[0])
    }
}

impl VoiceActivityDetector for SileroV6Vad {
    fn push_frame<'a>(&'a mut self, frame: &'a [f32]) -> Result<VadFrame<'a>> {
        if frame.len() != V6_FRAME_SAMPLES {
            anyhow::bail!("expected {V6_FRAME_SAMPLES} samples, got {}", frame.len());
        }
        let prob = self.probability(frame)?;
        if prob > self.threshold {
            Ok(VadFrame::Speech(frame))
        } else {
            Ok(VadFrame::Noise)
        }
    }

    fn reset(&mut self) {
        self.state.fill(0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn model_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/models/silero_vad_v6.onnx")
    }

    #[test]
    fn silence_is_not_speech() {
        let mut vad = SileroV6Vad::new(model_path(), 0.5).unwrap();
        let silence = vec![0.0f32; 512];
        let mut max_p = 0.0f32;
        for _ in 0..10 {
            max_p = max_p.max(vad.probability(&silence).unwrap());
        }
        assert!(max_p < 0.2, "silence probability too high: {max_p}");
    }

    #[test]
    fn jfk_fixture_contains_speech() {
        let wav = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/jfk.wav");
        let mut reader = hound::WavReader::open(&wav).unwrap();
        let spec = reader.spec();
        assert_eq!(spec.sample_rate, 16000, "fixture must be 16 kHz");
        assert_eq!(spec.channels, 1, "fixture must be mono");

        let samples: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Int => reader
                .samples::<i16>()
                .map(|s| s.unwrap() as f32 / 32768.0)
                .collect(),
            hound::SampleFormat::Float => {
                reader.samples::<f32>().map(|s| s.unwrap()).collect()
            }
        };

        let mut vad = SileroV6Vad::new(model_path(), 0.5).unwrap();
        let mut speech_frames = 0;
        for chunk in samples.chunks_exact(512) {
            if vad.probability(chunk).unwrap() > 0.5 {
                speech_frames += 1;
            }
        }
        assert!(speech_frames > 10, "expected speech to be detected, got {speech_frames} frames");
    }

    #[test]
    fn rejects_wrong_frame_size() {
        let mut vad = SileroV6Vad::new(model_path(), 0.5).unwrap();
        let wrong = vec![0.0f32; 480];
        assert!(vad.push_frame(&wrong).is_err());
    }
}
