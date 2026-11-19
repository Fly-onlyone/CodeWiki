//! Which model fits a machine. A fixed rule, so that the same machine always
//! gets the same answer and the answer can be explained in one line.

use serde::Serialize;

use crate::catalog::{self, Tier};
use crate::hardware::Hardware;

const GB: u64 = 1024 * 1024 * 1024;

/// Video memory from which the large model is recommended on the card.
const LARGE_VRAM: u64 = 6 * GB;
/// Memory and cores from which the balanced model is recommended. A machine
/// sold with 8 GB reports a little less, so the line is drawn below it.
const BALANCED_RAM: u64 = 7 * GB;
const BALANCED_CORES: usize = 4;
/// Free memory below which no model is recommended.
const LEAST_FREE_RAM: u64 = 2 * GB;
/// Disk space that must stay free after the download.
const DISK_MARGIN: u64 = GB;

/// What runs the model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Device {
    Cpu,
    Cuda,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Recommendation {
    /// Catalog id; `None` when the machine should keep the embedder that
    /// needs no model.
    pub model: Option<String>,
    pub device: Device,
    /// One sentence a person reads beside the recommendation.
    pub reason: String,
}

/// Gigabytes with one decimal, as Windows shows them.
fn gb(bytes: u64) -> String {
    format!("{:.1} GB", bytes as f64 / GB as f64)
}

/// The model for this machine. `cuda` says whether this installation can
/// run a model on an NVIDIA card at all (its CUDA worker is present).
pub fn recommend(hardware: &Hardware, cuda: bool) -> Recommendation {
    let none = |reason: String| Recommendation {
        model: None,
        device: Device::Cpu,
        reason,
    };
    if hardware.ram_available < LEAST_FREE_RAM {
        return none(format!(
            "Only {} of memory is free; a model needs {} or more.",
            gb(hardware.ram_available),
            gb(LEAST_FREE_RAM)
        ));
    }
    let card = hardware
        .nvidia
        .as_ref()
        .filter(|card| cuda && card.vram_total >= LARGE_VRAM);
    let (tier, device, reason) = if let Some(card) = card {
        (
            Tier::Large,
            Device::Cuda,
            format!(
                "{} with {} of video memory.",
                card.name,
                gb(card.vram_total)
            ),
        )
    } else {
        let machine = format!(
            "{} of memory and {} processor cores.",
            gb(hardware.ram_total),
            hardware.physical_cores
        );
        if hardware.ram_total >= BALANCED_RAM && hardware.physical_cores >= BALANCED_CORES {
            (Tier::Balanced, Device::Cpu, machine)
        } else {
            (Tier::Small, Device::Cpu, machine)
        }
    };
    let Some(entry) = catalog::of_tier(tier) else {
        return none("The catalog has no model for this machine.".to_string());
    };
    if let Some(free) = hardware.disk_free
        && free < entry.size() + DISK_MARGIN
    {
        return none(format!(
            "{} is free on the data drive; {} needs {}.",
            gb(free),
            entry.name,
            gb(entry.size() + DISK_MARGIN)
        ));
    }
    Recommendation {
        model: Some(entry.id.clone()),
        device,
        reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hardware::Nvidia;

    fn machine(ram_gb: f64, cores: usize) -> Hardware {
        let ram = (ram_gb * GB as f64) as u64;
        Hardware {
            ram_total: ram,
            ram_available: ram / 2,
            physical_cores: cores,
            nvidia: None,
            disk_free: Some(100 * GB),
        }
    }

    fn with_card(mut hardware: Hardware, vram_gb: u64) -> Hardware {
        hardware.nvidia = Some(Nvidia {
            name: "NVIDIA GeForce RTX 4070".into(),
            vram_total: vram_gb * GB,
            vram_free: vram_gb * GB,
        });
        hardware
    }

    fn pick(hardware: &Hardware, cuda: bool) -> (Option<String>, Device) {
        let recommendation = recommend(hardware, cuda);
        (recommendation.model, recommendation.device)
    }

    fn id(text: &str) -> Option<String> {
        Some(text.to_string())
    }

    #[test]
    fn the_rule_by_machine() {
        let small = id("granite-small-english-r2");
        let balanced = id("gte-modernbert-base");
        let large = id("qwen3-embedding-0.6b");
        let cases = [
            // The development machine.
            (
                with_card(machine(15.8, 12), 12),
                true,
                large.clone(),
                Device::Cuda,
            ),
            // The same machine in an installation without the CUDA worker.
            (
                with_card(machine(15.8, 12), 12),
                false,
                balanced.clone(),
                Device::Cpu,
            ),
            // A card too small for the large model.
            (
                with_card(machine(15.8, 12), 4),
                true,
                balanced.clone(),
                Device::Cpu,
            ),
            (with_card(machine(15.8, 12), 6), true, large, Device::Cuda),
            // "8 GB" as Windows reports it.
            (machine(7.9, 4), true, balanced.clone(), Device::Cpu),
            (machine(15.8, 12), true, balanced, Device::Cpu),
            // Enough memory, too few cores; enough cores, too little memory.
            (machine(15.8, 2), true, small.clone(), Device::Cpu),
            (machine(5.9, 8), true, small, Device::Cpu),
        ];
        for (hardware, cuda, model, device) in cases {
            assert_eq!(pick(&hardware, cuda), (model, device), "{hardware:?}");
        }
    }

    #[test]
    fn the_reason_names_what_decided() {
        let card = recommend(&with_card(machine(15.8, 12), 12), true);
        assert_eq!(
            card.reason,
            "NVIDIA GeForce RTX 4070 with 12.0 GB of video memory."
        );
        let processor = recommend(&machine(15.8, 12), true);
        assert_eq!(
            processor.reason,
            "15.8 GB of memory and 12 processor cores."
        );
    }

    #[test]
    fn too_little_free_memory_recommends_no_model() {
        // The clean test guest: 1.8 GB in all, 0.2 GB free.
        let mut guest = machine(1.8, 5);
        guest.ram_available = GB / 5;
        let recommendation = recommend(&guest, true);
        assert_eq!(recommendation.model, None);
        assert_eq!(
            recommendation.reason,
            "Only 0.2 GB of memory is free; a model needs 2.0 GB or more."
        );
    }

    #[test]
    fn too_little_disk_space_recommends_no_model() {
        let mut full = machine(15.8, 12);
        full.disk_free = Some(GB);
        let recommendation = recommend(&full, true);
        assert_eq!(recommendation.model, None);
        assert_eq!(
            recommendation.reason,
            "1.0 GB is free on the data drive; GTE ModernBERT Base needs 1.3 GB."
        );
        // A drive that could not be read does not stand in the way.
        full.disk_free = None;
        assert!(recommend(&full, true).model.is_some());
    }
}
