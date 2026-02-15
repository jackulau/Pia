use serde::Serialize;

#[derive(Serialize)]
pub struct PermissionStatus {
    pub screen_capture: bool,
    pub accessibility: bool,
}

pub fn check_permissions() -> PermissionStatus {
    #[cfg(target_os = "macos")]
    {
        PermissionStatus {
            screen_capture: macos::check_screen_capture(),
            accessibility: macos::check_accessibility(),
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        PermissionStatus {
            screen_capture: true,
            accessibility: true,
        }
    }
}

#[cfg(target_os = "macos")]
mod macos {
    extern "C" {
        fn CGPreflightScreenCaptureAccess() -> bool;
    }

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrusted() -> bool;
    }

    pub fn check_screen_capture() -> bool {
        unsafe { CGPreflightScreenCaptureAccess() }
    }

    pub fn check_accessibility() -> bool {
        unsafe { AXIsProcessTrusted() }
    }
}
