//! Platform detection utilities for display server identification.
//!
//! On Linux, the display server can be X11, Wayland, or Wayland with XWayland fallback.
//! This module detects which display server is in use and provides compatibility information
//! for screen capture and input simulation.

use once_cell::sync::Lazy;
use serde::Serialize;

/// Detected display server type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum DisplayServer {
    /// Pure X11 session
    X11,
    /// Pure Wayland session (no XWayland)
    Wayland,
    /// Wayland session with XWayland compatibility layer available
    WaylandWithXWayland,
    /// macOS (Quartz)
    MacOS,
    /// Windows (Win32/DWM)
    Windows,
    /// Could not determine the display server
    Unknown,
}

impl std::fmt::Display for DisplayServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DisplayServer::X11 => write!(f, "X11"),
            DisplayServer::Wayland => write!(f, "Wayland"),
            DisplayServer::WaylandWithXWayland => write!(f, "Wayland (with XWayland)"),
            DisplayServer::MacOS => write!(f, "macOS"),
            DisplayServer::Windows => write!(f, "Windows"),
            DisplayServer::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Compatibility status for a given display server configuration.
#[derive(Debug, Clone, Serialize)]
pub struct DisplayCompatibility {
    /// The detected display server
    pub display_server: DisplayServer,
    /// Whether screen capture is expected to work
    pub screen_capture_supported: bool,
    /// Whether input simulation is expected to work
    pub input_supported: bool,
    /// Human-readable warnings (empty if everything is fine)
    pub warnings: Vec<String>,
}

/// Cached display server detection result (detected once, reused everywhere).
static CACHED_DISPLAY_SERVER: Lazy<DisplayServer> = Lazy::new(detect_display_server);

/// Get the cached display server type. This is detected once at first access
/// and cached for the lifetime of the process.
pub fn get_display_server() -> DisplayServer {
    *CACHED_DISPLAY_SERVER
}

/// Detect the current display server.
///
/// On macOS, always returns `MacOS`.
/// On Windows, always returns `Windows`.
/// On Linux, checks environment variables to determine X11/Wayland/XWayland.
pub fn detect_display_server() -> DisplayServer {
    detect_display_server_from_env(
        std::env::var("XDG_SESSION_TYPE").ok(),
        std::env::var("WAYLAND_DISPLAY").ok(),
        std::env::var("DISPLAY").ok(),
    )
}

