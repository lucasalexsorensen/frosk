#[cfg(target_family="windows")]
pub mod windows {
    use std::sync::{Arc, Mutex};
    use thiserror::Error;
    use windows::Win32::{
        Foundation::*,
        UI::WindowsAndMessaging::{EnumWindows, GetWindowTextW, GetWindowThreadProcessId},
    };
    
    #[derive(Debug, Clone)]
    pub struct WindowInfo {
        pub window: HWND,
        pub title: String,
        pub process_id: u32,
    }
    
    #[derive(Debug, Error)]
    pub enum GetWindowError {
        #[error("EnumWindows failed")]
        EnumWindowsFailed,
        #[error("Game not found")]
        GameNotFound,
    }
    
    pub fn get_window_info(title: &str) -> Result<WindowInfo, GetWindowError> {
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
            .filter(|i| i.title == title)
            .next()
            .ok_or(GetWindowError::GameNotFound)?;
    
        Ok(window.clone())
    }
    
    pub extern "system" fn enum_callback(window: HWND, lparam: LPARAM) -> BOOL {
        unsafe {
            // get window title
            let mut raw_title: [u16; 512] = [0; 512];
            let len = GetWindowTextW(window, &mut raw_title);
            let title = String::from_utf16_lossy(&raw_title[..len as usize]);
    
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
                title,
                process_id,
            };
            vec.push(info);
    
            true.into()
        }
    }
}
