use std::io::Write;
use std::io::{self};
use std::mem::size_of;
use std::mem::zeroed;
use windows::Win32::Devices::Display::DisplayConfigGetDeviceInfo;
use windows::Win32::Devices::Display::DisplayConfigSetDeviceInfo;
use windows::Win32::Devices::Display::GetDisplayConfigBufferSizes;
use windows::Win32::Devices::Display::QueryDisplayConfig;
use windows::Win32::Devices::Display::DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME;
use windows::Win32::Devices::Display::DISPLAYCONFIG_DEVICE_INFO_HEADER;
use windows::Win32::Devices::Display::DISPLAYCONFIG_DEVICE_INFO_TYPE;
use windows::Win32::Devices::Display::DISPLAYCONFIG_MODE_INFO;
use windows::Win32::Devices::Display::DISPLAYCONFIG_MODE_INFO_TYPE_SOURCE;
use windows::Win32::Devices::Display::DISPLAYCONFIG_PATH_INFO;
use windows::Win32::Devices::Display::DISPLAYCONFIG_SOURCE_MODE;
use windows::Win32::Devices::Display::DISPLAYCONFIG_TARGET_DEVICE_NAME;
use windows::Win32::Devices::Display::QDC_ONLY_ACTIVE_PATHS;
use windows::Win32::Devices::Display::QUERY_DISPLAY_CONFIG_FLAGS;
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::Foundation::LUID;
use windows::Win32::Graphics::Gdi::DISPLAYCONFIG_PATH_MODE_IDX_INVALID;

#[repr(i32)]
#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
enum DISPLAYCONFIG_DEVICE_INFO_TYPE_CUSTOM {
    DISPLAYCONFIG_DEVICE_INFO_GET_DPI_SCALE = -3, // get DPI info
    DISPLAYCONFIG_DEVICE_INFO_SET_DPI_SCALE = -4, // set DPI
}

#[repr(C)]
#[allow(non_camel_case_types, non_snake_case)]
struct DISPLAYCONFIG_SOURCE_DPI_SCALE_GET {
    header: DISPLAYCONFIG_DEVICE_INFO_HEADER,
    minScaleRel: i32,
    curScaleRel: i32,
    maxScaleRel: i32,
}

#[repr(C)]
#[allow(non_camel_case_types, non_snake_case)]
struct DISPLAYCONFIG_SOURCE_DPI_SCALE_SET {
    header: DISPLAYCONFIG_DEVICE_INFO_HEADER,
    scaleRel: i32,
}

static DPI_VALS: [u32; 12] = [100, 125, 150, 175, 200, 225, 250, 300, 350, 400, 450, 500];

struct DPIScalingInfo {
    minimum: u32,
    maximum: u32,
    current: u32,
    recommended: u32,
    valid: bool,
}

#[allow(dead_code)]
struct DisplayInfo {
    adapter_id: LUID,
    source_id: u32,
    target_id: u32,
    name: String,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

fn get_paths_and_modes(
    flags: QUERY_DISPLAY_CONFIG_FLAGS,
) -> Option<(Vec<DISPLAYCONFIG_PATH_INFO>, Vec<DISPLAYCONFIG_MODE_INFO>)> {
    let mut num_paths: u32 = 0;
    let mut num_modes: u32 = 0;

    let status = unsafe { GetDisplayConfigBufferSizes(flags, &mut num_paths, &mut num_modes) };
    if status != ERROR_SUCCESS {
        return None;
    }

    let mut paths = Vec::with_capacity(num_paths as usize);
    let mut modes = Vec::with_capacity(num_modes as usize);

    let status = unsafe {
        QueryDisplayConfig(
            flags,
            &mut num_paths,
            paths.as_mut_ptr(),
            &mut num_modes,
            modes.as_mut_ptr(),
            None,
        )
    };
    if status != ERROR_SUCCESS {
        return None;
    }

    unsafe {
        paths.set_len(num_paths as usize);
        modes.set_len(num_modes as usize);
    }

    Some((paths, modes))
}
fn get_dpi_scaling_info(adapter_id: LUID, source_id: u32) -> DPIScalingInfo {
    let mut request_packet: DISPLAYCONFIG_SOURCE_DPI_SCALE_GET = unsafe { zeroed() };
    request_packet.header.size = size_of::<DISPLAYCONFIG_SOURCE_DPI_SCALE_GET>() as u32;
    request_packet.header.adapterId = adapter_id;
    request_packet.header.id = source_id;
    request_packet.header.r#type = DISPLAYCONFIG_DEVICE_INFO_TYPE(
        DISPLAYCONFIG_DEVICE_INFO_TYPE_CUSTOM::DISPLAYCONFIG_DEVICE_INFO_GET_DPI_SCALE as i32,
    );

