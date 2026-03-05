---
id: screenshot-optimize
name: Optimize Screenshot Capture and Encoding
wave: 1
priority: 2
dependencies: []
estimated_hours: 3
tags: [backend, performance, io]
---

## Objective

Optimize screenshot capture by caching monitor enumeration, improving encoding efficiency, and adding optional image quality controls.

## Context

Current bottlenecks:
1. Monitor list enumerated on every capture (~5-10ms overhead)
2. Full PNG encoding every frame (CPU intensive)
3. No image downsampling for high-DPI displays
4. Base64 encoding adds 33% size overhead

Key location:
- `src-tauri/src/capture/screenshot.rs` lines 24-30 - Monitor enumeration every capture

## Implementation

1. **Cache Monitor Enumeration**:
   - Store primary monitor reference in a `OnceCell` or `lazy_static`
   - Re-enumerate only on error or configuration change
   - Example:
     ```rust
     use once_cell::sync::Lazy;
     static PRIMARY_MONITOR: Lazy<Mutex<Option<Monitor>>> = Lazy::new(|| Mutex::new(None));
     ```

2. **Add JPEG Option**:
   - JPEG encoding is faster than PNG and produces smaller files
   - Add configuration option for image format (PNG vs JPEG)
   - Default to JPEG with 80% quality for faster encoding

3. **Optional Downsampling**:
   - For 4K+ displays, downsampling to 1080p is often sufficient for LLM vision
   - Add configurable max resolution (e.g., max_screenshot_width: 1920)
   - Use fast nearest-neighbor or bilinear scaling

4. **Pre-allocated Buffers**:
   - Reuse image buffers across captures where possible
   - Avoid allocating new Vec<u8> on every frame

## Acceptance Criteria

- [ ] Monitor enumeration cached (only done once or on error)
- [ ] JPEG encoding option available and working
- [ ] Optional downsampling for high-resolution displays
- [ ] Code compiles without errors
- [ ] Screenshots still capture correctly
- [ ] Configuration option for image quality/format

## Files to Create/Modify

- `src-tauri/src/capture/screenshot.rs` - Cache monitors, add JPEG support, downsampling
- `src-tauri/src/config/mod.rs` - Add screenshot_format and max_resolution options (optional)

## Integration Points

- **Provides**: Faster screenshot capture with configurable quality
- **Consumes**: xcap crate, image crate
- **Conflicts**: None - isolated to capture module
