//! Streaming sqrt-Hann STFT / iSTFT with 50%-overlap WOLA reconstruction.
//! n_fft = 512, hop = 256, 257 one-sided bins. Used by the GTCRN denoiser.

use realfft::{ComplexToReal, RealFftPlanner, RealToComplex};
use realfft::num_complex::Complex;
use std::sync::Arc;

const N_FFT: usize = 512;
const HOP: usize = 256;
const BINS: usize = N_FFT / 2 + 1; // 257

fn sqrt_hann() -> [f32; N_FFT] {
    let mut w = [0.0f32; N_FFT];
    for (n, wn) in w.iter_mut().enumerate() {
        // periodic Hann, then sqrt (sqrt-Hann gives unity-gain WOLA at 50% overlap).
        let hann = 0.5 - 0.5 * (2.0 * std::f32::consts::PI * n as f32 / N_FFT as f32).cos();
        *wn = hann.max(0.0).sqrt();
    }
    w
}

pub struct Stft {
    r2c: Arc<dyn RealToComplex<f32>>,
    c2r: Arc<dyn ComplexToReal<f32>>,
    window: [f32; N_FFT],
    in_buf: [f32; N_FFT],          // sliding analysis buffer (last 512 samples)
    ola: [f32; N_FFT + HOP],       // synthesis OLA accumulator (768 samples for 512-sample latency)
    scratch_c: Vec<Complex<f32>>,
    scratch_r: Vec<f32>,
}

impl Stft {
    pub fn new() -> Self {
        let mut planner = RealFftPlanner::<f32>::new();
        let r2c = planner.plan_fft_forward(N_FFT);
        let c2r = planner.plan_fft_inverse(N_FFT);
        Self {
            r2c,
            c2r,
            window: sqrt_hann(),
            in_buf: [0.0; N_FFT],
            ola: [0.0; N_FFT + HOP],
            scratch_c: vec![Complex::new(0.0, 0.0); BINS],
            scratch_r: vec![0.0; N_FFT],
        }
    }

    pub fn reset(&mut self) {
        self.in_buf = [0.0; N_FFT];
        self.ola = [0.0; N_FFT + HOP];
    }

    pub fn analyze(&mut self, hop: &[f32; HOP]) -> [[f32; 2]; BINS] {
        // Slide in the new hop: drop oldest 256, append newest 256.
        self.in_buf.copy_within(HOP.., 0);
        self.in_buf[HOP..].copy_from_slice(hop);

        let mut windowed: Vec<f32> = (0..N_FFT).map(|i| self.in_buf[i] * self.window[i]).collect();
        self.r2c.process(&mut windowed, &mut self.scratch_c).unwrap();

        let mut out = [[0.0f32; 2]; BINS];
        for (i, c) in self.scratch_c.iter().enumerate() {
            out[i] = [c.re, c.im];
        }
        out
    }

    pub fn synthesize(&mut self, bins: &[[f32; 2]; BINS]) -> [f32; HOP] {
        for (i, b) in bins.iter().enumerate() {
            self.scratch_c[i] = Complex::new(b[0], b[1]);
        }
        self.c2r.process(&mut self.scratch_c, &mut self.scratch_r).unwrap();

        // realfft inverse is unnormalized: divide by N_FFT. Apply synthesis window, overlap-add.
        // Add the N_FFT-sample frame at offset HOP in the buffer so the output (ola[0..HOP])
        // is delayed by one extra hop, giving the expected 512-sample (2-hop) round-trip latency.
        let norm = 1.0 / N_FFT as f32;
        for i in 0..N_FFT {
            self.ola[HOP + i] += self.scratch_r[i] * norm * self.window[i];
        }
        let mut out = [0.0f32; HOP];
        out.copy_from_slice(&self.ola[..HOP]);
        // Shift the accumulator left by HOP, zero the tail.
        self.ola.copy_within(HOP.., 0);
        for v in self.ola[N_FFT..].iter_mut() {
            *v = 0.0;
        }
        out
    }
}

impl Default for Stft {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Feed a signal through analyze->synthesize unchanged; after the one-frame
    // (512-sample) warm-up latency, output must reconstruct the input.
    #[test]
    fn round_trip_reconstructs_after_latency() {
        let n = 256 * 40;
        let input: Vec<f32> = (0..n).map(|i| 0.3 * (i as f32 * 0.05).sin()).collect();

        let mut fwd = Stft::new();
        let mut inv = Stft::new();
        let mut out: Vec<f32> = Vec::new();
        for chunk in input.chunks_exact(256) {
            let mut hop = [0.0f32; 256];
            hop.copy_from_slice(chunk);
            let bins = fwd.analyze(&hop);
            let rec = inv.synthesize(&bins);
            out.extend_from_slice(&rec);
        }

        // One frame (512 samples = 2 hops) of latency; compare the steady-state middle.
        let lat = 512;
        let mut max_err = 0.0f32;
        for i in (lat..n - 256).step_by(1) {
            max_err = max_err.max((out[i] - input[i - lat]).abs());
        }
        assert!(max_err < 1e-3, "reconstruction error too high: {max_err}");
    }

    #[test]
    fn dc_bin_is_real_for_constant_input() {
        let mut fwd = Stft::new();
        let hop = [1.0f32; 256];
        let _ = fwd.analyze(&hop); // prime
        let bins = fwd.analyze(&hop);
        // Nyquist (bin 256) imag must be ~0; DC (bin 0) imag must be ~0.
        assert!(bins[0][1].abs() < 1e-4);
        assert!(bins[256][1].abs() < 1e-4);
    }
}
