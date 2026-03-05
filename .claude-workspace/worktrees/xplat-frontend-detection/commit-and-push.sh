#!/bin/bash
set -e
cd /Users/jacklau/Pia/.claude-workspace/worktrees/xplat-frontend-detection/worktree

# Stage changes
git add src-tauri/src/lib.rs

# Commit
git commit -m "feat: add tests for get_platform and parse_shortcut in lib.rs

Add unit tests for the cross-platform frontend detection feature:

- test_get_platform_returns_valid_value: verifies output is one of
  macos/windows/linux
- test_get_platform_matches_current_os: verifies output matches the
  compile-time target OS
- test_get_platform_returns_string_type: verifies non-empty output

Parse shortcut tests (related to kill switch display):
- Tests for all letter keys (A-Z), digits (0-9), function keys (F1-F12)
- Tests for all supported modifiers (Cmd, Ctrl, Shift, Alt, etc.)
- Tests for special keys (Space, Enter, Tab, Escape, etc.)
- Tests for kill switch shortcuts on both platforms
- Error path tests for unknown modifiers, unknown keys, empty input

The get_platform command replaces the deprecated navigator.platform API
in the frontend for reliable cross-platform detection."

# Push
git push -u origin Pia/xplat-frontend-detection

# Print commit SHA
echo "Commit SHA: $(git rev-parse HEAD)"
