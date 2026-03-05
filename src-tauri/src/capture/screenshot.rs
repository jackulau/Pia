#![allow(dead_code)]

use base64::{engine::general_purpose::STANDARD, Engine as _};
use image::{imageops::FilterType, DynamicImage, ImageFormat};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::io::Cursor;
use std::sync::Arc;
use thiserror::Error;
use xcap::Monitor;

#[derive(Error, Debug)]
pub enum CaptureError {
    #[error("No monitors found")]
    NoMonitors,
    #[error("Failed to capture screen: {0}")]
    CaptureError(String),
    #[error("Failed to encode image: {0}")]
    EncodeError(#[from] image::ImageError),
    #[error("Wayland capture error: {0}")]
    WaylandError(String),
}

pub struct Screenshot {
    pub width: u32,
    pub height: u32,
    /// Physical screen width in pixels (before any downsampling for the LLM)
    pub physical_width: u32,
    /// Physical screen height in pixels (before any downsampling for the LLM)
    pub physical_height: u32,
    /// Base64-encoded screenshot data wrapped in Arc to avoid expensive clones.
    /// Screenshots are typically 1-2MB and are shared across conversation history,
    /// action history, and state without copying.
    pub base64: Arc<String>,
}

/// Compute a fast fingerprint hash of a screenshot's base64 data.
///
/// Instead of hashing the entire base64 string (which can be 1-2MB),
/// this samples the length plus the first 1KB, a middle 1KB, and the
/// last 1KB of the data. This is sufficient to detect screen changes
/// while completing in well under 1ms.
pub fn quick_screenshot_hash(base64: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    let bytes = base64.as_bytes();
    let len = bytes.len();

    // Always hash the length - different sizes = different screenshots
    hasher.write_usize(len);

    const SAMPLE_SIZE: usize = 1024;

    if len <= SAMPLE_SIZE * 3 {
        // Small enough to hash entirely
        hasher.write(bytes);
    } else {
        // Sample first, middle, and last 1KB
        hasher.write(&bytes[..SAMPLE_SIZE]);
        let mid = len / 2 - SAMPLE_SIZE / 2;
        hasher.write(&bytes[mid..mid + SAMPLE_SIZE]);
        hasher.write(&bytes[len - SAMPLE_SIZE..]);
    }

    hasher.finish()
}

/// Configuration for screenshot capture
#[derive(Debug, Clone)]
pub struct ScreenshotConfig {
    /// Image format to use (PNG or JPEG)
    pub format: ImageOutputFormat,
    /// JPEG quality (1-100), only used when format is JPEG
    pub jpeg_quality: u8,
    /// Maximum width for downsampling (None = no downsampling)
    pub max_width: Option<u32>,
    /// Filter type for resizing
    pub resize_filter: ResizeFilter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageOutputFormat {
    Png,
    Jpeg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeFilter {
    Nearest,
    Bilinear,
    CatmullRom,
    Lanczos3,
}

impl ResizeFilter {
    fn to_filter_type(self) -> FilterType {
        match self {
            ResizeFilter::Nearest => FilterType::Nearest,
            ResizeFilter::Bilinear => FilterType::Triangle,
            ResizeFilter::CatmullRom => FilterType::CatmullRom,
            ResizeFilter::Lanczos3 => FilterType::Lanczos3,
        }
    }
}

impl Default for ScreenshotConfig {
    fn default() -> Self {
        Self {
            format: ImageOutputFormat::Jpeg,
            jpeg_quality: 80,
            max_width: Some(1920),
            resize_filter: ResizeFilter::Bilinear,
        }
    }
}

/// Cached primary monitor reference
static CACHED_PRIMARY_MONITOR: Lazy<Mutex<Option<CachedMonitor>>> = Lazy::new(|| Mutex::new(None));

/// Pre-allocated buffer for image encoding (reused across captures)
static ENCODE_BUFFER: Lazy<Mutex<Vec<u8>>> =
    Lazy::new(|| Mutex::new(Vec::with_capacity(1024 * 1024)));

struct CachedMonitor {
    monitor: Monitor,
}

// We need this because Monitor doesn't implement Send/Sync by default
// but we know it's safe to share the monitor reference across threads
// since we're only using it for capture operations
unsafe impl Send for CachedMonitor {}
unsafe impl Sync for CachedMonitor {}

/// Get the primary monitor, using cached version if available
fn get_primary_monitor() -> Result<Monitor, CaptureError> {
    let mut cached = CACHED_PRIMARY_MONITOR.lock();

    // Try to use cached monitor first
    if let Some(ref cached_monitor) = *cached {
        return Ok(cached_monitor.monitor.clone());
    }

    // Enumerate monitors and cache the primary one
    let monitors = Monitor::all().map_err(|e| CaptureError::CaptureError(e.to_string()))?;

    let primary = monitors
        .into_iter()
        .find(|m| m.is_primary().unwrap_or(false))
        .or_else(|| Monitor::all().ok()?.into_iter().next())
        .ok_or(CaptureError::NoMonitors)?;

    let cloned = primary.clone();
    *cached = Some(CachedMonitor { monitor: primary });

    Ok(cloned)
}

/// Invalidate the cached monitor (call on error or config change)
pub fn invalidate_monitor_cache() {
    let mut cached = CACHED_PRIMARY_MONITOR.lock();
    *cached = None;
}

/// Process an image: optionally downsample and encode.
/// Returns (final_width, final_height, physical_width, physical_height, base64).
fn process_image(
    image: image::RgbaImage,
    config: &ScreenshotConfig,
) -> Result<(u32, u32, u32, u32, String), CaptureError> {
    let mut dynamic_image = DynamicImage::ImageRgba8(image);
    let original_width = dynamic_image.width();
    let original_height = dynamic_image.height();
    // Physical dimensions are the original capture size (before any downsampling)
    let physical_width = original_width;
    let physical_height = original_height;

    // Downsample if needed
    let (final_width, final_height) = if let Some(max_width) = config.max_width {
        if original_width > max_width {
            let scale = max_width as f32 / original_width as f32;
            let new_height = (original_height as f32 * scale) as u32;
            dynamic_image =
                dynamic_image.resize(max_width, new_height, config.resize_filter.to_filter_type());
            (max_width, new_height)
        } else {
            (original_width, original_height)
        }
    } else {
        (original_width, original_height)
    };

    // Get buffer from pool or use a new one
    let mut buffer = {
        let mut b = ENCODE_BUFFER.lock();
        b.clear();
        std::mem::take(&mut *b)
    };

    let mut cursor = Cursor::new(buffer);

    // Encode based on format
    match config.format {
        ImageOutputFormat::Png => {
            dynamic_image.write_to(&mut cursor, ImageFormat::Png)?;
        }
        ImageOutputFormat::Jpeg => {
            let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
                &mut cursor,
                config.jpeg_quality,
            );
            dynamic_image.write_with_encoder(encoder)?;
        }
    }

    buffer = cursor.into_inner();

    // Pre-allocate base64 string and encode in-place (avoids intermediate allocation)
    let encoded_len = buffer.len() * 4 / 3 + 4;
    let mut base64 = String::with_capacity(encoded_len);
    STANDARD.encode_string(&buffer, &mut base64);

    // Return buffer to pool
    {
        buffer.clear();
        let mut pooled = ENCODE_BUFFER.lock();
        *pooled = buffer;
    }

    Ok((
        final_width,
        final_height,
        physical_width,
        physical_height,
        base64,
    ))
}

/// Capture the primary screen with default configuration
pub fn capture_primary_screen() -> Result<Screenshot, CaptureError> {
    capture_primary_screen_with_config(&ScreenshotConfig::default())
}

/// Capture the primary screen with custom configuration.
///
/// On Wayland, xcap may use xdg-desktop-portal ScreenCast for capture.
/// If capture fails on Wayland, the error message includes troubleshooting hints.
pub fn capture_primary_screen_with_config(
    config: &ScreenshotConfig,
) -> Result<Screenshot, CaptureError> {
    let primary = match get_primary_monitor() {
        Ok(m) => m,
        Err(e) => {
            // Invalidate cache and retry once
            invalidate_monitor_cache();
            match get_primary_monitor() {
                Ok(m) => m,
                Err(_) => {
                    // Provide Wayland-specific error hint if applicable
                    let error_msg = crate::platform::wayland_capture_error_hint(&e.to_string());
                    return Err(CaptureError::CaptureError(error_msg));
                }
            }
        }
    };

    let image = match primary.capture_image() {
        Ok(img) => img,
        Err(_capture_err) => {
            // Invalidate cache on capture error and retry
            invalidate_monitor_cache();
            let primary = get_primary_monitor()?;
            match primary.capture_image() {
                Ok(img) => img,
                Err(e) => {
                    // Provide Wayland-specific error hint if applicable
                    let error_msg = crate::platform::wayland_capture_error_hint(&e.to_string());
                    return Err(CaptureError::CaptureError(error_msg));
                }
            }
        }
    };

    let (width, height, physical_width, physical_height, base64) = process_image(image, config)?;

    Ok(Screenshot {
        width,
        height,
        physical_width,
        physical_height,
        base64: Arc::new(base64),
    })
}

/// Capture all screens with default configuration
pub fn capture_all_screens() -> Result<Vec<Screenshot>, CaptureError> {
    capture_all_screens_with_config(&ScreenshotConfig::default())
}

/// Capture all screens with custom configuration
pub fn capture_all_screens_with_config(
    config: &ScreenshotConfig,
) -> Result<Vec<Screenshot>, CaptureError> {
    let monitors = Monitor::all().map_err(|e| CaptureError::CaptureError(e.to_string()))?;

    let mut screenshots = Vec::with_capacity(monitors.len());

    for monitor in monitors {
        let image = monitor
            .capture_image()
            .map_err(|e| CaptureError::CaptureError(e.to_string()))?;

        let (width, height, physical_width, physical_height, base64) =
            process_image(image, config)?;

        screenshots.push(Screenshot {
            width,
            height,
            physical_width,
            physical_height,
            base64: Arc::new(base64),
        });
    }

    Ok(screenshots)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quick_hash_identical_strings_produce_same_hash() {
        let data = "a".repeat(5000);
        let h1 = quick_screenshot_hash(&data);
        let h2 = quick_screenshot_hash(&data);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_quick_hash_different_strings_produce_different_hash() {
        let data_a = "a".repeat(5000);
        let data_b = "b".repeat(5000);
        let h1 = quick_screenshot_hash(&data_a);
        let h2 = quick_screenshot_hash(&data_b);
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_quick_hash_different_lengths_produce_different_hash() {
        let data_a = "a".repeat(5000);
        let data_b = "a".repeat(5001);
        let h1 = quick_screenshot_hash(&data_a);
        let h2 = quick_screenshot_hash(&data_b);
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_quick_hash_small_strings() {
        // Strings small enough to hash entirely (< 3 * 1024 = 3072)
        let data = "hello world screenshot data";
        let h1 = quick_screenshot_hash(data);
        let h2 = quick_screenshot_hash(data);
        assert_eq!(h1, h2);

        let different = "hello world screenshot datb";
        let h3 = quick_screenshot_hash(different);
        assert_ne!(h1, h3);
    }

    #[test]
    fn test_quick_hash_detects_middle_change() {
        // Create two strings that differ only in the middle
        let data_a: Vec<u8> = vec![b'a'; 5000];
        let mut data_b = data_a.clone();
        data_b[2500] = b'z'; // Change a byte in the middle

        let str_a = std::str::from_utf8(&data_a).unwrap();
        let str_b = std::str::from_utf8(&data_b).unwrap();

        let h1 = quick_screenshot_hash(str_a);
        let h2 = quick_screenshot_hash(str_b);
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_quick_hash_detects_end_change() {
        // Create two strings that differ only at the end
        let data_a: Vec<u8> = vec![b'a'; 5000];
        let mut data_b = data_a.clone();
        data_b[4999] = b'z'; // Change last byte

        let str_a = std::str::from_utf8(&data_a).unwrap();
        let str_b = std::str::from_utf8(&data_b).unwrap();

        let h1 = quick_screenshot_hash(str_a);
        let h2 = quick_screenshot_hash(str_b);
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_quick_hash_empty_string() {
        let h = quick_screenshot_hash("");
        // Should not panic, just produce some deterministic hash
        let h2 = quick_screenshot_hash("");
        assert_eq!(h, h2);
    }

    #[test]
    fn test_quick_hash_performance_large_string() {
        // Simulate a realistic screenshot base64 (~1.5MB)
        let data = "x".repeat(1_500_000);
        let start = std::time::Instant::now();
        for _ in 0..100 {
            let _ = quick_screenshot_hash(&data);
        }
        let elapsed = start.elapsed();
        // 100 hashes should complete in well under 100ms (should be < 1ms total)
        assert!(
            elapsed.as_millis() < 100,
            "100 hashes of 1.5MB string took {:?}, expected < 100ms",
            elapsed
        );
    }
}
