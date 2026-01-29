import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
// CSS is inlined in index.html for transparent window support

// DOM Elements
const mainModal = document.getElementById('main-modal');
const settingsPanel = document.getElementById('settings-panel');
const instructionInput = document.getElementById('instruction-input');
const submitBtn = document.getElementById('submit-btn');
const stopBtn = document.getElementById('stop-btn');
const settingsBtn = document.getElementById('settings-btn');
const settingsCloseBtn = document.getElementById('settings-close-btn');
const closeBtn = document.getElementById('close-btn');
const saveSettingsBtn = document.getElementById('save-settings-btn');

// Status elements
const statusDot = document.querySelector('.status-dot');
const statusText = document.querySelector('.status-text');
const iterationValue = document.getElementById('iteration-value');
const speedValue = document.getElementById('speed-value');
const tokensValue = document.getElementById('tokens-value');
const actionContent = document.getElementById('action-content');

// Settings elements
const providerSelect = document.getElementById('provider-select');
const confirmDangerous = document.getElementById('confirm-dangerous');

// Provider-specific settings
const providerSettings = {
  ollama: document.getElementById('ollama-settings'),
  anthropic: document.getElementById('anthropic-settings'),
  openai: document.getElementById('openai-settings'),
  openrouter: document.getElementById('openrouter-settings'),
};

// Confirmation dialog
const confirmationDialog = document.getElementById('confirmation-dialog');
const confirmationMessage = document.getElementById('confirmation-message');
const cancelActionBtn = document.getElementById('cancel-action-btn');
const confirmActionBtn = document.getElementById('confirm-action-btn');

// State
let isRunning = false;
let currentConfig = null;

// Touch state
let touchState = {
  startX: 0,
  startY: 0,
  startTime: 0,
  isTouching: false,
  longPressTimer: null,
  longPressTarget: null,
  swipeThreshold: 50,
  longPressDelay: 500
};

// Initialize
async function init() {
  await loadConfig();
  setupEventListeners();
  setupTauriListeners();
  setupTouchListeners();
}

// Load configuration from backend
async function loadConfig() {
  try {
    currentConfig = await invoke('get_config');
    updateSettingsUI();
  } catch (error) {
    console.error('Failed to load config:', error);
    showToast('Failed to load settings', 'error');
  }
}

// Update settings UI with current config
function updateSettingsUI() {
  if (!currentConfig) return;

  // Set provider
  providerSelect.value = currentConfig.general.default_provider;
  showProviderSettings(currentConfig.general.default_provider);

  // Set safety settings
  confirmDangerous.checked = currentConfig.general.confirm_dangerous_actions;

  // Set Ollama settings
  if (currentConfig.providers.ollama) {
    document.getElementById('ollama-host').value = currentConfig.providers.ollama.host || '';
    document.getElementById('ollama-model').value = currentConfig.providers.ollama.model || '';
  }

  // Set Anthropic settings
  if (currentConfig.providers.anthropic) {
    document.getElementById('anthropic-key').value = currentConfig.providers.anthropic.api_key || '';
    document.getElementById('anthropic-model').value = currentConfig.providers.anthropic.model || '';
  }

  // Set OpenAI settings
  if (currentConfig.providers.openai) {
    document.getElementById('openai-key').value = currentConfig.providers.openai.api_key || '';
    document.getElementById('openai-model').value = currentConfig.providers.openai.model || '';
  }

  // Set OpenRouter settings
  if (currentConfig.providers.openrouter) {
    document.getElementById('openrouter-key').value = currentConfig.providers.openrouter.api_key || '';
    document.getElementById('openrouter-model').value = currentConfig.providers.openrouter.model || '';
  }
}

// Show/hide provider-specific settings
function showProviderSettings(provider) {
  Object.keys(providerSettings).forEach(key => {
    if (providerSettings[key]) {
      providerSettings[key].classList.toggle('hidden', key !== provider);
    }
  });
}

// Setup DOM event listeners
function setupEventListeners() {
  // Submit instruction
  submitBtn.addEventListener('click', submitInstruction);
  instructionInput.addEventListener('keydown', (e) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      submitInstruction();
    }
  });

  // Stop agent
  stopBtn.addEventListener('click', stopAgent);

  // Settings
  settingsBtn.addEventListener('click', () => {
    mainModal.classList.add('hidden');
    settingsPanel.classList.remove('hidden');
  });

  settingsCloseBtn.addEventListener('click', () => {
    settingsPanel.classList.add('hidden');
    mainModal.classList.remove('hidden');
  });

  // Provider selection
  providerSelect.addEventListener('change', (e) => {
    showProviderSettings(e.target.value);
  });

  // Save settings
  saveSettingsBtn.addEventListener('click', saveSettings);

  // Close button
  closeBtn.addEventListener('click', async () => {
    await invoke('hide_window');
  });

  // Confirmation dialog
  cancelActionBtn.addEventListener('click', () => {
    confirmationDialog.classList.add('hidden');
    stopAgent();
  });

  confirmActionBtn.addEventListener('click', () => {
    confirmationDialog.classList.add('hidden');
    // Continue execution - the backend will handle this
  });
}

