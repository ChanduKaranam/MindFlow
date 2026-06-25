//! Detection-only gain normalization placed in front of the VAD.
//! Quiet speech is boosted so the VAD scores it as speech; the audio handed to
//! transcription is never modified by this stage.

/// Boost `frame` toward `target_dbfs` for VAD detection.
///
/// - Boost-only: gain is clamped to `[1.0, max_gain]`, loud frames pass through.
/// - Frames whose RMS is below `noise_gate_dbfs` are returned unchanged so the
///   silence noise floor is not amplified ("noise breathing").
/// - Output samples are clamped to `[-1.0, 1.0]`.
pub fn rms_normalized(frame: &[f32], target_dbfs: f32, noise_gate_dbfs: f32, max_gain: f32) -> Vec<f32> {
    if frame.is_empty() {
        return Vec::new();
    }
    let sum_sq: f32 = frame.iter().map(|s| s * s).sum();
    let rms = (sum_sq / frame.len() as f32).sqrt();
    if rms <= 1e-9 {
        return frame.to_vec();
    }
    let rms_dbfs = 20.0 * rms.log10();
    if rms_dbfs < noise_gate_dbfs {
        return frame.to_vec();
    }
    let target_rms = 10f32.powf(target_dbfs / 20.0);
    let gain = (target_rms / rms).clamp(1.0, max_gain);
    frame.iter().map(|s| (s * gain).clamp(-1.0, 1.0)).collect()
}

/// Normalize a whole captured clip toward `target_dbfs` with a single gain,
/// clamped to `[1.0, max_gain]` (boost-only). Output clamped to [-1, 1].
pub fn normalize_clip(samples: &[f32], target_dbfs: f32, max_gain: f32) -> Vec<f32> {
    if samples.is_empty() {
        return Vec::new();
    }
    let rms = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt();
    if rms <= 1e-9 {
        return samples.to_vec();
    }
    let target_rms = 10f32.powf(target_dbfs / 20.0);
    let gain = (target_rms / rms).clamp(1.0, max_gain);
    if gain == 1.0 {
        return samples.to_vec();
    }
    samples.iter().map(|s| (s * gain).clamp(-1.0, 1.0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rms(frame: &[f32]) -> f32 {
        (frame.iter().map(|s| s * s).sum::<f32>() / frame.len() as f32).sqrt()
    }

    #[test]
    fn silence_is_unchanged() {
        let silence = vec![0.0f32; 512];
        let out = rms_normalized(&silence, -20.0, -50.0, 8.0);
        assert!(out.iter().all(|&s| s == 0.0));
    }

    #[test]
    fn quiet_speech_is_boosted_toward_target() {
        // ~ -40 dBFS sine (rms ~0.01), above the -50 dBFS gate.
        let quiet: Vec<f32> = (0..512)
            .map(|i| 0.01 * (i as f32 * 0.2).sin())
            .collect();
        let before = rms(&quiet);
        let out = rms_normalized(&quiet, -20.0, -50.0, 8.0);
        let after = rms(&out);
        assert!(after > before * 2.0, "expected boost, before={before} after={after}");
    }

    #[test]
    fn noise_floor_below_gate_is_not_amplified() {
        // ~ -60 dBFS (rms ~0.001), below the -50 dBFS gate.
        let floor: Vec<f32> = (0..512)
            .map(|i| 0.001 * (i as f32 * 0.2).sin())
            .collect();
        let out = rms_normalized(&floor, -20.0, -50.0, 8.0);
        assert_eq!(out, floor, "frames below the gate must be returned unchanged");
    }

    #[test]
    fn loud_speech_is_not_attenuated() {
        // ~ -6 dBFS (rms ~0.5), already above the -20 dBFS target.
        let loud: Vec<f32> = (0..512)
            .map(|i| 0.5 * (i as f32 * 0.2).sin())
            .collect();
        let out = rms_normalized(&loud, -20.0, -50.0, 8.0);
        assert_eq!(out, loud, "boost-only: loud frames must be unchanged");
    }

    #[test]
    fn gain_is_clamped_and_output_stays_in_range() {
        // Just above the gate but very quiet → would want huge gain; must clamp to 8x.
        let v: Vec<f32> = (0..512)
            .map(|i| 0.004 * (i as f32 * 0.2).sin())
            .collect();
        let out = rms_normalized(&v, -20.0, -50.0, 8.0);
        assert!(out.iter().all(|&s| s >= -1.0 && s <= 1.0));
        // Gain capped at 8x, so the loudest sample is at most 8 * 0.004 = 0.032.
        let peak = out.iter().fold(0.0f32, |m, &s| m.max(s.abs()));
        assert!(peak <= 0.04, "gain not clamped: peak={peak}");
    }

    #[test]
    fn normalize_clip_boosts_quiet_clip_toward_target() {
        let quiet: Vec<f32> = (0..16000).map(|i| 0.02 * (i as f32 * 0.05).sin()).collect();
        let before = (quiet.iter().map(|s| s * s).sum::<f32>() / quiet.len() as f32).sqrt();
        let out = normalize_clip(&quiet, -20.0, 10.0);
        let after = (out.iter().map(|s| s * s).sum::<f32>() / out.len() as f32).sqrt();
        assert!(after > before * 2.0, "clip not boosted: {before} -> {after}");
        assert!(out.iter().all(|&s| (-1.0..=1.0).contains(&s)));
    }

    #[test]
    fn normalize_clip_leaves_loud_clip_unchanged() {
        let loud: Vec<f32> = (0..16000).map(|i| 0.5 * (i as f32 * 0.05).sin()).collect();
        let out = normalize_clip(&loud, -20.0, 10.0);
        assert_eq!(out, loud);
    }

    #[test]
    fn normalize_clip_handles_empty() {
        assert!(normalize_clip(&[], -20.0, 10.0).is_empty());
    }
}
