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
let previousFocusElement = null;

// Initialize
async function init() {
  await loadConfig();
  setupEventListeners();
  setupTauriListeners();
  setupKeyboardNavigation();

  // Auto-focus input on app start
  instructionInput.focus();
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
    openSettings();
  });

  settingsCloseBtn.addEventListener('click', () => {
    closeSettings();
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
    restoreFocus();
  });

  confirmActionBtn.addEventListener('click', () => {
    confirmationDialog.classList.add('hidden');
    restoreFocus();
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
    previousFocusElement = document.activeElement;
    confirmationMessage.textContent = event.payload;
    confirmationDialog.classList.remove('hidden');
    // Focus cancel button (safer default)
    cancelActionBtn.focus();
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
    closeSettings();
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

// Setup keyboard navigation
function setupKeyboardNavigation() {
  // Global keyboard shortcuts
  document.addEventListener('keydown', (e) => {
    const isMod = e.metaKey || e.ctrlKey;

    // Escape key handling
    if (e.key === 'Escape') {
      e.preventDefault();
      // Close confirmation dialog first
      if (!confirmationDialog.classList.contains('hidden')) {
        confirmationDialog.classList.add('hidden');
        stopAgent();
        restoreFocus();
        return;
      }
      // Close settings panel
      if (!settingsPanel.classList.contains('hidden')) {
        closeSettings();
        return;
      }
    }

    // Cmd/Ctrl + , - Open settings
    if (isMod && e.key === ',') {
      e.preventDefault();
      if (settingsPanel.classList.contains('hidden')) {
        openSettings();
      }
      return;
    }

    // Cmd/Ctrl + . - Stop agent
    if (isMod && e.key === '.') {
      e.preventDefault();
      if (isRunning) {
        stopAgent();
      }
      return;
    }

    // Cmd/Ctrl + Enter - Force submit
    if (isMod && e.key === 'Enter') {
      e.preventDefault();
      const instruction = instructionInput.value.trim();
      if (instruction) {
        forceSubmitInstruction();
      }
      return;
    }
  });

  // Focus trap for confirmation dialog
  confirmationDialog.addEventListener('keydown', (e) => {
    if (e.key === 'Tab') {
      handleDialogFocusTrap(e);
    }
  });

  // Arrow key navigation for provider select
  providerSelect.addEventListener('keydown', (e) => {
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      // Browser handles select navigation natively
      return;
    }
  });
}

// Open settings panel
function openSettings() {
  previousFocusElement = document.activeElement;
  mainModal.classList.add('hidden');
  settingsPanel.classList.remove('hidden');
  // Focus the first focusable element in settings
  providerSelect.focus();
}

// Close settings panel
function closeSettings() {
  settingsPanel.classList.add('hidden');
  mainModal.classList.remove('hidden');
  restoreFocus();
}

// Restore focus to previous element
function restoreFocus() {
  if (previousFocusElement && document.body.contains(previousFocusElement)) {
    previousFocusElement.focus();
    previousFocusElement = null;
  } else {
    instructionInput.focus();
  }
}

// Handle focus trap within confirmation dialog
function handleDialogFocusTrap(e) {
  const focusableElements = confirmationDialog.querySelectorAll(
    'button:not([disabled]), [tabindex]:not([tabindex="-1"])'
  );
  const firstElement = focusableElements[0];
  const lastElement = focusableElements[focusableElements.length - 1];

  if (e.shiftKey) {
    // Shift+Tab: go to last element if on first
    if (document.activeElement === firstElement) {
      e.preventDefault();
      lastElement.focus();
    }
  } else {
    // Tab: go to first element if on last
    if (document.activeElement === lastElement) {
      e.preventDefault();
      firstElement.focus();
    }
  }
}

// Force submit instruction (bypasses disabled state)
async function forceSubmitInstruction() {
  const instruction = instructionInput.value.trim();
  if (!instruction) return;

  try {
    await invoke('start_agent', { instruction });
    instructionInput.value = '';
  } catch (error) {
    console.error('Failed to start agent:', error);
    showToast(error, 'error');
  }
}

// Initialize the app
init();
