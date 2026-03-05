#!/bin/bash
set -e
cd /Users/jacklau/Pia/.claude-workspace/worktrees/xplat-frontend-detection/worktree

# Show git status
echo "=== Git Status ==="
git status

# Check if there are any changes to commit
if git diff --quiet && git diff --staged --quiet; then
  echo "No changes to commit beyond worktree setup."
fi

# Add only the changed files
git add src-tauri/src/lib.rs

echo "=== Git Diff (staged) ==="
git diff --staged --stat

# Commit
git commit -m "feat: add tests for get_platform command and parse_shortcut

- Add unit tests for get_platform() verifying it returns a valid platform
  string matching the current OS (macos/windows/linux)
- Add comprehensive tests for parse_shortcut() covering all letter keys,
  digits, function keys, special keys, and all supported modifier names
- Test error paths for unknown modifiers, unknown keys, and empty input
- Tests cover the kill switch shortcut patterns used on both macOS
  (Cmd+Shift+Escape) and Windows/Linux (Ctrl+Shift+Escape)

This completes the cross-platform frontend detection task by ensuring
the backend get_platform command (which replaced navigator.platform in
the frontend) has proper test coverage."

# Push to remote
git push -u origin Pia/xplat-frontend-detection

echo "=== Done ==="
echo "Commit SHA:"
git rev-parse HEAD