// Setup Tauri event listeners
async function setupTauriListeners() {
  // Agent state updates
  await listen('agent-state', (event) => {
    updateAgentState(event.payload);
  });

  // LLM streaming chunks
  await listen('llm-chunk', (event) => {
    // Could display streaming text if needed
  });

  // Confirmation required
  await listen('confirmation-required', (event) => {
    confirmationMessage.textContent = event.payload;
    confirmationDialog.classList.remove('hidden');
  });
}

// Submit instruction to agent
async function submitInstruction() {
  const instruction = instructionInput.value.trim();
  if (!instruction || isRunning) return;

  try {
    await invoke('start_agent', { instruction });
    instructionInput.value = '';
  } catch (error) {
    console.error('Failed to start agent:', error);
    showToast(error, 'error');
  }
}

// Stop the agent
async function stopAgent() {
  try {
    await invoke('stop_agent');
  } catch (error) {
    console.error('Failed to stop agent:', error);
  }
}

// Update UI with agent state
function updateAgentState(state) {
  isRunning = state.status === 'Running';

  // Update status indicator
  statusDot.className = 'status-dot';
  switch (state.status) {
    case 'Running':
      statusDot.classList.add('running');
      statusText.textContent = 'Running';
      break;
    case 'Completed':
      statusDot.classList.add('completed');
      statusText.textContent = 'Completed';
      break;
    case 'Error':
      statusDot.classList.add('error');
      statusText.textContent = 'Error';
      break;
    case 'Paused':
      statusText.textContent = 'Paused';
      break;
    default:
      statusText.textContent = 'Ready';
  }

  // Update metrics
  iterationValue.textContent = `${state.iteration}/${state.max_iterations}`;
  speedValue.textContent = state.tokens_per_second > 0
    ? `${state.tokens_per_second.toFixed(1)} tok/s`
    : '-- tok/s';
  tokensValue.textContent = (state.total_input_tokens + state.total_output_tokens).toLocaleString();

  // Update action display
  if (state.last_error) {
    actionContent.textContent = `Error: ${state.last_error}`;
    actionContent.style.color = 'var(--error)';
  } else if (state.last_action) {
    try {
      const action = JSON.parse(state.last_action);
      actionContent.textContent = formatAction(action);
      actionContent.style.color = '';
    } catch {
      actionContent.textContent = state.last_action;
      actionContent.style.color = '';
    }
  } else if (state.instruction) {
    actionContent.textContent = `Task: ${state.instruction}`;
    actionContent.style.color = '';
  }

  // Show/hide stop button
  stopBtn.classList.toggle('hidden', !isRunning);

  // Disable input while running
  instructionInput.disabled = isRunning;
  submitBtn.disabled = isRunning;
}

// Format action for display
function formatAction(action) {
  switch (action.action) {
    case 'click':
      return `Click ${action.button || 'left'} at (${action.x}, ${action.y})`;
    case 'double_click':
      return `Double-click at (${action.x}, ${action.y})`;
    case 'move':
      return `Move to (${action.x}, ${action.y})`;
    case 'type':
      const text = action.text.length > 30 ? action.text.substring(0, 30) + '...' : action.text;
      return `Type: "${text}"`;
    case 'key':
      const mods = action.modifiers?.join('+') || '';
      return `Key: ${mods ? mods + '+' : ''}${action.key}`;
    case 'scroll':
      return `Scroll ${action.direction} at (${action.x}, ${action.y})`;
    case 'complete':
      return `Completed: ${action.message}`;
    case 'error':
      return `Error: ${action.message}`;
    default:
      return JSON.stringify(action);
  }
}

