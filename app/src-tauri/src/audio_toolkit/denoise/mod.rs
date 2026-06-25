pub mod gtcrn;
pub use gtcrn::GtcrnDenoiser;

/// A streaming single-channel denoiser. `process` is stream-in/out: feed any
/// length of 16 kHz mono samples, receive the cleaned samples ready so far.
pub trait Denoiser: Send {
    fn process(&mut self, input: &[f32]) -> Vec<f32>;
    fn reset(&mut self);
}
