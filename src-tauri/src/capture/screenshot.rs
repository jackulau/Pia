use base64::{engine::general_purpose::STANDARD, Engine};
use image::ImageFormat;
use std::io::Cursor;
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
}

pub struct Screenshot {
    pub width: u32,
    pub height: u32,
    pub base64: String,
}

pub fn capture_primary_screen() -> Result<Screenshot, CaptureError> {
    let monitors = Monitor::all().map_err(|e| CaptureError::CaptureError(e.to_string()))?;

    let primary = monitors
        .into_iter()
        .find(|m| m.is_primary().unwrap_or(false))
        .or_else(|| Monitor::all().ok()?.into_iter().next())
        .ok_or(CaptureError::NoMonitors)?;

    let image = primary
        .capture_image()
        .map_err(|e| CaptureError::CaptureError(e.to_string()))?;

    let width = image.width();
    let height = image.height();

    // Convert to PNG and base64 encode
    let mut buffer = Cursor::new(Vec::new());
    image.write_to(&mut buffer, ImageFormat::Png)?;

    let base64 = STANDARD.encode(buffer.into_inner());

    Ok(Screenshot {
        width,
        height,
        base64,
    })
}

pub fn capture_all_screens() -> Result<Vec<Screenshot>, CaptureError> {
    let monitors = Monitor::all().map_err(|e| CaptureError::CaptureError(e.to_string()))?;

    let mut screenshots = Vec::new();

    for monitor in monitors {
        let image = monitor
            .capture_image()
            .map_err(|e| CaptureError::CaptureError(e.to_string()))?;

        let width = image.width();
        let height = image.height();

        let mut buffer = Cursor::new(Vec::new());
        image.write_to(&mut buffer, ImageFormat::Png)?;

        let base64 = STANDARD.encode(buffer.into_inner());

        screenshots.push(Screenshot {
            width,
            height,
            base64,
        });
    }

    Ok(screenshots)
}