// Save settings to backend
async function saveSettings() {
  try {
    // Build config object
    const config = {
      general: {
        default_provider: providerSelect.value,
        max_iterations: 50,
        confirm_dangerous_actions: confirmDangerous.checked,
      },
      providers: {
        ollama: {
          host: document.getElementById('ollama-host').value || 'http://localhost:11434',
          model: document.getElementById('ollama-model').value || 'llava',
        },
        anthropic: document.getElementById('anthropic-key').value ? {
          api_key: document.getElementById('anthropic-key').value,
          model: document.getElementById('anthropic-model').value || 'claude-sonnet-4-20250514',
        } : null,
        openai: document.getElementById('openai-key').value ? {
          api_key: document.getElementById('openai-key').value,
          model: document.getElementById('openai-model').value || 'gpt-4o',
        } : null,
        openrouter: document.getElementById('openrouter-key').value ? {
          api_key: document.getElementById('openrouter-key').value,
          model: document.getElementById('openrouter-model').value || 'anthropic/claude-sonnet-4-20250514',
        } : null,
      },
    };

    await invoke('save_config', { config });
    currentConfig = config;
    showToast('Settings saved', 'success');

    // Return to main view
    settingsPanel.classList.add('hidden');
    mainModal.classList.remove('hidden');
  } catch (error) {
    console.error('Failed to save settings:', error);
    showToast('Failed to save settings', 'error');
  }
}

// Show toast notification
function showToast(message, type = 'info') {
  const toast = document.createElement('div');
  toast.className = `toast ${type}`;
  toast.textContent = message;
  document.body.appendChild(toast);

  setTimeout(() => {
    toast.remove();
  }, 3000);
}

// Setup touch event listeners
function setupTouchListeners() {
  const dragHandle = document.querySelector('.drag-handle');

  // Touch feedback for all interactive elements
  setupTouchFeedback(submitBtn);
  setupTouchFeedback(stopBtn);
  setupTouchFeedback(settingsBtn);
  setupTouchFeedback(settingsCloseBtn);
  setupTouchFeedback(closeBtn);
  setupTouchFeedback(saveSettingsBtn);
  setupTouchFeedback(cancelActionBtn);
  setupTouchFeedback(confirmActionBtn);

  // Swipe down on drag handle to hide window
  if (dragHandle) {
    setupDragHandleSwipe(dragHandle);
  }

  // Swipe right on settings panel to close
  setupSettingsPanelSwipe();

  // Long-press on status indicator for detailed info
  setupLongPress(statusDot.parentElement, () => {
    showStatusDetails();
  });

  // Long-press on action content to copy
  setupLongPress(actionContent, () => {
    copyActionContent();
  });
}

// Add touch feedback to interactive elements
function setupTouchFeedback(element) {
  if (!element) return;

  let touchStarted = false;

  element.addEventListener('touchstart', (e) => {
    touchStarted = true;
    element.classList.add('touch-active');
  }, { passive: true });

  element.addEventListener('touchend', (e) => {
    element.classList.remove('touch-active');
    // Prevent double-firing with click
    if (touchStarted) {
      touchStarted = false;
    }
  }, { passive: true });

  element.addEventListener('touchcancel', () => {
    element.classList.remove('touch-active');
    touchStarted = false;
  }, { passive: true });
}

// Swipe handling for drag handle
function setupDragHandleSwipe(dragHandle) {
  let startY = 0;
  let isDragging = false;

  dragHandle.addEventListener('touchstart', (e) => {
    startY = e.touches[0].clientY;
    isDragging = true;
    dragHandle.classList.add('touch-active');
  }, { passive: true });

  dragHandle.addEventListener('touchmove', (e) => {
    if (!isDragging) return;
    const deltaY = e.touches[0].clientY - startY;
    // Visual feedback during swipe
    if (deltaY > 10) {
      dragHandle.style.opacity = Math.max(0.5, 1 - deltaY / 100);
    }
  }, { passive: true });

  dragHandle.addEventListener('touchend', async (e) => {
    if (!isDragging) return;
    isDragging = false;
    dragHandle.classList.remove('touch-active');
    dragHandle.style.opacity = '';

    const endY = e.changedTouches[0].clientY;
    const deltaY = endY - startY;

    // Swipe down detected - hide window
    if (deltaY > touchState.swipeThreshold) {
      await invoke('hide_window');
    }
  }, { passive: true });

  dragHandle.addEventListener('touchcancel', () => {
    isDragging = false;
    dragHandle.classList.remove('touch-active');
    dragHandle.style.opacity = '';
  }, { passive: true });
}

