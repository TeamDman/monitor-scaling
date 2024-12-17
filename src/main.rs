use std::io::Write;
use std::io::{self};
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::WindowsAndMessaging::*;

fn main() -> windows::core::Result<()> {
    println!("Listing monitors and their current scaling resolutions:\n");
    let mut monitor_data = Vec::new();

    unsafe {
        // Enumerate monitors and store information
        let enum_proc = Some(monitor_enum_proc);
        EnumDisplayMonitors(
            HWND::default(),
            None,
            enum_proc,
            &mut monitor_data as *mut _ as isize,
        );
    }

    if monitor_data.is_empty() {
        println!("No monitors found.");
        return Ok(());
    }

    // Display the monitors
    for (i, monitor) in monitor_data.iter().enumerate() {
        println!("{}: {}", i + 1, monitor.description);
    }

    // Ask the user to pick a monitor
    let monitor_index = prompt_for_choice(monitor_data.len())? - 1;
    let selected_monitor = &monitor_data[monitor_index];
    println!("\nYou selected: {}\n", selected_monitor.description);

    // List supported resolutions for the chosen monitor
    let resolutions = get_supported_resolutions(&selected_monitor.device_name)?;
    println!(
        "Supported resolutions for {}:\n",
        selected_monitor.description
    );
    for (i, resolution) in resolutions.iter().enumerate() {
        println!("{}: {}x{}", i + 1, resolution.width, resolution.height);
    }

    // Ask the user to pick a resolution
    let res_index = prompt_for_choice(resolutions.len())? - 1;
    let selected_res = &resolutions[res_index];

    println!(
        "\nSetting resolution to {}x{}...\n",
        selected_res.width, selected_res.height
    );
    set_monitor_resolution(
        &selected_monitor.device_name,
        selected_res.width,
        selected_res.height,
    )?;

    println!("Resolution changed successfully!");
    Ok(())
}

struct MonitorInfo {
    description: String,
    device_name: String,
}

struct Resolution {
    width: u32,
    height: u32,
}

unsafe extern "system" fn monitor_enum_proc(
    monitor: HMONITOR,
    _hdc: HDC,
    _rect: *mut RECT,
    lparam: LPARAM,
) -> BOOL {
    let monitor_data = &mut *(lparam.0 as *mut Vec<MonitorInfo>);
    let mut monitor_info = MONITORINFOEXW::default();
    monitor_info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;

    if GetMonitorInfoW(monitor, &mut monitor_info as *mut _ as *mut MONITORINFO).as_bool() {
        let device_name = String::from_utf16_lossy(&monitor_info.szDevice);
        let rect = monitor_info.monitorInfo.rcMonitor;
        let description = format!(
            "{}: {}x{} ({}x{})",
            device_name.trim_end_matches('\0'),
            rect.right - rect.left,
            rect.bottom - rect.top,
            rect.right - rect.left, // Actual scaled resolution logic can be added here
            rect.bottom - rect.top
        );
        monitor_data.push(MonitorInfo {
            description,
            device_name: device_name.trim_end_matches('\0').to_string(),
        });
    }
    true.into()
}

fn get_supported_resolutions(device_name: &str) -> windows::core::Result<Vec<Resolution>> {
    let mut resolutions = Vec::new();
    unsafe {
        let mut dm = DEVMODEW::default();
        dm.dmSize = std::mem::size_of::<DEVMODEW>() as u16;
        let mut mode_num = 0;

        while EnumDisplaySettingsW(device_name, mode_num, &mut dm).as_bool() {
            resolutions.push(Resolution {
                width: dm.dmPelsWidth,
                height: dm.dmPelsHeight,
            });
            mode_num += 1;
        }
    }
    Ok(resolutions)
}

fn set_monitor_resolution(device_name: &str, width: u32, height: u32) -> windows::core::Result<()> {
    unsafe {
        let mut dm = DEVMODEW::default();
        dm.dmSize = std::mem::size_of::<DEVMODEW>() as u16;
        dm.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT;
        dm.dmPelsWidth = width;
        dm.dmPelsHeight = height;

        let result = ChangeDisplaySettingsExW(
            device_name,
            &mut dm,
            HWND::default(),
            CDS_UPDATEREGISTRY | CDS_NORESET,
            None,
        );

        if result == DISP_CHANGE_SUCCESSFUL {
            Ok(())
        } else {
            Err(windows::core::Error::from_win32())
        }
    }
}

fn prompt_for_choice(max: usize) -> io::Result<usize> {
    let mut input = String::new();
    loop {
        print!("Enter a number (1-{}): ", max);
        io::stdout().flush()?;
        input.clear();
        io::stdin().read_line(&mut input)?;
        if let Ok(choice) = input.trim().parse::<usize>() {
            if choice > 0 && choice <= max {
                return Ok(choice);
            }
        }
        println!("Invalid choice. Please try again.");
    }
}
