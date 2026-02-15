---
id: windows-credential-paths
name: Add Windows-specific credential scanning paths
wave: 1
priority: 2
dependencies: []
estimated_hours: 4
tags: [backend, windows, credentials]
---

## Objective

Extend credential detection to scan Windows-specific file paths and configuration locations.

## Context

The credential scanning in `src-tauri/src/config/credentials.rs` currently only looks at Unix shell RC files (`.bashrc`, `.zshrc`, `.bash_profile`, fish config). On Windows, these files don't exist. Users on Windows typically set API keys via:
- Windows Environment Variables (already covered via `env::var()`)
- PowerShell profile files (`$PROFILE`)
- `.env` files in home directory (already covered)
- Windows-specific app config locations

The environment variable detection (`env::var()`) works cross-platform, but file-based scanning needs Windows paths.

## Implementation

1. **Update `shell_rc_file_paths()` in `credentials.rs`** to conditionally include Windows-specific paths:
   ```rust
   fn shell_rc_file_paths() -> Vec<PathBuf> {
       let mut paths = Vec::new();
       if let Some(home) = dirs::home_dir() {
           // Unix shell RC files
           #[cfg(not(target_os = "windows"))]
           {
               paths.push(home.join(".bashrc"));
               paths.push(home.join(".bash_profile"));
               paths.push(home.join(".profile"));
               paths.push(home.join(".zshrc"));
               paths.push(home.join(".zprofile"));
               paths.push(home.join(".config").join("fish").join("config.fish"));
           }

           // Windows PowerShell profiles
           #[cfg(target_os = "windows")]
           {
               // PowerShell profile
               if let Some(docs) = dirs::document_dir() {
                   paths.push(docs.join("PowerShell").join("Microsoft.PowerShell_profile.ps1"));
                   paths.push(docs.join("WindowsPowerShell").join("Microsoft.PowerShell_profile.ps1"));
               }
               // Also check if WSL-style dotfiles exist (some Windows devs use them)
               paths.push(home.join(".bashrc"));
               paths.push(home.join(".zshrc"));
           }
       }
       paths
   }
   ```

2. **Add PowerShell profile parsing** - PowerShell sets env vars with `$env:VAR = "value"` syntax:
   ```rust
   fn parse_powershell_profile(path: &Path) -> HashMap<String, String> {
       // Parse lines like: $env:ANTHROPIC_API_KEY = "sk-ant-..."
   }
   ```

3. **Update `dotenv_file_paths()`** to include Windows-appropriate paths:
   - `%USERPROFILE%\.env` (already covered via `dirs::home_dir()`)
   - `%APPDATA%\pia\.env`

4. **Update `scan_config_files()`** to also check Windows-specific config locations:
   - Aider configs may be at `%APPDATA%\aider\`
   - Claude CLI config may be at `%APPDATA%\claude\`

## Acceptance Criteria

- [ ] Credential scanning includes Windows PowerShell profile paths
- [ ] PowerShell `$env:VAR = "value"` syntax is parsed correctly
- [ ] Windows-specific config file locations are checked
- [ ] Unix paths are still scanned on macOS/Linux
- [ ] All existing credential tests pass
- [ ] New tests added for PowerShell profile parsing

## Files to Create/Modify

- `src-tauri/src/config/credentials.rs` - Add Windows-specific paths and PowerShell parsing

## Integration Points

- **Provides**: Windows credential detection
- **Consumes**: None
- **Conflicts**: Avoid modifying the core credential detection logic, only add new paths
