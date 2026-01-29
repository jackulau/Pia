import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getActionIcon } from './icons/action-icons.js';
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
const timelineList = document.getElementById('timeline-list');
const actionCount = document.getElementById('action-count');

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
let lastIteration = 0;
let lastTokens = 0;
let lastAction = null;

// Initialize
async function init() {
  await loadConfig();
  setupEventListeners();
  setupTauriListeners();
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
    settingsBtn.setAttribute('aria-expanded', 'true');
  });

  settingsCloseBtn.addEventListener('click', () => {
    settingsPanel.classList.add('hidden');
    mainModal.classList.remove('hidden');
    settingsBtn.setAttribute('aria-expanded', 'false');
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

  // Update metrics with animation
  const newIteration = state.iteration;
  const newTokens = state.total_input_tokens + state.total_output_tokens;

  if (newIteration !== lastIteration) {
    iterationValue.textContent = `${newIteration}/${state.max_iterations}`;
    triggerPulse(iterationValue);
    lastIteration = newIteration;
  }

  speedValue.textContent = state.tokens_per_second > 0
    ? `${state.tokens_per_second.toFixed(1)} tok/s`
    : '-- tok/s';

  // Update action timeline
  if (state.action_history && state.action_history.length > 0) {
    actionCount.textContent = `${state.action_history.length} action${state.action_history.length === 1 ? '' : 's'}`;

    // Clear and rebuild timeline
    timelineList.innerHTML = '';

    // Show most recent first (reverse order)
    const recentActions = [...state.action_history].reverse();

    for (const entry of recentActions) {
      const item = document.createElement('div');
      item.className = `timeline-item${entry.is_error ? ' error' : ''}`;

      const time = document.createElement('span');
      time.className = 'timeline-time';
      time.textContent = formatTimestamp(entry.timestamp);

      const actionContainer = document.createElement('span');
      actionContainer.className = 'timeline-action';

      try {
        const parsed = JSON.parse(entry.action);
        const actionType = parsed.action || 'default';
        renderActionWithIcon(actionContainer, actionType, formatAction(parsed));
      } catch {
        renderActionWithIcon(actionContainer, 'default', entry.action);
      }

      item.appendChild(time);
      item.appendChild(actionContainer);
      timelineList.appendChild(item);
    }

    // Auto-scroll to show newest (at top)
    timelineList.scrollTop = 0;
  } else {
    timelineList.innerHTML = '<div class="timeline-empty">Waiting for instruction...</div>';
    actionCount.textContent = '0 actions';
  }

  // Show/hide stop button
  stopBtn.classList.toggle('hidden', !isRunning);

  // Disable input while running
  instructionInput.disabled = isRunning;
  submitBtn.disabled = isRunning;

  // Sync aria-disabled for screen readers
  instructionInput.setAttribute('aria-disabled', isRunning.toString());
  submitBtn.setAttribute('aria-disabled', isRunning.toString());
}

// Format a single action for display (used by formatAction and batch display)
function formatSingleAction(action) {
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

// Format action for display
function formatAction(action) {
  if (action.action === 'batch') {
    const count = action.actions?.length || 0;
    if (count === 0) {
      return 'Batch (empty)';
    }
    const actionList = action.actions.map(formatSingleAction).join(' → ');
    return `Batch (${count}): ${actionList}`;
  }
  return formatSingleAction(action);
}

// Render action with icon
function renderActionWithIcon(container, actionType, text) {
  container.innerHTML = '';

  // Create icon element
  const iconEl = document.createElement('span');
  iconEl.className = `action-icon ${actionType}`;
  iconEl.innerHTML = getActionIcon(actionType);

  // Create text element
  const textEl = document.createElement('span');
  textEl.className = 'action-text';
  textEl.textContent = text;

  container.appendChild(iconEl);
  container.appendChild(textEl);
}

// Format timestamp for timeline display
function formatTimestamp(isoString) {
  const date = new Date(isoString);
  return date.toLocaleTimeString('en-US', {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false
  });
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
    settingsBtn.setAttribute('aria-expanded', 'false');
  } catch (error) {
    console.error('Failed to save settings:', error);
    showToast('Failed to save settings', 'error');
  }
}

// Show toast notification with animation
function showToast(message, type = 'info') {
  const toast = document.createElement('div');
  toast.className = `toast ${type}`;
  toast.textContent = message;
  document.body.appendChild(toast);

  setTimeout(() => {
    toast.classList.add('hiding');
    toast.addEventListener('animationend', () => toast.remove(), { once: true });
  }, 2700);
}

// Animation helper: trigger pulse on metric value
function triggerPulse(element) {
  element.classList.remove('updated');
  void element.offsetWidth; // Force reflow
  element.classList.add('updated');
}

// Animation helper: trigger slide-in on action content
function triggerSlideIn(element) {
  element.classList.remove('slide-in');
  void element.offsetWidth; // Force reflow
  element.classList.add('slide-in');
}

// Animation helper: trigger shake on input (for validation errors)
function triggerShake(element) {
  element.classList.remove('shake');
  void element.offsetWidth; // Force reflow
  element.classList.add('shake');
}

// Initialize the app
init();
