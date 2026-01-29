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

// Queue elements
const addQueueBtn = document.getElementById('add-queue-btn');
const queuePanel = document.getElementById('queue-panel');
const queueList = document.getElementById('queue-list');
const queueProgress = document.getElementById('queue-progress');
const queueStartBtn = document.getElementById('queue-start-btn');
const queueClearBtn = document.getElementById('queue-clear-btn');

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
const queueFailureMode = document.getElementById('queue-failure-mode');
const queueDelay = document.getElementById('queue-delay');

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
let queueItems = [];

// Initialize
async function init() {
  await loadConfig();
  setupEventListeners();
  setupTauriListeners();
  await refreshQueue();
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

  // Set queue settings
  queueFailureMode.value = currentConfig.general.queue_failure_mode || 'stop';
  queueDelay.value = currentConfig.general.queue_delay_ms || 500;

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

  // Queue event listeners
  addQueueBtn.addEventListener('click', addToQueue);
  queueStartBtn.addEventListener('click', startQueue);
  queueClearBtn.addEventListener('click', clearQueue);
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

  // Queue events
  await listen('queue-update', (event) => {
    queueItems = event.payload.items || [];
    renderQueue();
  });

  await listen('queue-item-started', (event) => {
    updateQueueItemStatus(event.payload.current_index, 'running');
  });

  await listen('queue-item-completed', (event) => {
    updateQueueItemStatus(event.payload.current_index, 'completed');
  });

  await listen('queue-item-failed', (event) => {
    updateQueueItemStatus(event.payload.current_index, 'failed');
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
        queue_failure_mode: queueFailureMode.value,
        queue_delay_ms: parseInt(queueDelay.value, 10) || 500,
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

// Queue Management Functions

// Parse multi-step instructions (split on "then", "after that", "next")
function parseMultiStepInstruction(text) {
  const separators = /\s*(?:,?\s*then\s+|,?\s*after that\s+|,?\s*next\s+|,?\s*finally\s+)/gi;
  const parts = text.split(separators).map(s => s.trim()).filter(s => s.length > 0);
  return parts.length > 1 ? parts : [text];
}

// Add instruction(s) to queue
async function addToQueue() {
  const instruction = instructionInput.value.trim();
  if (!instruction || isRunning) return;

  try {
    const instructions = parseMultiStepInstruction(instruction);

    if (instructions.length > 1) {
      await invoke('add_multiple_to_queue', { instructions });
    } else {
      await invoke('add_to_queue', { instruction });
    }

    instructionInput.value = '';
    await refreshQueue();
  } catch (error) {
    console.error('Failed to add to queue:', error);
    showToast('Failed to add to queue', 'error');
  }
}

// Remove item from queue
async function removeFromQueue(id) {
  try {
    await invoke('remove_from_queue', { id });
    await refreshQueue();
  } catch (error) {
    console.error('Failed to remove from queue:', error);
  }
}

// Start processing queue
async function startQueue() {
  if (isRunning || queueItems.length === 0) return;

  try {
    await invoke('start_queue');
  } catch (error) {
    console.error('Failed to start queue:', error);
    showToast(error, 'error');
  }
}

// Clear all queue items
async function clearQueue() {
  try {
    await invoke('clear_queue');
    await refreshQueue();
  } catch (error) {
    console.error('Failed to clear queue:', error);
  }
}

// Refresh queue from backend
async function refreshQueue() {
  try {
    const queue = await invoke('get_queue');
    queueItems = queue.items || [];
    renderQueue();
  } catch (error) {
    console.error('Failed to get queue:', error);
  }
}

// Render queue UI
function renderQueue() {
  const pendingItems = queueItems.filter(i => i.status === 'Pending');
  const completedItems = queueItems.filter(i => i.status === 'Completed');
  const runningItem = queueItems.find(i => i.status === 'Running');

  // Show/hide queue panel based on whether there are items
  const hasItems = queueItems.length > 0;
  queuePanel.classList.toggle('hidden', !hasItems);
  addQueueBtn.classList.toggle('has-items', pendingItems.length > 0);

  // Update progress
  queueProgress.textContent = `${completedItems.length}/${queueItems.length}`;

  // Update button states
  queueStartBtn.disabled = isRunning || pendingItems.length === 0;

  if (!hasItems) return;

  // Render items
  queueList.innerHTML = queueItems.map(item => {
    let statusClass = item.status.toLowerCase();
    let itemClass = statusClass;

    return `
      <div class="queue-item ${itemClass}" data-id="${item.id}">
        <span class="queue-item-status ${statusClass}"></span>
        <span class="queue-item-text" title="${escapeHtml(item.instruction)}">${escapeHtml(item.instruction)}</span>
        ${item.status === 'Pending' ? `
          <button class="queue-item-remove" onclick="window.removeQueueItem('${item.id}')" title="Remove">
            <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="18" y1="6" x2="6" y2="18"></line>
              <line x1="6" y1="6" x2="18" y2="18"></line>
            </svg>
          </button>
        ` : ''}
      </div>
    `;
  }).join('');
}

// Update a specific queue item's status (for real-time updates)
function updateQueueItemStatus(index, status) {
  if (queueItems[index]) {
    queueItems[index].status = status.charAt(0).toUpperCase() + status.slice(1);
    renderQueue();
  }
}

// Escape HTML for safe display
function escapeHtml(text) {
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
}

// Expose removeQueueItem to window for inline onclick handlers
window.removeQueueItem = removeFromQueue;

// Initialize the app
init();