// Swipe handling for settings panel
function setupSettingsPanelSwipe() {
  let startX = 0;
  let isSwiping = false;

  settingsPanel.addEventListener('touchstart', (e) => {
    // Only track horizontal swipes from the left edge area
    if (e.touches[0].clientX < 50) {
      startX = e.touches[0].clientX;
      isSwiping = true;
    }
  }, { passive: true });

  settingsPanel.addEventListener('touchmove', (e) => {
    if (!isSwiping) return;
    const deltaX = e.touches[0].clientX - startX;
    // Visual feedback during swipe
    if (deltaX > 10) {
      settingsPanel.style.transform = `translateX(${Math.min(deltaX, 100)}px)`;
      settingsPanel.style.opacity = Math.max(0.5, 1 - deltaX / 200);
    }
  }, { passive: true });

  settingsPanel.addEventListener('touchend', (e) => {
    if (!isSwiping) return;
    isSwiping = false;

    const endX = e.changedTouches[0].clientX;
    const deltaX = endX - startX;

    // Swipe right detected - close settings
    if (deltaX > touchState.swipeThreshold) {
      settingsPanel.classList.add('swipe-closing');
      settingsPanel.style.transform = 'translateX(100%)';
      settingsPanel.style.opacity = '0';

      setTimeout(() => {
        settingsPanel.classList.add('hidden');
        settingsPanel.classList.remove('swipe-closing');
        settingsPanel.style.transform = '';
        settingsPanel.style.opacity = '';
        mainModal.classList.remove('hidden');
      }, 200);
    } else {
      // Reset position
      settingsPanel.style.transform = '';
      settingsPanel.style.opacity = '';
    }
  }, { passive: true });

  settingsPanel.addEventListener('touchcancel', () => {
    isSwiping = false;
    settingsPanel.style.transform = '';
    settingsPanel.style.opacity = '';
  }, { passive: true });
}

// Long-press detection
function setupLongPress(element, callback) {
  if (!element) return;

  let longPressTimer = null;
  let isLongPress = false;

  element.addEventListener('touchstart', (e) => {
    isLongPress = false;
    element.classList.add('long-press-active');

    longPressTimer = setTimeout(() => {
      isLongPress = true;
      element.classList.remove('long-press-active');
      element.classList.add('long-press-triggered');

      // Haptic feedback if available
      if (navigator.vibrate) {
        navigator.vibrate(50);
      }

      callback();

      setTimeout(() => {
        element.classList.remove('long-press-triggered');
      }, 300);
    }, touchState.longPressDelay);
  }, { passive: true });

  element.addEventListener('touchmove', (e) => {
    // Cancel long press if user moves finger
    const touch = e.touches[0];
    const rect = element.getBoundingClientRect();
    if (touch.clientX < rect.left || touch.clientX > rect.right ||
        touch.clientY < rect.top || touch.clientY > rect.bottom) {
      clearTimeout(longPressTimer);
      element.classList.remove('long-press-active');
    }
  }, { passive: true });

  element.addEventListener('touchend', (e) => {
    clearTimeout(longPressTimer);
    element.classList.remove('long-press-active');

    // Prevent click if it was a long press
    if (isLongPress) {
      e.preventDefault();
    }
  });

  element.addEventListener('touchcancel', () => {
    clearTimeout(longPressTimer);
    element.classList.remove('long-press-active');
  }, { passive: true });
}

// Show detailed status information
function showStatusDetails() {
  const statusInfo = `Status: ${statusText.textContent}\nIteration: ${iterationValue.textContent}\nSpeed: ${speedValue.textContent}\nTokens: ${tokensValue.textContent}`;

  showContextHint(statusDot.parentElement, 'Status Details');
  showToast(statusInfo.split('\n').join(' | '), 'info');
}

// Copy action content to clipboard
async function copyActionContent() {
  const text = actionContent.textContent;

  try {
    await navigator.clipboard.writeText(text);
    showContextHint(actionContent, 'Copied!');
    showToast('Action copied to clipboard', 'success');
  } catch (err) {
    // Fallback for older browsers
    const textArea = document.createElement('textarea');
    textArea.value = text;
    textArea.style.position = 'fixed';
    textArea.style.opacity = '0';
    document.body.appendChild(textArea);
    textArea.select();
    document.execCommand('copy');
    document.body.removeChild(textArea);

    showContextHint(actionContent, 'Copied!');
    showToast('Action copied to clipboard', 'success');
  }
}

// Show context hint near element
function showContextHint(element, message) {
  const hint = document.createElement('div');
  hint.className = 'context-hint';
  hint.textContent = message;

  const rect = element.getBoundingClientRect();
  hint.style.left = `${rect.left + rect.width / 2}px`;
  hint.style.top = `${rect.top - 30}px`;
  hint.style.transform = 'translateX(-50%)';

  document.body.appendChild(hint);

  setTimeout(() => {
    hint.remove();
  }, 1000);
}

// Initialize the app
init();
