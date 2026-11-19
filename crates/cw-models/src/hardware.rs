//! What the machine has to run a model with.

use std::path::Path;

use serde::Serialize;

/// The NVIDIA card a model can run on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Nvidia {
    pub name: String,
    /// Bytes of video memory.
    pub vram_total: u64,
    pub vram_free: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Hardware {
    /// Bytes of memory.
    pub ram_total: u64,
    /// Bytes a new program could take right now.
    pub ram_available: u64,
    pub physical_cores: usize,
    /// The card with the most video memory; `None` without an NVIDIA driver.
    pub nvidia: Option<Nvidia>,
    /// Bytes free on the drive that holds the data directory; `None` when
    /// the drive could not be found.
    pub disk_free: Option<u64>,
}

/// Read the machine. Nothing here fails: a reading that cannot be taken is
/// reported as absent or zero, and the recommendation is cautious about it.
pub fn probe(data_dir: &Path) -> Hardware {
    let (ram_total, ram_available) = match sysinfo::System::new() {
        Ok(mut system) => {
            system.refresh_memory();
            (system.total_memory(), system.available_memory())
        }
        Err(error) => {
            log::warn!("could not read the machine's memory: {error}");
            (0, 0)
        }
    };
    Hardware {
        ram_total,
        ram_available,
        physical_cores: sysinfo::System::physical_core_count().unwrap_or(0),
        nvidia: nvidia(),
        disk_free: disk_free(data_dir),
    }
}

/// The NVIDIA card with the most video memory. The driver's library is
/// loaded at run time, so a machine without the driver simply has no card.
fn nvidia() -> Option<Nvidia> {
    let nvml = nvml_wrapper::Nvml::init().ok()?;
    let mut best: Option<Nvidia> = None;
    for index in 0..nvml.device_count().ok()? {
        let Ok(device) = nvml.device_by_index(index) else {
            continue;
        };
        let (Ok(name), Ok(memory)) = (device.name(), device.memory_info()) else {
            continue;
        };
        if best
            .as_ref()
            .is_none_or(|card| memory.total > card.vram_total)
        {
            best = Some(Nvidia {
                name,
                vram_total: memory.total,
                vram_free: memory.free,
            });
        }
    }
    best
}

/// The machine's NVIDIA card, read once for this process: asking the driver
/// loads its library, and the answer does not change while a program runs.
/// The free video memory in it is as old as the first call.
pub fn nvidia_card() -> Option<&'static Nvidia> {
    static CARD: std::sync::OnceLock<Option<Nvidia>> = std::sync::OnceLock::new();
    CARD.get_or_init(nvidia).as_ref()
}

/// Free bytes on the drive whose mount point is the longest start of `path`.
fn disk_free(path: &Path) -> Option<u64> {
    // The folder may not exist yet; its drive does.
    let path = std::path::absolute(path).ok()?;
    let disks = sysinfo::Disks::new_with_refreshed_list().ok()?;
    disks
        .iter()
        .filter(|disk| path.starts_with(disk.mount_point()))
        .max_by_key(|disk| disk.mount_point().as_os_str().len())
        .map(|disk| disk.available_space())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_machine_that_runs_the_tests_can_be_read() {
        let dir = tempfile::tempdir().unwrap();
        // Not yet there: the drive is still found.
        let hardware = probe(&dir.path().join("models"));
        assert!(hardware.ram_total > 0);
        assert!(hardware.ram_available <= hardware.ram_total);
        assert!(hardware.physical_cores > 0);
        assert!(hardware.disk_free.is_some_and(|free| free > 0));
        if let Some(card) = hardware.nvidia {
            assert!(!card.name.is_empty());
            assert!(card.vram_free <= card.vram_total);
        }
    }
}
