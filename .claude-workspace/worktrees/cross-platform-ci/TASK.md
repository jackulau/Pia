---
id: cross-platform-ci
name: Add GitHub Actions CI for Windows and macOS builds
wave: 2
priority: 2
dependencies: [windows-bundle-config]
estimated_hours: 4
tags: [ci, devops, cross-platform]
---

## Objective

Set up a GitHub Actions CI pipeline that builds and tests the app on both Windows and macOS.

## Context

There is currently **no CI/CD pipeline**. The app needs automated builds on both platforms to catch cross-platform issues early. The app uses:
- Rust + Tauri 2.x backend
- Vite + vanilla JS frontend
- `cargo test` for Rust unit tests
- npm for frontend dependencies

## Implementation

1. **Create `.github/workflows/ci.yml`**:
   ```yaml
   name: CI
   on:
     push:
       branches: [main]
     pull_request:
       branches: [main]

   jobs:
     build:
       strategy:
         fail-fast: false
         matrix:
           platform:
             - os: macos-latest
               target: aarch64-apple-darwin
             - os: macos-latest
               target: x86_64-apple-darwin
             - os: windows-latest
               target: x86_64-pc-windows-msvc
       runs-on: ${{ matrix.platform.os }}
       steps:
         - uses: actions/checkout@v4
         - uses: actions/setup-node@v4
           with:
             node-version: 20
         - uses: dtolnay/rust-toolchain@stable
           with:
             targets: ${{ matrix.platform.target }}
         - name: Install dependencies (npm)
           run: npm ci
         - name: Run Rust tests
           run: cargo test --manifest-path src-tauri/Cargo.toml
         - name: Build frontend
           run: npm run build
         - name: Build Tauri app
           uses: tauri-apps/tauri-action@v0
           env:
             GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
           with:
             args: --target ${{ matrix.platform.target }}
   ```

2. **Add a Rust test job** that runs unit tests on both platforms.

3. **Add caching** for Rust dependencies and npm packages to speed up builds.

4. **Consider adding a release workflow** (`release.yml`) for building distributable binaries.

## Acceptance Criteria

- [ ] CI pipeline runs on both macOS and Windows
- [ ] Rust tests pass on both platforms
- [ ] Tauri app builds successfully on both platforms
- [ ] CI caches dependencies for faster builds
- [ ] Pipeline is triggered on push to main and PRs

## Files to Create/Modify

- `.github/workflows/ci.yml` - Main CI pipeline
- `.github/workflows/release.yml` (optional) - Release pipeline

## Integration Points

- **Provides**: Automated cross-platform build verification
- **Consumes**: Windows bundle config (depends on windows-bundle-config)
- **Conflicts**: None
