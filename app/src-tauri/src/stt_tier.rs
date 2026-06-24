use crate::managers::model::ModelTier;

/// Maps a ModelTier to its canonical lowercase string for the frontend.
pub fn tier_to_str(t: ModelTier) -> &'static str {
    match t {
        ModelTier::Turbo => "turbo",
        ModelTier::Balanced => "balanced",
        ModelTier::Max => "max",
    }
}

#[allow(dead_code)]
pub struct CpuProfile {
    pub physical_cores: usize,
    pub total_ram_gb: f32,
}

/// Returns the recommended model tier based on CPU profile.
/// Pure function — no I/O, deterministic.
///
/// - Turbo (tiny Moonshine): weak machines with < 8 GB RAM or < 4 physical cores
/// - Max (Parakeet v3):      strong machines with >= 8 cores AND >= 24 GB RAM
/// - Balanced (Parakeet v2): everything in between
#[allow(dead_code)]
pub fn recommend_tier(p: &CpuProfile) -> ModelTier {
    if p.total_ram_gb < 8.0 || p.physical_cores < 4 {
        ModelTier::Turbo
    } else if p.physical_cores >= 8 && p.total_ram_gb >= 24.0 {
        ModelTier::Max
    } else {
        ModelTier::Balanced
    }
}

/// Reads the real CPU core count and total RAM from the OS via sysinfo.
/// This is a thin detector — all logic lives in `recommend_tier`.
pub fn detect_cpu_profile() -> CpuProfile {
    use sysinfo::{CpuRefreshKind, RefreshKind, System};
    let mut sys = System::new_with_specifics(
        RefreshKind::nothing()
            .with_memory(sysinfo::MemoryRefreshKind::everything())
            .with_cpu(CpuRefreshKind::nothing()),
    );
    sys.refresh_cpu_list(CpuRefreshKind::nothing());
    let total_ram_bytes = sys.total_memory();
    let total_ram_gb = total_ram_bytes as f32 / (1024.0 * 1024.0 * 1024.0);
    // physical_core_count() returns None if detection fails; fall back to 1.
    let physical_cores = sys.physical_core_count().unwrap_or(1);
    CpuProfile {
        physical_cores,
        total_ram_gb,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::managers::model::ModelTier;

    #[test]
    fn tier_to_str_turbo() {
        assert_eq!(tier_to_str(ModelTier::Turbo), "turbo");
    }

    #[test]
    fn tier_to_str_balanced() {
        assert_eq!(tier_to_str(ModelTier::Balanced), "balanced");
    }

    #[test]
    fn tier_to_str_max() {
        assert_eq!(tier_to_str(ModelTier::Max), "max");
    }

    #[test]
    fn weak_machine_gets_turbo() {
        assert_eq!(
            recommend_tier(&CpuProfile {
                physical_cores: 2,
                total_ram_gb: 4.0
            }),
            ModelTier::Turbo
        );
    }

    #[test]
    fn typical_laptop_gets_balanced() {
        assert_eq!(
            recommend_tier(&CpuProfile {
                physical_cores: 4,
                total_ram_gb: 16.0
            }),
            ModelTier::Balanced
        );
    }

    #[test]
    fn strong_desktop_gets_max() {
        assert_eq!(
            recommend_tier(&CpuProfile {
                physical_cores: 12,
                total_ram_gb: 32.0
            }),
            ModelTier::Max
        );
    }
}
