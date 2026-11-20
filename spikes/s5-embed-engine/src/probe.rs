//! The readings the real feature needs from the machine: memory, cores,
//! disk, the NVIDIA card, and the two halves of the "a game is running" rule.
//! Their presence in this program is also the test that a clean Windows can
//! start a program that links them.

use crate::Result;

const GB: f64 = 1024.0 * 1024.0 * 1024.0;

pub fn run() -> Result<()> {
    let mut system = sysinfo::System::new()?;
    system.refresh_memory();
    println!(
        "memory: {:.1} GB total, {:.1} GB available",
        system.total_memory() as f64 / GB,
        system.available_memory() as f64 / GB
    );
    match sysinfo::System::physical_core_count() {
        Ok(cores) => println!("physical cores: {cores}"),
        Err(error) => println!("physical cores: unknown ({error})"),
    }
    match sysinfo::Disks::new_with_refreshed_list() {
        Ok(disks) => {
            for disk in &disks {
                println!(
                    "disk {}: {:.1} GB free",
                    disk.mount_point().display(),
                    disk.available_space() as f64 / GB
                );
            }
        }
        Err(error) => println!("disks: unknown ({error})"),
    }
    match nvidia() {
        Ok(cards) if cards.is_empty() => println!("nvidia: no card"),
        Ok(cards) => {
            for card in cards {
                println!("nvidia: {card}");
            }
        }
        // No driver, no nvml.dll: the ordinary case on most machines.
        Err(error) => println!("nvidia: none ({error})"),
    }
    game::report();
    Ok(())
}

fn nvidia() -> std::result::Result<Vec<String>, nvml_wrapper::error::NvmlError> {
    let nvml = nvml_wrapper::Nvml::init()?;
    let mut cards = Vec::new();
    for index in 0..nvml.device_count()? {
        let device = nvml.device_by_index(index)?;
        let memory = device.memory_info()?;
        cards.push(format!(
            "{}, {:.1} GB video memory, {:.1} GB free",
            device.name()?,
            memory.total as f64 / GB,
            memory.free as f64 / GB
        ));
    }
    Ok(cards)
}

#[cfg(not(windows))]
mod game {
    pub fn report() {
        println!("game readings: Windows only");
    }
}

#[cfg(windows)]
mod game {
    use std::time::Duration;

    use windows_sys::Win32::System::Performance::{
        PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY, PDH_MORE_DATA,
        PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterArrayW,
        PdhOpenQueryW,
    };
    use windows_sys::Win32::UI::Shell::{
        QUNS_BUSY, QUNS_RUNNING_D3D_FULL_SCREEN, SHQueryUserNotificationState,
    };

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// What Windows says the user is doing; 3 is a Direct3D full-screen app,
    /// 2 a full-screen app.
    fn notification_state() -> Option<i32> {
        let mut state = 0;
        // SAFETY: the function writes one i32 through a pointer that is valid
        // for the call.
        let result = unsafe { SHQueryUserNotificationState(&mut state) };
        (result >= 0).then_some(state)
    }

    /// Percent of the graphics card's 3D engines in use, by process id, over
    /// one second.
    fn load_3d() -> Result<Vec<(u32, f64)>, String> {
        let mut query: PDH_HQUERY = std::ptr::null_mut();
        let mut counter: PDH_HCOUNTER = std::ptr::null_mut();
        let path = wide(r"\GPU Engine(*engtype_3D)\Utilization Percentage");
        // SAFETY: every pointer is to a local that outlives the call; the
        // query is closed on every path below.
        unsafe {
            let status = PdhOpenQueryW(std::ptr::null(), 0, &mut query);
            if status != 0 {
                return Err(format!("PdhOpenQueryW: {status:#x}"));
            }
            let status = PdhAddEnglishCounterW(query, path.as_ptr(), 0, &mut counter);
            if status != 0 {
                PdhCloseQuery(query);
                return Err(format!("PdhAddEnglishCounterW: {status:#x}"));
            }
            // A rate needs two samples.
            PdhCollectQueryData(query);
            std::thread::sleep(Duration::from_secs(1));
            let status = PdhCollectQueryData(query);
            if status != 0 {
                PdhCloseQuery(query);
                return Err(format!("PdhCollectQueryData: {status:#x}"));
            }
            let mut size = 0u32;
            let mut count = 0u32;
            let status = PdhGetFormattedCounterArrayW(
                counter,
                PDH_FMT_DOUBLE,
                &mut size,
                &mut count,
                std::ptr::null_mut(),
            );
            if status != PDH_MORE_DATA {
                PdhCloseQuery(query);
                return Err(format!("PdhGetFormattedCounterArrayW (size): {status:#x}"));
            }
            // The buffer holds the items and, after them, their names; u64
            // gives it the items' alignment.
            let mut buffer = vec![0u64; (size as usize).div_ceil(8)];
            let items = buffer.as_mut_ptr().cast::<PDH_FMT_COUNTERVALUE_ITEM_W>();
            let status =
                PdhGetFormattedCounterArrayW(counter, PDH_FMT_DOUBLE, &mut size, &mut count, items);
            if status != 0 {
                PdhCloseQuery(query);
                return Err(format!("PdhGetFormattedCounterArrayW: {status:#x}"));
            }
            let mut by_process: Vec<(u32, f64)> = Vec::new();
            for index in 0..count as usize {
                let item = &*items.add(index);
                let mut length = 0;
                while *item.szName.add(length) != 0 {
                    length += 1;
                }
                let name = String::from_utf16_lossy(std::slice::from_raw_parts(item.szName, length));
                // pid_1234_luid_0x..._phys_0_eng_0_engtype_3D
                let Some(pid) = name
                    .strip_prefix("pid_")
                    .and_then(|rest| rest.split('_').next())
                    .and_then(|pid| pid.parse::<u32>().ok())
                else {
                    continue;
                };
                let value = item.FmtValue.Anonymous.doubleValue;
                match by_process.iter_mut().find(|(known, _)| *known == pid) {
                    Some((_, total)) => *total += value,
                    None => by_process.push((pid, value)),
                }
            }
            PdhCloseQuery(query);
            Ok(by_process)
        }
    }

    pub fn report() {
        let state = notification_state();
        let full_screen = matches!(state, Some(QUNS_RUNNING_D3D_FULL_SCREEN | QUNS_BUSY));
        println!("notification state: {state:?} (full screen: {full_screen})");
        match load_3d() {
            Ok(mut loads) => {
                let own = std::process::id();
                let others: f64 = loads
                    .iter()
                    .filter(|(pid, _)| *pid != own)
                    .map(|(_, load)| load)
                    .sum();
                loads.sort_by(|a, b| b.1.total_cmp(&a.1));
                println!(
                    "3D load of other processes: {others:.1}% over {} processes; busiest: {:?}",
                    loads.len(),
                    &loads[..loads.len().min(3)]
                );
            }
            Err(error) => println!("3D load: unknown ({error})"),
        }
    }
}
