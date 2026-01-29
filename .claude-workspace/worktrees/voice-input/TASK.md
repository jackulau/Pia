---
id: voice-input
name: Voice Input Microphone Button
wave: 1
priority: 4
dependencies: []
estimated_hours: 5
tags: [frontend, backend, audio, api]
---

## Objective

Add a microphone button for hands-free instruction entry using speech-to-text.

## Context

Currently, instructions can only be entered via text input. This feature adds voice input capability using the Web Speech API (browser-native) for speech recognition, with a clean fallback for unsupported environments.

## Implementation

### Frontend (JavaScript) - Primary Implementation

1. **Add microphone button** in `index.html`:
   - Add mic button next to instruction input
   - Style with microphone icon (SVG)
   - Add recording indicator (pulsing red dot)
   - Add waveform visualization (optional)

2. **Implement speech recognition** in `src/main.js`:
   - Use Web Speech API (`webkitSpeechRecognition` / `SpeechRecognition`)
   - Check for API availability on load
   - Handle microphone permission request
   - Implement recording start/stop toggle
   - Real-time transcription display in input field
   - Auto-submit option after speech ends (configurable)

3. **Add visual feedback**:
   - Button state: idle, listening, processing
   - Transcript appears in input as user speaks
   - Error states: permission denied, no speech detected

### Backend (Rust) - Optional Enhancement

4. **Add configuration** in `src-tauri/src/config/settings.rs`:
   - Add `voice_input_enabled: bool` to GeneralConfig
   - Add `voice_auto_submit: bool` - auto-submit after speech
   - Add `voice_language: String` - recognition language (default: "en-US")

5. **Add Tauri commands** (optional, for native recording):
   - Could add native audio recording if Web Speech API is insufficient
   - For now, Web Speech API should be sufficient in WebView

### Error Handling

6. **Graceful degradation**:
   - Hide mic button if Speech API unavailable
   - Show tooltip explaining why voice is unavailable
   - Handle permission denial gracefully
   - Retry mechanism for network errors

## Acceptance Criteria

- [ ] Microphone button visible next to input field
- [ ] Clicking mic starts speech recognition
- [ ] Visual indicator shows recording state
- [ ] Speech is transcribed to input field in real-time
- [ ] Clicking mic again stops recording
- [ ] Auto-submit works when enabled
- [ ] Permission denied shows clear error message
- [ ] Button hidden on browsers without Speech API
- [ ] Language preference configurable in settings

## Files to Create/Modify

- `index.html` - Add mic button, recording indicator, styles
- `src/main.js` - Add speech recognition implementation
- `src-tauri/src/config/settings.rs` - Add voice config options (optional)

## Integration Points

- **Provides**: Hands-free instruction entry
- **Consumes**: Browser Speech API
- **Conflicts**: Shares input area with instruction-history dropdown

## Browser Compatibility

- Chrome/Chromium: Full support via webkitSpeechRecognition
- Safari: Supported via SpeechRecognition
- Firefox: Limited support (may need polyfill)
- Tauri WebView: Inherits from system browser

## Privacy Note

- Speech is processed by browser's speech service (Google for Chrome)
- No audio sent to Pia servers
- Consider adding privacy notice in settings