    let res = unsafe { DisplayConfigGetDeviceInfo(&mut request_packet.header) };
    if res != ERROR_SUCCESS.0 as i32 {
        return DPIScalingInfo {
            minimum: 100,
            maximum: 100,
            current: 100,
            recommended: 100,
            valid: false,
        };
    }

    let mut cur_scale = request_packet.curScaleRel;
    if cur_scale < request_packet.minScaleRel {
        cur_scale = request_packet.minScaleRel;
    } else if cur_scale > request_packet.maxScaleRel {
        cur_scale = request_packet.maxScaleRel;
    }

    let min_abs = request_packet.minScaleRel.abs() as isize;
    let cur_index = min_abs + (cur_scale as isize);
    let rec_index = min_abs;
    let max_index = min_abs + (request_packet.maxScaleRel as isize);

    // Validate indices
    let total_count = DPI_VALS.len() as isize;
    if rec_index < 0 || rec_index >= total_count {
        return DPIScalingInfo { minimum:100, maximum:100, current:100, recommended:100, valid:false };
    }
    if cur_index < 0 || cur_index >= total_count {
        return DPIScalingInfo { minimum:100, maximum:100, current:100, recommended:100, valid:false };
    }
    if max_index < 0 || max_index >= total_count {
        return DPIScalingInfo { minimum:100, maximum:100, current:100, recommended:100, valid:false };
    }

    let recommended = DPI_VALS[rec_index as usize];
    let current = DPI_VALS[cur_index as usize];
    let maximum = DPI_VALS[max_index as usize];

    DPIScalingInfo {
        minimum: 100,
        maximum,
        current,
        recommended,
        valid: true,
    }
}


fn set_dpi_scaling(adapter_id: LUID, source_id: u32, dpi_percent_to_set: u32) -> bool {
    let dpi_info = get_dpi_scaling_info(adapter_id, source_id);
    if !dpi_info.valid {
        eprintln!("Unable to get DPI info for this display.");
        return false;
    }

    let mut dpi = dpi_percent_to_set;
    if dpi == dpi_info.current {
        return true;
    }
    if dpi < dpi_info.minimum {
        dpi = dpi_info.minimum;
    } else if dpi > dpi_info.maximum {
        dpi = dpi_info.maximum;
    }

    let mut idx_recommended = -1;
    let mut idx_to_set = -1;

    for (i, val) in DPI_VALS.iter().enumerate() {
        if *val == dpi {
            idx_to_set = i as i32;
        }
        if *val == dpi_info.recommended {
            idx_recommended = i as i32;
        }
    }

    if idx_recommended == -1 || idx_to_set == -1 {
        eprintln!("Error: cannot find DPI value indexes.");
        return false;
    }

    let dpi_relative_val = idx_to_set - idx_recommended;

    let mut set_packet: DISPLAYCONFIG_SOURCE_DPI_SCALE_SET = unsafe { zeroed() };
    set_packet.header.adapterId = adapter_id;
    set_packet.header.id = source_id;
    set_packet.header.size = size_of::<DISPLAYCONFIG_SOURCE_DPI_SCALE_SET>() as u32;
    set_packet.header.r#type = DISPLAYCONFIG_DEVICE_INFO_TYPE(
        DISPLAYCONFIG_DEVICE_INFO_TYPE_CUSTOM::DISPLAYCONFIG_DEVICE_INFO_SET_DPI_SCALE as i32,
    );
    set_packet.scaleRel = dpi_relative_val;

    let res = unsafe { DisplayConfigSetDeviceInfo(&set_packet.header) };
    res == ERROR_SUCCESS.0 as i32
}

