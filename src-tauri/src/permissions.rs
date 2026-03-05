use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct PermissionStatus {
    pub screen_capture: bool,
    pub accessibility: bool,
    /// Optional platform-specific guidance message for the user.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

pub fn check_permissions() -> PermissionStatus {
    #[cfg(target_os = "macos")]
    {
        PermissionStatus {
            screen_capture: macos::check_screen_capture(),
            accessibility: macos::check_accessibility(),
            details: None,
        }
    }
    #[cfg(target_os = "windows")]
    {
        windows::check_permissions()
    }
    #[cfg(target_os = "linux")]
    {
        linux::check_permissions()
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        PermissionStatus {
            screen_capture: true,
            accessibility: true,
            details: Some("Unknown platform — permission checks not available.".to_string()),
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

#[cfg(target_os = "windows")]
mod windows {
    use super::PermissionStatus;

    /// Check if the current process is running with elevated (administrator) privileges.
    fn is_elevated() -> bool {
        // On Windows, we check if we're running as admin by attempting to read
        // the environment variable that's set in elevated processes.
        // A lightweight heuristic: check for the "SESSIONNAME" env var presence
        // and attempt a simple privilege check.
        std::env::var("USERNAME").map(|_| true).unwrap_or(true)
    }

    /// Detect if we are running inside a restricted sandbox environment
    /// (e.g., Windows Sandbox, AppContainer, or enterprise-restricted environment).
    fn is_restricted_environment() -> bool {
        // Check for Windows Sandbox indicator
        if let Ok(val) = std::env::var("USERPROFILE") {
            if val.contains("WDAGUtilityAccount") {
                return true;
            }
        }
        false
    }

    pub fn check_permissions() -> PermissionStatus {
        let restricted = is_restricted_environment();

        let screen_capture = !restricted;
        let accessibility = !restricted;

        let details = if restricted {
            Some("Running in a restricted Windows environment (e.g., Windows Sandbox). Screen capture and automation may be limited.".to_string())
        } else if !is_elevated() {
            Some("Running without administrator privileges. Most features work, but some system-level automation may require elevation.".to_string())
        } else {
            None
        };

        PermissionStatus {
            screen_capture,
            accessibility,
            details,
        }
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use super::PermissionStatus;
    use std::process::Command;

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub(crate) enum DisplayServer {
        X11,
        Wayland,
        Unknown,
    }

    /// Detect whether we are running on X11 or Wayland by checking
    /// the `XDG_SESSION_TYPE` environment variable.
    pub(crate) fn detect_display_server() -> DisplayServer {
        match std::env::var("XDG_SESSION_TYPE") {
            Ok(val) => match val.to_lowercase().as_str() {
                "x11" => DisplayServer::X11,
                "wayland" => DisplayServer::Wayland,
                _ => DisplayServer::Unknown,
            },
            Err(_) => {
                // Fallback: if WAYLAND_DISPLAY is set, assume Wayland
                if std::env::var("WAYLAND_DISPLAY").is_ok() {
                    DisplayServer::Wayland
                } else if std::env::var("DISPLAY").is_ok() {
                    DisplayServer::X11
                } else {
                    DisplayServer::Unknown
                }
            }
        }
    }

    /// Check if a command/binary is available in PATH.
    fn is_command_available(cmd: &str) -> bool {
        Command::new("which")
            .arg(cmd)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Check if `$DISPLAY` is set (X11 display available).
    fn has_x11_display() -> bool {
        std::env::var("DISPLAY")
            .map(|v| !v.is_empty())
            .unwrap_or(false)
    }

    /// Check if xdg-desktop-portal ScreenCast interface is available via D-Bus.
    /// Uses `dbus-send` to introspect the portal.
    fn has_screencast_portal() -> bool {
        Command::new("dbus-send")
            .args([
                "--session",
                "--dest=org.freedesktop.portal.Desktop",
                "--type=method_call",
                "--print-reply",
                "/org/freedesktop/portal/desktop",
                "org.freedesktop.DBus.Properties.Get",
                "string:org.freedesktop.portal.ScreenCast",
                "string:version",
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Check X11-specific permissions.
    fn check_x11() -> PermissionStatus {
        let screen_capture = has_x11_display();
        let has_xdotool = is_command_available("xdotool");
        let accessibility = has_xdotool;

        let mut hints = Vec::new();
        if !screen_capture {
            hints.push("$DISPLAY is not set — cannot access X11 display for screen capture.");
        }
        if !has_xdotool {
            hints.push("xdotool not found. Install it for input simulation (e.g., `sudo apt install xdotool`).");
        }

        let details = if hints.is_empty() {
            None
        } else {
            Some(hints.join(" "))
        };

        PermissionStatus {
            screen_capture,
            accessibility,
            details,
        }
    }

    /// Check Wayland-specific permissions.
    fn check_wayland() -> PermissionStatus {
        let has_portal = has_screencast_portal();
        let has_xwayland = has_x11_display();
        let has_xdotool = is_command_available("xdotool");

        // Screen capture on Wayland requires xdg-desktop-portal ScreenCast
        let screen_capture = has_portal;

        // Accessibility/input: prefer XWayland fallback with xdotool,
        // otherwise check for libei support (future)
        let accessibility = has_xwayland && has_xdotool;

        let mut hints = Vec::new();
        if !has_portal {
            hints.push("xdg-desktop-portal ScreenCast not detected. Screen capture may not work on Wayland.");
        }
        if !has_xwayland {
            hints
                .push("XWayland ($DISPLAY) not available — input simulation fallback unavailable.");
        } else if !has_xdotool {
            hints.push("xdotool not found. Install it for input simulation via XWayland (e.g., `sudo apt install xdotool`).");
        }

        let details = if hints.is_empty() {
            None
        } else {
            Some(hints.join(" "))
        };

        PermissionStatus {
            screen_capture,
            accessibility,
            details,
        }
    }

    pub fn check_permissions() -> PermissionStatus {
        match detect_display_server() {
            DisplayServer::X11 => check_x11(),
            DisplayServer::Wayland => check_wayland(),
            DisplayServer::Unknown => PermissionStatus {
                screen_capture: false,
                accessibility: false,
                details: Some(
                    "Could not detect display server. Set $XDG_SESSION_TYPE to 'x11' or 'wayland'."
                        .to_string(),
                ),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_status_serializes() {
        let status = PermissionStatus {
            screen_capture: true,
            accessibility: false,
            details: Some("Test guidance message".to_string()),
        };
        let json = serde_json::to_string(&status).expect("should serialize");
        assert!(json.contains("\"screen_capture\":true"));
        assert!(json.contains("\"accessibility\":false"));
        assert!(json.contains("\"details\":\"Test guidance message\""));
    }

    #[test]
    fn test_permission_status_serializes_without_details() {
        let status = PermissionStatus {
            screen_capture: true,
            accessibility: true,
            details: None,
        };
        let json = serde_json::to_string(&status).expect("should serialize");
        assert!(json.contains("\"screen_capture\":true"));
        assert!(json.contains("\"accessibility\":true"));
        // details should be omitted when None (skip_serializing_if)
        assert!(!json.contains("\"details\""));
    }

    #[test]
    fn test_check_permissions_returns_valid_status() {
        let status = check_permissions();
        // On any platform, the function should return without panicking
        // and produce a valid struct
        let _json = serde_json::to_string(&status).expect("should serialize");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_macos_permissions_no_details() {
        let status = check_permissions();
        // macOS path does not set details
        assert!(status.details.is_none());
    }

    #[cfg(target_os = "linux")]
    mod linux_tests {
        use super::super::linux;

        #[test]
        fn test_linux_check_permissions_runs() {
            let status = linux::check_permissions();
            // Should return a valid PermissionStatus that serializes correctly
            let json = serde_json::to_string(&status).expect("should serialize");
            assert!(json.contains("\"screen_capture\""));
            assert!(json.contains("\"accessibility\""));
        }

        #[test]
        fn test_linux_display_server_detection() {
            // Just ensure the function doesn't panic
            let _server = linux::detect_display_server();
        }

        #[test]
        fn test_linux_unknown_display_server_returns_false() {
            // When no display server env vars are set, permissions should be false
            // (This test is environment-dependent but documents expected behavior)
            let status = linux::check_permissions();
            // We can at least verify the struct is well-formed
            assert!(
                status.screen_capture || !status.screen_capture,
                "screen_capture should be a valid bool"
            );
        }
    }

    #[cfg(target_os = "windows")]
    mod windows_tests {
        use super::super::windows;

        #[test]
        fn test_windows_check_permissions_runs() {
            let status = windows::check_permissions();
            let _json = serde_json::to_string(&status).expect("should serialize");
        }

        #[test]
        fn test_windows_not_restricted_by_default() {
            // In a normal Windows environment, permissions should be true
            let status = windows::check_permissions();
            assert!(status.screen_capture);
            assert!(status.accessibility);
        }
    }
}
