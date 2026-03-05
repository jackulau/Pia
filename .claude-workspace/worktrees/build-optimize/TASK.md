---
id: build-optimize
name: Add Release Build Optimizations
wave: 1
priority: 3
dependencies: []
estimated_hours: 1
tags: [build, performance, configuration]
---

## Objective

Add Rust release profile optimizations to Cargo.toml for maximum binary performance.

## Context

The current Cargo.toml has no release profile configuration, using Rust defaults. Adding LTO (Link-Time Optimization), higher optimization levels, and reduced codegen units can significantly improve runtime performance at the cost of longer compile times.

Key location:
- `src-tauri/Cargo.toml` - No `[profile.release]` section exists

## Implementation

1. **Add Release Profile to Cargo.toml**:
   ```toml
   [profile.release]
   opt-level = 3           # Maximum optimization
   lto = true              # Link-Time Optimization (smaller, faster binary)
   codegen-units = 1       # Single codegen unit for better optimization
   panic = "abort"         # Smaller binary, no unwinding overhead
   strip = true            # Strip symbols from binary
   ```

2. **Add Development Profile for Faster Builds** (optional):
   ```toml
   [profile.dev]
   opt-level = 0
   debug = true
   
   [profile.dev.package."*"]
   opt-level = 2           # Optimize dependencies even in dev
   ```

3. **Verify Tauri Build Settings**:
   - Ensure `tauri.conf.json` has appropriate bundle settings
   - Verify frontend minification is enabled for production

## Acceptance Criteria

- [ ] `[profile.release]` section added to Cargo.toml
- [ ] LTO enabled for release builds
- [ ] `cargo build --release` completes successfully
- [ ] Binary size reduced compared to before
- [ ] No runtime errors or crashes

## Files to Create/Modify

- `src-tauri/Cargo.toml` - Add profile configurations

## Integration Points

- **Provides**: Optimized release builds
- **Consumes**: Cargo build system
- **Conflicts**: None - configuration only, no code changes