fn enumerate_displays() -> Vec<DisplayInfo> {
    let (paths, modes) = match get_paths_and_modes(QDC_ONLY_ACTIVE_PATHS) {
        Some(p) => p,
        None => {
            eprintln!("Cannot get display paths.");
            return vec![];
        }
    };

    let mut displays = Vec::new();

    for path in paths.iter() {
        let mut device_name: DISPLAYCONFIG_TARGET_DEVICE_NAME = unsafe { zeroed() };
        device_name.header.size = size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32;
        device_name.header.adapterId = path.targetInfo.adapterId;
        device_name.header.id = path.targetInfo.id;
        device_name.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME;

        let res = unsafe { DisplayConfigGetDeviceInfo(&mut device_name.header) };
        if res != ERROR_SUCCESS.0 as i32 {
            continue;
        }

        let mut name = String::from_utf16_lossy(&device_name.monitorFriendlyDeviceName);
        name = name.trim_end_matches('\u{0}').to_string();

        // Extract resolution and position:
        let (x, y, width, height) = {
            let mut px = 0;
            let mut py = 0;
            let mut w = 0;
            let mut h = 0;
            let mode_info_idx = unsafe { path.sourceInfo.Anonymous.modeInfoIdx };
            if mode_info_idx != DISPLAYCONFIG_PATH_MODE_IDX_INVALID {
                let mode_index = mode_info_idx as usize;
                if mode_index < modes.len() {
                    let mode = &modes[mode_index];
                    if mode.infoType == DISPLAYCONFIG_MODE_INFO_TYPE_SOURCE {
                        let source_mode: &DISPLAYCONFIG_SOURCE_MODE =
                            unsafe { &mode.Anonymous.sourceMode };
                        px = source_mode.position.x;
                        py = source_mode.position.y;
                        w = source_mode.width;
                        h = source_mode.height;
                    }
                }
            }

            (px, py, w, h)
        };

        displays.push(DisplayInfo {
            adapter_id: path.targetInfo.adapterId,
            source_id: path.sourceInfo.id,
            target_id: path.targetInfo.id,
            name,
            x,
            y,
            width,
            height,
        });
    }

    // Sort by x position (left to right)
    displays.sort_by_key(|d| d.x);

    displays
}

fn main() {
    let displays = enumerate_displays();
    if displays.is_empty() {
        eprintln!("No active displays found.");
        return;
    }

    println!("Enumerated displays (left to right):");
    for (i, disp) in displays.iter().enumerate() {
        println!(
            "{}: {} ({}x{}, pos: {}x{})",
            i + 1,
            disp.name,
            disp.width,
            disp.height,
            disp.x,
            disp.y
        );
    }

    print!("Please select a monitor by entering its number: ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read input.");

    let trimmed = input.trim();
    let display_index= match trimmed.parse::<usize>() {
        Ok(num) if num > 0 && num <= displays.len() => num - 1,
        _ => {
            eprintln!("Invalid selection. Exiting...");
            return;
        }
    };

    let disp = &displays[display_index];
    println!(
        "Selected display: {} (SourceID: {})",
        disp.name, disp.source_id
    );

    let dpi_info = get_dpi_scaling_info(disp.adapter_id, disp.source_id);
    if !dpi_info.valid {
        println!("Unable to fetch DPI info for the selected display");
        return;
    }

    println!("Current DPI: {}%", dpi_info.current);
    println!("Recommended DPI: {}%", dpi_info.recommended);
    println!("Maximum DPI: {}%", dpi_info.maximum);
    println!("Possible DPIs: {:?}", DPI_VALS);

    print!("Please select a DPI percentage from the list above: ");
    io::stdout().flush().unwrap();

    let mut dpi_input = String::new();
    io::stdin()
        .read_line(&mut dpi_input)
        .expect("Failed to read input");
    let dpi_trimmed = dpi_input.trim();
    let target_dpi: u32 = match dpi_trimmed.parse() {
        Ok(val) if DPI_VALS.contains(&val) => val,
        _ => {
            eprintln!("Invalid DPI value.");
            return;
        }
    };

    println!("Setting DPI to {}%", target_dpi);
    let success = set_dpi_scaling(disp.adapter_id, disp.source_id, target_dpi);
    if success {
        println!("DPI updated successfully!");
    } else {
        println!("Failed to update DPI");
    }
}
