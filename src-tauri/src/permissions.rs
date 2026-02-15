use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct PermissionStatus {
    pub screen_capture: bool,
    pub accessibility: bool,
}

pub fn check_permissions() -> PermissionStatus {
    PermissionStatus {
        screen_capture: check_screen_capture(),
        accessibility: check_accessibility(),
    }
}

#[cfg(target_os = "macos")]
fn check_screen_capture() -> bool {
    extern "C" {
        fn CGPreflightScreenCaptureAccess() -> bool;
    }
    unsafe { CGPreflightScreenCaptureAccess() }
}

#[cfg(not(target_os = "macos"))]
fn check_screen_capture() -> bool {
    true // Non-macOS platforms don't need this permission
}

#[cfg(target_os = "macos")]
fn check_accessibility() -> bool {
    extern "C" {
        fn AXIsProcessTrusted() -> bool;
    }
    unsafe { AXIsProcessTrusted() }
}

#[cfg(not(target_os = "macos"))]
fn check_accessibility() -> bool {
    true // Non-macOS platforms don't need this permission
}
