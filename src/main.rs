use std::ffi::OsStr;
use std::io::Write;
use std::io::{self};
use std::os::windows::ffi::OsStrExt;
use windows::core::PCWSTR;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;

fn main() -> windows::core::Result<()> {
    println!("Listing monitors and their current scaling resolutions:\n");

    let mut monitor_data: Vec<MonitorInfo> = Vec::new();

    unsafe {
        EnumDisplayMonitors(
            None,
            None,
            Some(monitor_enum_proc),
            LPARAM(&mut monitor_data as *mut _ as isize),
        ).ok()?;
    }

    if monitor_data.is_empty() {
        println!("No monitors found.");
        return Ok(());
    }

    // Display the monitors
    for (i, monitor) in monitor_data.iter().enumerate() {
        println!("{}: {}", i + 1, monitor.description);
    }

    let monitor_index = prompt_for_choice(monitor_data.len())? - 1;
    let selected_monitor = &monitor_data[monitor_index];
    println!("\nYou selected: {}\n", selected_monitor.description);

    let resolutions = get_supported_resolutions(&selected_monitor.device_name)?;
    for (i, resolution) in resolutions.iter().enumerate() {
        println!("{}: {}x{}", i + 1, resolution.width, resolution.height);
    }

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
    _: HDC,
    _: *mut RECT,
    lparam: LPARAM,
) -> BOOL {
    let monitor_data = &mut *(lparam.0 as *mut Vec<MonitorInfo>);
    let mut monitor_info = MONITORINFOEXW::default();
    monitor_info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;

    if GetMonitorInfoW(monitor, &mut monitor_info as *mut _ as *mut MONITORINFO).as_bool() {
        let device_name = widestring_to_string(&monitor_info.szDevice);
        let rect = monitor_info.monitorInfo.rcMonitor;
        let description = format!(
            "{}: {}x{}",
            device_name,
            rect.right - rect.left,
            rect.bottom - rect.top
        );
        monitor_data.push(MonitorInfo {
            description,
            device_name,
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
        let dev_name = string_to_pcwstr(device_name);

        while EnumDisplaySettingsW(dev_name, ENUM_DISPLAY_SETTINGS_MODE(mode_num), &mut dm)
            .as_bool()
        {
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

        let dev_name = string_to_pcwstr(device_name);

        let result = ChangeDisplaySettingsExW(
            dev_name,
            Some(&dm),
            HWND(std::ptr::null_mut()),
            CDS_UPDATEREGISTRY,
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
        println!("Invalid choice. Try again.");
    }
}

fn widestring_to_string(wstr: &[u16]) -> String {
    String::from_utf16_lossy(&wstr[..wstr.iter().position(|&c| c == 0).unwrap_or(wstr.len())])
}

fn string_to_pcwstr(s: &str) -> PCWSTR {
    let utf16: Vec<u16> = OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    PCWSTR(utf16.as_ptr())
}