/// Internal detection logic, separated for testability.
/// Takes the relevant environment variable values as parameters.
fn detect_display_server_from_env(
    xdg_session_type: Option<String>,
    wayland_display: Option<String>,
    x11_display: Option<String>,
) -> DisplayServer {
    #[cfg(target_os = "macos")]
    {
        let _ = (xdg_session_type, wayland_display, x11_display);
        return DisplayServer::MacOS;
    }

    #[cfg(target_os = "windows")]
    {
        let _ = (xdg_session_type, wayland_display, x11_display);
        return DisplayServer::Windows;
    }

    #[cfg(target_os = "linux")]
    {
        let session_type = xdg_session_type.unwrap_or_default().to_lowercase();
        let has_wayland =
            session_type == "wayland" || wayland_display.as_ref().map_or(false, |v| !v.is_empty());
        let has_x11 =
            session_type == "x11" || x11_display.as_ref().map_or(false, |v| !v.is_empty());

        match (has_wayland, has_x11) {
            (true, true) => DisplayServer::WaylandWithXWayland,
            (true, false) => DisplayServer::Wayland,
            (false, true) => DisplayServer::X11,
            (false, false) => DisplayServer::Unknown,
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let _ = (xdg_session_type, wayland_display, x11_display);
        DisplayServer::Unknown
    }
}

/// Check display server compatibility and return warnings.
///
/// This function determines whether screen capture and input simulation
/// are expected to work for the current display server configuration.
pub fn check_display_compatibility() -> DisplayCompatibility {
    let display_server = get_display_server();

    match display_server {
        DisplayServer::X11 => DisplayCompatibility {
            display_server,
            screen_capture_supported: true,
            input_supported: true,
            warnings: vec![],
        },
        DisplayServer::WaylandWithXWayland => DisplayCompatibility {
            display_server,
            screen_capture_supported: true,
            input_supported: true,
            warnings: vec![
                "Running on Wayland with XWayland. Screen capture uses xcap which may require \
                 xdg-desktop-portal for full Wayland support. Input simulation uses XWayland \
                 compatibility layer."
                    .to_string(),
            ],
        },
        DisplayServer::Wayland => DisplayCompatibility {
            display_server,
            screen_capture_supported: true,
            input_supported: false,
            warnings: vec![
                "Running on pure Wayland without XWayland. Input simulation (mouse/keyboard) \
                 requires XWayland. Please ensure XWayland is installed and the DISPLAY \
                 environment variable is set. Most Wayland compositors include XWayland by default."
                    .to_string(),
                "Screen capture may require xdg-desktop-portal and PipeWire to be running."
                    .to_string(),
            ],
        },
        DisplayServer::MacOS | DisplayServer::Windows => DisplayCompatibility {
            display_server,
            screen_capture_supported: true,
            input_supported: true,
            warnings: vec![],
        },
        DisplayServer::Unknown => DisplayCompatibility {
            display_server,
            screen_capture_supported: false,
            input_supported: false,
            warnings: vec![
                "Could not detect display server. Screen capture and input simulation may not work."
                    .to_string(),
            ],
        },
    }
}

/// Build a user-friendly error message for screen capture failures on Wayland.
pub fn wayland_capture_error_hint(original_error: &str) -> String {
    let display = get_display_server();
    match display {
        DisplayServer::Wayland | DisplayServer::WaylandWithXWayland => {
            format!(
                "Screen capture failed on {}: {}. \
                 \n\nWayland troubleshooting:\n\
                 1. Ensure xdg-desktop-portal is installed and running\n\
                 2. Ensure PipeWire is installed and running (pipewire, wireplumber)\n\
                 3. Grant screen sharing permission when prompted by your desktop environment\n\
                 4. Try: systemctl --user restart xdg-desktop-portal",
                display, original_error
            )
        }
        _ => original_error.to_string(),
    }
}

/// Build a user-friendly error message for input simulation failures on Wayland.
pub fn wayland_input_error_hint(original_error: &str) -> String {
    let display = get_display_server();
    match display {
        DisplayServer::Wayland => {
            format!(
                "Input simulation failed on pure Wayland: {}. \
                 \n\nInput simulation requires XWayland. Please:\n\
                 1. Install XWayland (xorg-xwayland or xwayland package)\n\
                 2. Ensure your Wayland compositor has XWayland enabled\n\
                 3. Check that the DISPLAY environment variable is set (e.g., DISPLAY=:0)",
                original_error
            )
        }
        DisplayServer::WaylandWithXWayland => {
            format!(
                "Input simulation failed on Wayland with XWayland: {}. \
                 \n\nXWayland is available but input failed. Please:\n\
                 1. Ensure the target application window is running under XWayland\n\
                 2. Some native Wayland apps may not receive XWayland input events",
                original_error
            )
        }
        _ => original_error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_x11_session() {
        let result =
            detect_display_server_from_env(Some("x11".to_string()), None, Some(":0".to_string()));
        // On macOS/Windows this will return the platform-specific value
        #[cfg(target_os = "linux")]
        assert_eq!(result, DisplayServer::X11);
        #[cfg(target_os = "macos")]
        assert_eq!(result, DisplayServer::MacOS);
        #[cfg(target_os = "windows")]
        assert_eq!(result, DisplayServer::Windows);
    }

    #[test]
    fn test_detect_wayland_session() {
        let result = detect_display_server_from_env(
            Some("wayland".to_string()),
            Some("wayland-0".to_string()),
            None,
        );
        #[cfg(target_os = "linux")]
        assert_eq!(result, DisplayServer::Wayland);
        #[cfg(target_os = "macos")]
        assert_eq!(result, DisplayServer::MacOS);
    }

    #[test]
    fn test_detect_wayland_with_xwayland() {
        let result = detect_display_server_from_env(
            Some("wayland".to_string()),
            Some("wayland-0".to_string()),
            Some(":0".to_string()),
        );
        #[cfg(target_os = "linux")]
        assert_eq!(result, DisplayServer::WaylandWithXWayland);
        #[cfg(target_os = "macos")]
        assert_eq!(result, DisplayServer::MacOS);
    }

    #[test]
    fn test_detect_unknown_session() {
        let result = detect_display_server_from_env(None, None, None);
        #[cfg(target_os = "linux")]
        assert_eq!(result, DisplayServer::Unknown);
        #[cfg(target_os = "macos")]
        assert_eq!(result, DisplayServer::MacOS);
    }

    #[test]
    fn test_detect_wayland_via_env_only() {
        // WAYLAND_DISPLAY set but XDG_SESSION_TYPE not set to wayland
        let result = detect_display_server_from_env(
            Some("".to_string()),
            Some("wayland-0".to_string()),
            None,
        );
        #[cfg(target_os = "linux")]
        assert_eq!(result, DisplayServer::Wayland);
    }

    #[test]
    fn test_detect_x11_via_display_only() {
        // DISPLAY set but no XDG_SESSION_TYPE
        let result = detect_display_server_from_env(None, None, Some(":0".to_string()));
        #[cfg(target_os = "linux")]
        assert_eq!(result, DisplayServer::X11);
    }

    #[test]
    fn test_display_server_display_trait() {
        assert_eq!(format!("{}", DisplayServer::X11), "X11");
        assert_eq!(format!("{}", DisplayServer::Wayland), "Wayland");
        assert_eq!(
            format!("{}", DisplayServer::WaylandWithXWayland),
            "Wayland (with XWayland)"
        );
        assert_eq!(format!("{}", DisplayServer::MacOS), "macOS");
        assert_eq!(format!("{}", DisplayServer::Windows), "Windows");
        assert_eq!(format!("{}", DisplayServer::Unknown), "Unknown");
    }

    #[test]
    fn test_compatibility_x11() {
        // We can't easily test this on non-Linux without mocking,
        // but we can test the function signature and result structure
        let compat = check_display_compatibility();
        // On any platform, the fields should be populated
        assert!(format!("{}", compat.display_server).len() > 0);
    }

    #[test]
    fn test_wayland_capture_error_hint_on_non_wayland() {
        // On macOS/Windows, the hint should just return the original error
        #[cfg(not(target_os = "linux"))]
        {
            let hint = wayland_capture_error_hint("test error");
            assert_eq!(hint, "test error");
        }
    }

    #[test]
    fn test_wayland_input_error_hint_on_non_wayland() {
        #[cfg(not(target_os = "linux"))]
        {
            let hint = wayland_input_error_hint("test error");
            assert_eq!(hint, "test error");
        }
    }

    #[test]
    fn test_cached_display_server_consistent() {
        // Multiple calls should return the same value
        let first = get_display_server();
        let second = get_display_server();
        assert_eq!(first, second);
    }

    #[test]
    fn test_empty_display_env_var() {
        // Empty string for DISPLAY should not count as having X11
        let result = detect_display_server_from_env(None, None, Some("".to_string()));
        #[cfg(target_os = "linux")]
        assert_eq!(result, DisplayServer::Unknown);
    }

    #[test]
    fn test_empty_wayland_display_env_var() {
        // Empty string for WAYLAND_DISPLAY should not count as having Wayland
        let result = detect_display_server_from_env(None, Some("".to_string()), None);
        #[cfg(target_os = "linux")]
        assert_eq!(result, DisplayServer::Unknown);
    }
}
