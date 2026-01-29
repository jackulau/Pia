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

// Template elements
const templateSelect = document.getElementById('template-select');
const saveTemplateBtn = document.getElementById('save-template-btn');
const saveTemplateDialog = document.getElementById('save-template-dialog');
const templateNameInput = document.getElementById('template-name-input');
const cancelTemplateBtn = document.getElementById('cancel-template-btn');
const confirmTemplateBtn = document.getElementById('confirm-template-btn');
const templateList = document.getElementById('template-list');

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
let cachedTemplates = [];

// Initialize
async function init() {
  await loadConfig();
  await loadTemplates();
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

// Load templates from backend
async function loadTemplates() {
  try {
    cachedTemplates = await invoke('get_templates');
    updateTemplateDropdown();
    updateTemplateList();
  } catch (error) {
    console.error('Failed to load templates:', error);
  }
}

// Update template dropdown
function updateTemplateDropdown() {
  // Clear existing options (keep the placeholder)
  templateSelect.innerHTML = '<option value="">Select a template...</option>';

  // Sort templates alphabetically by name
  const sorted = [...cachedTemplates].sort((a, b) => a.name.localeCompare(b.name));

  sorted.forEach(template => {
    const option = document.createElement('option');
    option.value = template.id;
    option.textContent = template.name;
    templateSelect.appendChild(option);
  });
}

// Update template list in settings
function updateTemplateList() {
  if (cachedTemplates.length === 0) {
    templateList.innerHTML = '<div class="no-templates">No templates saved yet</div>';
    return;
  }

  const sorted = [...cachedTemplates].sort((a, b) => a.name.localeCompare(b.name));

  templateList.innerHTML = sorted.map(template => `
    <div class="template-item" data-id="${template.id}">
      <div class="template-item-info">
        <div class="template-item-name">${escapeHtml(template.name)}</div>
        <div class="template-item-preview">${escapeHtml(template.instruction.substring(0, 60))}${template.instruction.length > 60 ? '...' : ''}</div>
      </div>
      <div class="template-item-actions">
        <button class="template-delete-btn" data-id="${template.id}" title="Delete template">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <polyline points="3 6 5 6 21 6"></polyline>
            <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
          </svg>
        </button>
      </div>
    </div>
  `).join('');

  // Add delete handlers
  templateList.querySelectorAll('.template-delete-btn').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const id = btn.dataset.id;
      await deleteTemplate(id);
    });
  });
}

// Select a template
function selectTemplate(id) {
  if (!id) {
    instructionInput.value = '';
    return;
  }

  const template = cachedTemplates.find(t => t.id === id);
  if (template) {
    instructionInput.value = template.instruction;
    instructionInput.focus();
  }
}

// Save current instruction as template
async function saveAsTemplate(name) {
  const instruction = instructionInput.value.trim();

  if (!instruction) {
    showToast('Enter an instruction first', 'error');
    return;
  }

  if (!name || !name.trim()) {
    showToast('Template name is required', 'error');
    return;
  }

  if (name.length > 50) {
    showToast('Template name must be 50 characters or less', 'error');
    return;
  }

  try {
    const template = await invoke('save_template', { name: name.trim(), instruction });
    cachedTemplates.push(template);
    updateTemplateDropdown();
    updateTemplateList();
    showToast('Template saved', 'success');
  } catch (error) {
    console.error('Failed to save template:', error);
    showToast(error, 'error');
  }
}

// Delete a template
async function deleteTemplate(id) {
  try {
    await invoke('delete_template', { id });
    cachedTemplates = cachedTemplates.filter(t => t.id !== id);
    updateTemplateDropdown();
    updateTemplateList();

    // Reset dropdown if deleted template was selected
    if (templateSelect.value === id) {
      templateSelect.value = '';
    }

    showToast('Template deleted', 'success');
  } catch (error) {
    console.error('Failed to delete template:', error);
    showToast(error, 'error');
  }
}

// Escape HTML to prevent XSS
function escapeHtml(text) {
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
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

  // Template selection
  templateSelect.addEventListener('change', (e) => {
    selectTemplate(e.target.value);
  });

  // Save as template button
  saveTemplateBtn.addEventListener('click', () => {
    const instruction = instructionInput.value.trim();
    if (!instruction) {
      showToast('Enter an instruction first', 'error');
      return;
    }
    templateNameInput.value = '';
    saveTemplateDialog.classList.remove('hidden');
    templateNameInput.focus();
  });

  // Cancel save template
  cancelTemplateBtn.addEventListener('click', () => {
    saveTemplateDialog.classList.add('hidden');
  });

  // Confirm save template
  confirmTemplateBtn.addEventListener('click', async () => {
    const name = templateNameInput.value;
    await saveAsTemplate(name);
    saveTemplateDialog.classList.add('hidden');
  });

  // Enter key in template name input
  templateNameInput.addEventListener('keydown', async (e) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      const name = templateNameInput.value;
      await saveAsTemplate(name);
      saveTemplateDialog.classList.add('hidden');
    } else if (e.key === 'Escape') {
      saveTemplateDialog.classList.add('hidden');
    }
  });

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

// Initialize the app
init();
