use std::sync::{Arc, Mutex};
use thiserror::Error;

use windows::Win32::{
    Foundation::*,
    UI::WindowsAndMessaging::{EnumWindows, GetWindowTextW, GetWindowThreadProcessId},
};

#[derive(Debug, Clone)]
pub struct WindowInfo {
    window: HWND,
    text: String,
    process_id: u32,
}

#[derive(Debug, Error)]
pub enum GetWindowError {
    #[error("EnumWindows failed")]
    EnumWindowsFailed,
    #[error("Game not found")]
    GameNotFound,
}

fn get_window_info() -> Result<WindowInfo, GetWindowError> {
    let window_infos = Arc::new(Mutex::new(Vec::<WindowInfo>::new()));

    let window_infos_clone = Arc::clone(&window_infos);
    unsafe {
        EnumWindows(
            Some(enum_callback),
            LPARAM(&*window_infos_clone as *const _ as isize),
        )
        .map_err(|_| GetWindowError::EnumWindowsFailed)?;
    }

    let infos = window_infos.lock().unwrap();
    let window = infos
        .iter()
        .filter(|i| i.text == "World of Warcraft")
        .next()
        .ok_or(GetWindowError::GameNotFound)?;

    Ok(window.clone())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let window = get_window_info()?;

    println!("Window: {:?}", window);

    Ok(())
}

extern "system" fn enum_callback(window: HWND, lparam: LPARAM) -> BOOL {
    unsafe {
        // get window text
        let mut text: [u16; 512] = [0; 512];
        let len = GetWindowTextW(window, &mut text);
        let text = String::from_utf16_lossy(&text[..len as usize]);

        // get process ID
        let mut process_id: u32 = 0;
        let result = GetWindowThreadProcessId(window, Some(&mut process_id));

        if result == 0 {
            return true.into();
        }

        // push back to the vector via the mutex
        let window_infos = &*(lparam.0 as *const Mutex<Vec<WindowInfo>>);
        let mut vec = window_infos.lock().unwrap();
        let info = WindowInfo {
            window,
            text,
            process_id,
        };
        vec.push(info);

        true.into()
    }
}
