import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getActionIcon } from './icons/action-icons.js';
import { getCurrentWindow, LogicalSize, PhysicalPosition, PhysicalSize, availableMonitors, currentMonitor } from '@tauri-apps/api/window';
// CSS is inlined in index.html for transparent window support

// Size presets configuration
const SIZE_PRESETS = {
  mini: { width: 300, height: 180, name: 'Mini', cssClass: 'size-mini' },
  standard: { width: 420, height: 380, name: 'Standard', cssClass: 'size-standard' },
  detailed: { width: 550, height: 520, name: 'Detailed', cssClass: 'size-detailed' }
};

// DOM Elements
const mainModal = document.getElementById('main-modal');
const settingsPanel = document.getElementById('settings-panel');
const instructionInput = document.getElementById('instruction-input');
const submitBtn = document.getElementById('submit-btn');
const controlButtons = document.getElementById('control-buttons');
const pauseBtn = document.getElementById('pause-btn');
const resumeBtn = document.getElementById('resume-btn');
const recordBtn = document.getElementById('record-btn');
const stopBtn = document.getElementById('stop-btn');
const exportBtn = document.getElementById('export-btn');
const undoBtn = document.getElementById('undo-btn');
const settingsBtn = document.getElementById('settings-btn');
const settingsCloseBtn = document.getElementById('settings-close-btn');
const closeBtn = document.getElementById('close-btn');
const saveSettingsBtn = document.getElementById('save-settings-btn');
const dragHandle = document.querySelector('.drag-handle');
const expandBtn = document.getElementById('expand-btn');

// Expanded mode elements
const elapsedValue = document.getElementById('elapsed-value');
const actionsCount = document.getElementById('actions-count');
const actionHistoryList = document.getElementById('action-history-list');

// Export dialog elements
const exportDialog = document.getElementById('export-dialog');
const exportJsonBtn = document.getElementById('export-json-btn');
const exportTextBtn = document.getElementById('export-text-btn');
const exportCancelBtn = document.getElementById('export-cancel-btn');
const includeScreenshots = document.getElementById('include-screenshots');

// Queue elements
const addQueueBtn = document.getElementById('add-queue-btn');
const queuePanel = document.getElementById('queue-panel');
const queueList = document.getElementById('queue-list');
const queueProgress = document.getElementById('queue-progress');
const queueStartBtn = document.getElementById('queue-start-btn');
const queueClearBtn = document.getElementById('queue-clear-btn');

// Recording elements
const recordingPanel = document.getElementById('recording-panel');
const recordedActionsList = document.getElementById('recorded-actions-list');
const recordingCount = document.getElementById('recording-count');
const clearRecordingBtn = document.getElementById('clear-recording-btn');
const executeRecordingBtn = document.getElementById('execute-recording-btn');

// Status elements
const statusDot = document.querySelector('.status-dot');
const statusText = document.querySelector('.status-text');
const progressRingFill = document.querySelector('.progress-ring-fill');
const iterationValue = document.getElementById('iteration-value');
const speedValue = document.getElementById('speed-value');
const tokensValue = document.getElementById('tokens-value');
const timelineList = document.getElementById('timeline-list');
const actionCount = document.getElementById('action-count');
const actionContent = document.getElementById('action-content');
const screenshotThumbnail = document.getElementById('screenshot-thumbnail');
const previewPlaceholder = document.getElementById('preview-placeholder');
const thinkingContent = document.getElementById('thinking-content');

// Settings elements
const providerSelect = document.getElementById('provider-select');
const confirmDangerous = document.getElementById('confirm-dangerous');
const showOverlay = document.getElementById('show-overlay');
const visualFeedback = document.getElementById('visual-feedback');
const globalHotkeyInput = document.getElementById('global-hotkey-input');
const clearHotkeyBtn = document.getElementById('clear-hotkey-btn');
const hotkeyError = document.getElementById('hotkey-error');
const queueFailureMode = document.getElementById('queue-failure-mode');
const queueDelay = document.getElementById('queue-delay');
const speedSlider = document.getElementById('speed-slider');
const speedSliderValue = document.getElementById('speed-slider-value');
const agentSpeedValue = document.getElementById('agent-speed-value');

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
  glm: document.getElementById('glm-settings'),
  'openai-compatible': document.getElementById('openai-compatible-settings'),
};

// Confirmation dialog
const confirmationDialog = document.getElementById('confirmation-dialog');
const confirmationMessage = document.getElementById('confirmation-message');
const cancelActionBtn = document.getElementById('cancel-action-btn');
const confirmActionBtn = document.getElementById('confirm-action-btn');

// History elements
const historyBtn = document.getElementById('history-btn');
const historyDropdown = document.getElementById('history-dropdown');
const historyList = document.getElementById('history-list');
const historyClearBtn = document.getElementById('history-clear-btn');

// Preview mode
const previewToggle = document.getElementById('preview-toggle');

// Size selector buttons
const sizeMiniBtn = document.getElementById('size-mini');
const sizeStandardBtn = document.getElementById('size-standard');
const sizeDetailedBtn = document.getElementById('size-detailed');

// Position menu elements
const positionBtn = document.getElementById('position-btn');
const positionDropdown = document.getElementById('position-dropdown');
const positionOptions = document.querySelectorAll('.position-option');

// Kill switch elements
const killSwitch = document.getElementById('kill-switch');
const killSwitchShortcut = document.getElementById('kill-switch-shortcut');
const killSwitchTooltip = document.getElementById('kill-switch-tooltip');

// State
let isRunning = false;
let isPaused = false;
let isRecording = false;
let recordedActions = [];
let currentConfig = null;
let lastIteration = 0;
let lastTokens = 0;
let lastAction = null;
let isExpanded = localStorage.getItem('pia-expanded-mode') === 'true';
let actionHistory = [];
let totalActionsCount = 0;
let sessionStartTime = null;
let elapsedTimer = null;
let previousFocusElement = null;
let hasHistory = false;
let previousStatus = null;
let historyEntries = [];
let queueItems = [];
let previewMode = false;
let resizeDebounceTimer = null;
let currentSizePreset = 'standard';
let currentPosition = localStorage.getItem('pia-window-position') || null;
let cachedTemplates = [];
let killSwitchTriggered = false;
let canUndo = false;
let lastUndoableAction = null;
let renderQueueTimer = null;
let tauriUnlisteners = [];
let lastRenderedTimelineCount = 0;
let pendingAgentStateRAF = null;
let lastSubmittedInstruction = null;
let errorLog = [];

// Window sizes
const COMPACT_SIZE = { width: 420, height: 380 };
const EXPANDED_SIZE = { width: 500, height: 550 };

// Position constants
const POSITION_PADDING = 20;

// Platform detection
const isMac = navigator.platform.toUpperCase().indexOf('MAC') >= 0 ||
              navigator.userAgent.toUpperCase().indexOf('MAC') >= 0;

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
  await loadHistory();
  await loadPreviewMode();
  await restoreWindowSize();
  await loadTemplates();
  setupEventListeners();
  setupTauriListeners();
  setupKeyboardNavigation();
  setupResizeListener();
  setupSizeSelector();
  setupPositionMenu();
  setupKillSwitchDisplay();
  setupTouchListeners();
  await restoreExpandedState();
  await refreshQueue();
  await loadSavedSizePreset();
  await restoreSavedPosition();

  // Check onboarding after everything is loaded
  await checkOnboarding();

  // Auto-focus input on app start
  instructionInput.focus();
}

// Check if onboarding should be shown
async function checkOnboarding() {
  if (!currentConfig) return;
  if (currentConfig.general.onboarding_complete) return;
  await showOnboardingWizard();
}

// Show onboarding wizard with credential detection and permission check
async function showOnboardingWizard() {
  const overlay = document.getElementById('onboarding-overlay');
  if (!overlay) return;

  // Update kill switch text for platform
  const killSwitchEl = document.getElementById('onboarding-kill-switch');
  if (killSwitchEl) {
    killSwitchEl.innerHTML = isMac
      ? 'Press <strong>Cmd+Shift+Escape</strong> to stop the agent at any time.'
      : 'Press <strong>Ctrl+Shift+Escape</strong> to stop the agent at any time.';
  }

  // Detect credentials
  const credentialsEl = document.getElementById('onboarding-credentials');
  if (credentialsEl) {
    try {
      const creds = await invoke('detect_credentials');
      if (creds && creds.length > 0) {
        const items = creds.map(c =>
          `<div style="display:flex;align-items:center;gap:6px;padding:3px 0;">
            <span style="color:#32d74b;font-size:12px;">&#10003;</span>
            <span style="font-size:12px;color:rgba(255,255,255,0.8);">${escapeHtml(c.provider)}</span>
            <span style="font-size:10px;color:rgba(255,255,255,0.4);">(${escapeHtml(c.source_type || 'detected')})</span>
          </div>`
        ).join('');
        credentialsEl.innerHTML = `<p style="font-size:11px;font-weight:600;color:rgba(255,255,255,0.7);margin-bottom:4px;">Detected API Keys</p>${items}`;
      } else {
        credentialsEl.innerHTML = `<p style="font-size:11px;color:rgba(255,159,10,0.9);">No API keys detected. Configure a provider in Settings.</p>`;
      }
    } catch (e) {
      credentialsEl.innerHTML = `<p style="font-size:11px;color:rgba(255,255,255,0.5);">Could not detect API keys.</p>`;
    }
  }

  // Check permissions (macOS)
  const permissionsEl = document.getElementById('onboarding-permissions');
  if (permissionsEl && isMac) {
    try {
      const perms = await invoke('check_permissions');
      const screenOk = perms.screen_capture;
      const accessOk = perms.accessibility;
      permissionsEl.innerHTML = `
        <p style="font-size:11px;font-weight:600;color:rgba(255,255,255,0.7);margin-bottom:4px;">macOS Permissions</p>
        <div style="display:flex;align-items:center;gap:6px;padding:3px 0;">
          <span style="color:${screenOk ? '#32d74b' : '#ff453a'};font-size:12px;">${screenOk ? '&#10003;' : '&#10007;'}</span>
          <span style="font-size:12px;color:rgba(255,255,255,0.8);">Screen Recording</span>
        </div>
        <div style="display:flex;align-items:center;gap:6px;padding:3px 0;">
          <span style="color:${accessOk ? '#32d74b' : '#ff453a'};font-size:12px;">${accessOk ? '&#10003;' : '&#10007;'}</span>
          <span style="font-size:12px;color:rgba(255,255,255,0.8);">Accessibility</span>
        </div>
        ${(!screenOk || !accessOk) ? '<p style="font-size:10px;color:rgba(255,159,10,0.9);margin-top:4px;">Grant permissions in System Settings &gt; Privacy &amp; Security</p>' : ''}
      `;
    } catch (e) {
      permissionsEl.innerHTML = '';
    }
  }

  overlay.classList.remove('hidden');
}

// Load preview mode state from backend
async function loadPreviewMode() {
  try {
    previewMode = await invoke('get_preview_mode');
    updatePreviewModeUI();
  } catch (error) {
    console.error('Failed to load preview mode:', error);
  }
}

// Update UI for preview mode
function updatePreviewModeUI() {
  previewToggle.classList.toggle('active', previewMode);
  mainModal.classList.toggle('preview-mode', previewMode);
}

// Toggle preview mode
async function togglePreviewMode() {
  if (isRunning) {
    showToast('Cannot change preview mode while running', 'error');
    return;
  }

  try {
    previewMode = !previewMode;
    await invoke('set_preview_mode', { enabled: previewMode });
    updatePreviewModeUI();
    showToast(previewMode ? 'Preview mode enabled' : 'Preview mode disabled', 'info');
  } catch (error) {
    console.error('Failed to set preview mode:', error);
    previewMode = !previewMode; // Revert
    showToast('Failed to set preview mode', 'error');
  }
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

// Load history from backend
async function loadHistory() {
  try {
    historyEntries = await invoke('get_instruction_history');
    renderHistoryList();
  } catch (error) {
    console.error('Failed to load history:', error);
  }
}

// Render history dropdown list
function renderHistoryList() {
  if (historyEntries.length === 0) {
    historyList.innerHTML = '<div class="history-empty">No history yet</div>';
    return;
  }

  historyList.innerHTML = historyEntries.map((entry, index) => `
    <div class="history-item" data-index="${index}" data-instruction="${escapeHtml(entry.instruction)}">
      <div class="history-status ${entry.success ? 'success' : 'failure'}"></div>
      <div class="history-content">
        <div class="history-text">${escapeHtml(entry.instruction)}</div>
        <div class="history-time">${formatTimestamp(entry.timestamp)}</div>
      </div>
    </div>
  `).join('');
}

// Format timestamp for display
function formatTimestamp(timestamp) {
  const date = new Date(timestamp);
  const now = new Date();
  const diffMs = now - date;
  const diffMins = Math.floor(diffMs / 60000);
  const diffHours = Math.floor(diffMs / 3600000);
  const diffDays = Math.floor(diffMs / 86400000);

  if (diffMins < 1) return 'Just now';
  if (diffMins < 60) return `${diffMins}m ago`;
  if (diffHours < 24) return `${diffHours}h ago`;
  if (diffDays < 7) return `${diffDays}d ago`;
  return date.toLocaleDateString();
}

// Escape HTML to prevent XSS
function escapeHtml(text) {
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
}

// Update settings UI with current config
function updateSettingsUI() {
  if (!currentConfig) return;

  // Set provider
  providerSelect.value = currentConfig.general.default_provider;
  showProviderSettings(currentConfig.general.default_provider);

  // Set max iterations
  document.getElementById('max-iterations').value = currentConfig.general.max_iterations || 50;

  // Set speed multiplier
  const speedMultiplier = currentConfig.general.speed_multiplier || 1.0;
  if (speedSlider) {
    speedSlider.value = speedMultiplier;
    speedSliderValue.textContent = `${speedMultiplier.toFixed(2)}x`;
  }
  if (agentSpeedValue) {
    agentSpeedValue.textContent = `${speedMultiplier.toFixed(1)}x`;
  }

  // Set temperature
  const temperature = currentConfig.general.temperature != null ? currentConfig.general.temperature : 0.7;
  const tempSlider = document.getElementById('temperature-slider');
  const tempSliderValue = document.getElementById('temperature-slider-value');
  if (tempSlider) {
    tempSlider.value = temperature;
  }
  if (tempSliderValue) {
    tempSliderValue.textContent = temperature.toFixed(1);
  }

  // Set safety settings
  confirmDangerous.checked = currentConfig.general.confirm_dangerous_actions;

  // Set debug settings
  if (showOverlay) {
    showOverlay.checked = currentConfig.general.show_coordinate_overlay || false;
  }

  // Set visual feedback setting
  if (visualFeedback) {
    visualFeedback.checked = currentConfig.general.show_visual_feedback !== false;
  }

  // Set global hotkey
  if (globalHotkeyInput) {
    globalHotkeyInput.value = currentConfig.general.global_hotkey || '';
  }
  if (hotkeyError) {
    hotkeyError.style.display = 'none';
  }

  // Set queue settings
  if (queueFailureMode) {
    queueFailureMode.value = currentConfig.general.queue_failure_mode || 'stop';
  }
  if (queueDelay) {
    queueDelay.value = currentConfig.general.queue_delay_ms || 500;
  }

  // Set advanced settings
  const maxRetriesEl = document.getElementById('max-retries');
  if (maxRetriesEl) maxRetriesEl.value = currentConfig.general.max_retries ?? 3;
  const retryDelayEl = document.getElementById('retry-delay-ms');
  if (retryDelayEl) retryDelayEl.value = currentConfig.general.retry_delay_ms ?? 1000;
  const selfCorrectionEl = document.getElementById('enable-self-correction');
  if (selfCorrectionEl) selfCorrectionEl.checked = currentConfig.general.enable_self_correction !== false;
  const connectTimeoutEl = document.getElementById('connect-timeout');
  if (connectTimeoutEl) connectTimeoutEl.value = currentConfig.general.connect_timeout_secs ?? 30;
  const responseTimeoutEl = document.getElementById('response-timeout');
  if (responseTimeoutEl) responseTimeoutEl.value = currentConfig.general.response_timeout_secs ?? 300;
  const maxTokensEl = document.getElementById('max-tokens-per-task');
  if (maxTokensEl) maxTokensEl.value = currentConfig.general.max_tokens_per_task || '';

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

  // Set GLM settings
  if (currentConfig.providers.glm) {
    document.getElementById('glm-key').value = currentConfig.providers.glm.api_key || '';
    document.getElementById('glm-model').value = currentConfig.providers.glm.model || '';
  }

  // Set OpenAI Compatible settings
  if (currentConfig.providers.openai_compatible) {
    document.getElementById('openai-compatible-url').value = currentConfig.providers.openai_compatible.base_url || '';
    document.getElementById('openai-compatible-key').value = currentConfig.providers.openai_compatible.api_key || '';
    document.getElementById('openai-compatible-model').value = currentConfig.providers.openai_compatible.model || '';
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

// Test connection to a provider
async function testConnection(providerName, statusEl, btn) {
  statusEl.textContent = 'Testing...';
  statusEl.className = 'connection-status';
  btn.disabled = true;
  try {
    const healthy = await invoke('check_provider_health', { providerName });
    if (healthy) {
      statusEl.textContent = 'Connected';
      statusEl.className = 'connection-status success';
    } else {
      statusEl.textContent = 'Unreachable';
      statusEl.className = 'connection-status error';
    }
  } catch (error) {
    statusEl.textContent = 'Failed: ' + (error || 'Unknown error');
    statusEl.className = 'connection-status error';
  } finally {
    btn.disabled = false;
  }
}

// Refresh model list from a provider
async function refreshModels(providerName, datalistId, inputId, btn) {
  btn.classList.add('spinning');
  btn.disabled = true;
  try {
    const models = await invoke('list_provider_models', { providerName });
    const datalist = document.getElementById(datalistId);
    datalist.innerHTML = '';
    models.forEach(model => {
      const option = document.createElement('option');
      option.value = model;
      datalist.appendChild(option);
    });
    if (models.length > 0) {
      const input = document.getElementById(inputId);
      if (!input.value) {
        input.value = models[0];
      }
      showToast(`Found ${models.length} model${models.length === 1 ? '' : 's'}`, 'success');
    } else {
      showToast('No models found', 'error');
    }
  } catch (error) {
    showToast('Failed to list models: ' + (error || 'Unknown error'), 'error');
  } finally {
    btn.classList.remove('spinning');
    btn.disabled = false;
  }
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

  // Pause agent
  pauseBtn.addEventListener('click', pauseAgent);

  // Resume agent
  resumeBtn.addEventListener('click', resumeAgent);

  // Record instruction
  recordBtn.addEventListener('click', startRecording);

  // Recording controls
  clearRecordingBtn.addEventListener('click', clearRecording);
  executeRecordingBtn.addEventListener('click', executeRecordedActions);

  // Stop agent
  stopBtn.addEventListener('click', stopAgent);

  // Undo last action
  undoBtn.addEventListener('click', undoLastAction);

  // Keyboard shortcut for undo (Cmd+Z on Mac, Ctrl+Z on other platforms)
  document.addEventListener('keydown', (e) => {
    if ((e.metaKey || e.ctrlKey) && e.key === 'z' && !isRunning && canUndo) {
      e.preventDefault();
      undoLastAction();
    }
  });

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

  // Ollama test connection and refresh models
  document.getElementById('ollama-test-connection').addEventListener('click', async function() {
    try {
      await saveConfigQuiet();
      await testConnection('ollama', document.getElementById('ollama-connection-status'), this);
    } catch (e) { showToast('Save failed: ' + e, 'error'); }
  });
  document.getElementById('ollama-refresh-models').addEventListener('click', async function() {
    try {
      await saveConfigQuiet();
      await refreshModels('ollama', 'ollama-model-list', 'ollama-model', this);
    } catch (e) { showToast('Save failed: ' + e, 'error'); }
  });

  // OpenAI Compatible test connection and refresh models
  document.getElementById('openai-compatible-test-connection').addEventListener('click', async function() {
    try {
      await saveConfigQuiet();
      await testConnection('openai-compatible', document.getElementById('openai-compatible-connection-status'), this);
    } catch (e) { showToast('Save failed: ' + e, 'error'); }
  });
  document.getElementById('openai-compatible-refresh-models').addEventListener('click', async function() {
    try {
      await saveConfigQuiet();
      await refreshModels('openai-compatible', 'openai-compatible-model-list', 'openai-compatible-model', this);
    } catch (e) { showToast('Save failed: ' + e, 'error'); }
  });

  // Speed slider
  speedSlider.addEventListener('input', (e) => {
    const value = Math.min(3.0, Math.max(0.25, parseFloat(e.target.value)));
    e.target.value = value;
    speedSliderValue.textContent = `${value.toFixed(2)}x`;
  });

  // Temperature slider
  const temperatureSlider = document.getElementById('temperature-slider');
  const temperatureSliderValue = document.getElementById('temperature-slider-value');
  if (temperatureSlider) {
    temperatureSlider.addEventListener('input', (e) => {
      const value = Math.min(2.0, Math.max(0, parseFloat(e.target.value)));
      e.target.value = value;
      temperatureSliderValue.textContent = value.toFixed(1);
    });
  }

  // Save settings
  saveSettingsBtn.addEventListener('click', saveSettings);

  // Detect subscriptions
  const detectSubscriptionsBtn = document.getElementById('detect-subscriptions-btn');
  if (detectSubscriptionsBtn) {
    detectSubscriptionsBtn.addEventListener('click', detectSubscriptions);
  }

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
  cancelActionBtn.addEventListener('click', async () => {
    confirmationDialog.classList.add('hidden');
    try {
      await invoke('deny_action');
    } catch (error) {
      console.error('Failed to deny action:', error);
    }
    restoreFocus();
  });

  confirmActionBtn.addEventListener('click', async () => {
    confirmationDialog.classList.add('hidden');
    try {
      await invoke('confirm_action');
    } catch (error) {
      console.error('Failed to confirm action:', error);
    }
    restoreFocus();
  });

  // Drag state visual feedback
  if (dragHandle) {
    dragHandle.addEventListener('mousedown', () => {
      mainModal.classList.add('dragging');
    });

    // Listen for mouseup on window to catch release outside the handle
    window.addEventListener('mouseup', () => {
      mainModal.classList.remove('dragging');
    });
  }

  // Expand/collapse toggle
  if (expandBtn) {
    expandBtn.addEventListener('click', toggleExpandedMode);
  }

  // Clear hotkey button
  if (clearHotkeyBtn) {
    clearHotkeyBtn.addEventListener('click', async () => {
      try {
        await invoke('unregister_global_hotkey');
        globalHotkeyInput.value = '';
        hotkeyError.style.display = 'none';
        showToast('Hotkey disabled', 'success');
      } catch (error) {
        console.error('Failed to disable hotkey:', error);
        showToast('Failed to disable hotkey', 'error');
      }
    });
  }

  // Export button
  if (exportBtn) {
    exportBtn.addEventListener('click', () => {
      if (exportDialog) exportDialog.classList.remove('hidden');
    });
  }

  // Export dialog
  if (exportJsonBtn) exportJsonBtn.addEventListener('click', () => exportSession('json'));
  if (exportTextBtn) exportTextBtn.addEventListener('click', () => exportSession('text'));
  if (exportCancelBtn) {
    exportCancelBtn.addEventListener('click', () => {
      if (exportDialog) exportDialog.classList.add('hidden');
    });
  }

  // History dropdown toggle
  historyBtn.addEventListener('click', (e) => {
    e.stopPropagation();
    const isVisible = !historyDropdown.classList.contains('hidden');
    historyDropdown.classList.toggle('hidden', isVisible);
    historyBtn.classList.toggle('active', !isVisible);
  });

  // Close history dropdown when clicking outside
  document.addEventListener('click', (e) => {
    if (!historyDropdown.contains(e.target) && e.target !== historyBtn) {
      historyDropdown.classList.add('hidden');
      historyBtn.classList.remove('active');
    }
  });

  // History item click handlers
  historyList.addEventListener('click', (e) => {
    const item = e.target.closest('.history-item');
    if (item) {
      const instruction = item.dataset.instruction;
      instructionInput.value = instruction;
      historyDropdown.classList.add('hidden');
      historyBtn.classList.remove('active');
      instructionInput.focus();
    }
  });

  // History item double-click to run immediately
  historyList.addEventListener('dblclick', (e) => {
    const item = e.target.closest('.history-item');
    if (item && !isRunning) {
      const instruction = item.dataset.instruction;
      instructionInput.value = instruction;
      historyDropdown.classList.add('hidden');
      historyBtn.classList.remove('active');
      submitInstruction();
    }
  });

  // Clear history button
  historyClearBtn.addEventListener('click', async (e) => {
    e.stopPropagation();
    try {
      await invoke('clear_history');
      historyEntries = [];
      renderHistoryList();
      showToast('History cleared', 'success');
    } catch (error) {
      console.error('Failed to clear history:', error);
      showToast('Failed to clear history', 'error');
    }
  });

  // Queue event listeners
  addQueueBtn.addEventListener('click', addToQueue);
  queueStartBtn.addEventListener('click', startQueue);
  queueClearBtn.addEventListener('click', clearQueue);

  // Preview mode toggle
  previewToggle.addEventListener('click', togglePreviewMode);

  // Auto-expand textarea
  instructionInput.addEventListener('input', () => {
    instructionInput.style.height = 'auto';
    instructionInput.style.height = Math.min(instructionInput.scrollHeight, 120) + 'px';
  });

  // Retry button
  document.getElementById('retry-btn')?.addEventListener('click', async () => {
    if (!lastSubmittedInstruction || isRunning) return;
    try {
      await invoke('start_agent', { instruction: lastSubmittedInstruction });
    } catch (error) {
      showToast(error, 'error');
    }
  });

  // Advanced settings toggle
  document.getElementById('advanced-toggle')?.addEventListener('click', function() {
    const section = document.getElementById('advanced-settings');
    section.classList.toggle('hidden');
    this.innerHTML = section.classList.contains('hidden') ? 'Advanced Settings &#9656;' : 'Advanced Settings &#9662;';
  });

  // Error log clear
  document.getElementById('error-log-clear')?.addEventListener('click', () => {
    errorLog = [];
    updateErrorLogUI();
  });

  // Onboarding start button
  document.getElementById('onboarding-start-btn')?.addEventListener('click', async () => {
    document.getElementById('onboarding-overlay')?.classList.add('hidden');
    if (currentConfig) {
      currentConfig.general.onboarding_complete = true;
      try {
        await saveConfigQuiet();
      } catch (e) {
        console.error('Failed to save onboarding state:', e);
      }
    }
  });
}

// Setup Tauri event listeners
async function setupTauriListeners() {
  // Clean up previous listeners to prevent duplication
  for (const unlisten of tauriUnlisteners) {
    unlisten();
  }
  tauriUnlisteners = [];

  // Agent state updates (throttled to one per animation frame)
  tauriUnlisteners.push(await listen('agent-state', (event) => {
    if (pendingAgentStateRAF !== null) {
      cancelAnimationFrame(pendingAgentStateRAF);
    }
    pendingAgentStateRAF = requestAnimationFrame(() => {
      pendingAgentStateRAF = null;
      updateAgentState(event.payload);
    });
  }));

  // Recorded actions updates
  tauriUnlisteners.push(await listen('recorded-actions', (event) => {
    recordedActions = event.payload;
    updateRecordedActionsDisplay();
  }));

  // Confirmation required
  tauriUnlisteners.push(await listen('confirmation-required', (event) => {
    previousFocusElement = document.activeElement;
    confirmationMessage.textContent = event.payload;
    confirmationDialog.classList.remove('hidden');
    // Focus cancel button (safer default)
    cancelActionBtn.focus();
  }));

  // Retry info notification
  tauriUnlisteners.push(await listen('retry-info', (event) => {
    showToast(event.payload, 'info');
  }));

  // Parse error notification
  tauriUnlisteners.push(await listen('parse-error', (event) => {
    showToast(event.payload, 'error');
  }));

  // Instruction completed - save to history
  tauriUnlisteners.push(await listen('instruction-completed', async (event) => {
    const { instruction, success } = event.payload;
    try {
      await invoke('add_to_history', { instruction, success });
      await loadHistory();
    } catch (error) {
      console.error('Failed to save to history:', error);
    }
  }));

  // Queue events
  tauriUnlisteners.push(await listen('queue-update', (event) => {
    queueItems = event.payload.items || [];
    debouncedRenderQueue();
  }));

  tauriUnlisteners.push(await listen('queue-item-started', (event) => {
    updateQueueItemStatus(event.payload.current_index, 'running');
  }));

  tauriUnlisteners.push(await listen('queue-item-completed', (event) => {
    updateQueueItemStatus(event.payload.current_index, 'completed');
  }));

  tauriUnlisteners.push(await listen('queue-item-failed', (event) => {
    updateQueueItemStatus(event.payload.current_index, 'failed');
  }));

  // Kill switch triggered
  tauriUnlisteners.push(await listen('kill-switch-triggered', () => {
    handleKillSwitchTriggered();
  }));

  // Token budget exceeded
  tauriUnlisteners.push(await listen('token-budget-exceeded', (event) => {
    showToast(`Token budget reached (${event.payload.total_tokens.toLocaleString()}). Consider stopping the agent.`, 'warning');
  }));

  // Session summary
  tauriUnlisteners.push(await listen('session-summary', (event) => {
    const s = event.payload;
    showToast(`Done: ${s.total_iterations} iterations, ${s.total_actions} actions, ${(s.total_tokens || 0).toLocaleString()} tokens in ${Math.round(s.duration_seconds)}s`, 'success');
  }));
}

// Setup kill switch display based on platform
function setupKillSwitchDisplay() {
  if (isMac) {
    killSwitchShortcut.textContent = '⌘⇧⎋';
    killSwitchTooltip.textContent = 'Emergency Stop: Press ⌘+Shift+Escape to stop';
  } else {
    killSwitchShortcut.textContent = 'Ctrl+Shift+Esc';
    killSwitchTooltip.textContent = 'Emergency Stop: Press Ctrl+Shift+Escape to stop';
  }
}

// Update kill switch indicator state
function updateKillSwitchState(status, triggered) {
  killSwitch.className = 'kill-switch';

  if (triggered) {
    killSwitch.classList.add('kill-switch--triggered');
    killSwitchTriggered = true;

    // Reset to idle after animation completes
    setTimeout(() => {
      killSwitchTriggered = false;
      updateKillSwitchState('Idle', false);
    }, 1500);
  } else if (status === 'Running') {
    killSwitch.classList.add('kill-switch--armed');
  } else {
    killSwitch.classList.add('kill-switch--idle');
  }
}

// Handle kill switch triggered event
function handleKillSwitchTriggered() {
  updateKillSwitchState('Idle', true);
}

// Submit instruction to agent
async function submitInstruction() {
  const instruction = instructionInput.value.trim();
  if (!instruction || isRunning || isRecording) return;

  lastSubmittedInstruction = instruction;

  // Disable submit button immediately to prevent rapid double-clicks
  submitBtn.disabled = true;

  try {
    await invoke('start_agent', { instruction });
    instructionInput.value = '';
    // Reset textarea height
    instructionInput.style.height = 'auto';
  } catch (error) {
    console.error('Failed to start agent:', error);
    showToast(error, 'error');
  } finally {
    // Re-enable only if the agent is not running (agent state listener
    // will keep it disabled while running)
    if (!isRunning) {
      submitBtn.disabled = false;
    }
  }
}

// Pause the agent
async function pauseAgent() {
  try {
    await invoke('pause_agent');
  } catch (error) {
    console.error('Failed to pause agent:', error);
  }
}

// Resume the agent
async function resumeAgent() {
  try {
    await invoke('resume_agent');
  } catch (error) {
    console.error('Failed to resume agent:', error);
  }
}

// Start recording mode
async function startRecording() {
  const instruction = instructionInput.value.trim();
  if (!instruction || isRunning || isRecording) return;

  // Disable record button immediately to prevent rapid double-clicks
  if (recordBtn) recordBtn.disabled = true;

  try {
    recordedActions = [];
    updateRecordedActionsDisplay();
    recordingPanel.classList.remove('hidden');
    await invoke('start_agent_recording', { instruction });
    instructionInput.value = '';
  } catch (error) {
    console.error('Failed to start recording:', error);
    showToast(error, 'error');
  } finally {
    // Re-enable only if not running/recording (agent state listener
    // will keep it disabled while active)
    if (!isRunning && !isRecording) {
      if (recordBtn) recordBtn.disabled = false;
    }
  }
}

// Clear recorded actions
async function clearRecording() {
  try {
    await invoke('clear_recorded_actions');
    recordedActions = [];
    updateRecordedActionsDisplay();
    recordingPanel.classList.add('hidden');
    showToast('Recording cleared', 'info');
  } catch (error) {
    console.error('Failed to clear recording:', error);
    showToast('Failed to clear recording', 'error');
  }
}

// Execute recorded actions
async function executeRecordedActions() {
  if (recordedActions.length === 0) {
    showToast('No actions to execute', 'error');
    return;
  }

  showToast('Re-running instruction...', 'info');

  // Start normal agent with the same instruction
  const state = await invoke('get_agent_state');
  if (state.instruction) {
    try {
      await invoke('clear_recorded_actions');
      await invoke('start_agent', { instruction: state.instruction });
      recordingPanel.classList.add('hidden');
    } catch (error) {
      console.error('Failed to execute:', error);
      showToast('Failed to execute', 'error');
    }
  }
}

// Update recorded actions display
function updateRecordedActionsDisplay() {
  recordingCount.textContent = `${recordedActions.length} action${recordedActions.length !== 1 ? 's' : ''}`;

  if (recordedActions.length === 0) {
    recordedActionsList.innerHTML = '<div style="color: rgba(255,255,255,0.4); font-size: 11px; text-align: center; padding: 10px;">No actions recorded yet...</div>';
    return;
  }

  recordedActionsList.innerHTML = recordedActions.map((action, index) => {
    let actionDesc = 'Unknown action';
    try {
      const parsed = JSON.parse(action.action);
      actionDesc = formatAction(parsed);
    } catch {
      actionDesc = action.action;
    }

    const reasoning = action.reasoning
      ? `<div class="recorded-action-reasoning">${truncate(action.reasoning, 80)}</div>`
      : '';

    return `
      <div class="recorded-action-item">
        <span class="recorded-action-num">${index + 1}.</span>
        <div class="recorded-action-content">
          <div class="recorded-action-desc">${actionDesc}</div>
          ${reasoning}
        </div>
      </div>
    `;
  }).join('');
}

// Truncate string helper
function truncate(str, maxLen) {
  if (str.length <= maxLen) return str;
  return str.substring(0, maxLen) + '...';
}

// Stop the agent
async function stopAgent() {
  try {
    await invoke('stop_agent');
  } catch (error) {
    console.error('Failed to stop agent:', error);
  }
}

// Update progress ring
function updateProgressRing(state) {
  if (!progressRingFill) return;

  // Calculate progress percentage (0-100)
  const maxIterations = state.max_iterations || 50;
  const progress = maxIterations > 0 ? (state.iteration / maxIterations) * 100 : 0;

  // SVG circle circumference calculation: 2 * PI * r
  // With r=10 and viewBox 24x24, the circumference is ~62.83
  // We use stroke-dasharray with percentage values relative to 100
  const circumference = 2 * Math.PI * 10; // ~62.83
  const dashLength = (progress / 100) * circumference;

  progressRingFill.style.strokeDasharray = `${dashLength}, ${circumference}`;

  // Update ring color based on status
  progressRingFill.className = 'progress-ring-fill';
  switch (state.status) {
    case 'Running':
      progressRingFill.classList.add('running');
      break;
    case 'Completed':
      progressRingFill.classList.add('completed');
      // Show full ring on completion
      progressRingFill.style.strokeDasharray = `${circumference}, ${circumference}`;
      break;
    case 'Error':
      progressRingFill.classList.add('error');
      // Show full ring on error
      progressRingFill.style.strokeDasharray = `${circumference}, ${circumference}`;
      break;
    default:
      // Idle/Ready state - show empty ring
      progressRingFill.style.strokeDasharray = `0, ${circumference}`;
  }
}

// Undo the last action
async function undoLastAction() {
  if (isRunning || !canUndo) return;

  try {
    const result = await invoke('undo_last_action');
    showToast(result, 'success');
  } catch (error) {
    console.error('Failed to undo action:', error);
    showToast(error, 'error');
  }
}

// Update UI with agent state
function updateAgentState(state) {
  const wasRunning = isRunning;
  isRunning = state.status === 'Running';
  isPaused = state.status === 'Paused';
  isRecording = state.status === 'Recording';
  canUndo = state.can_undo && !isRunning;
  lastUndoableAction = state.last_undoable_action;

  // Start timer when agent starts running
  if (isRunning && !wasRunning) {
    startElapsedTimer();
  }

  // Stop timer when agent stops running
  if (!isRunning && wasRunning) {
    stopElapsedTimer();
  }

  // Update preview mode from state
  if (state.preview_mode !== previewMode) {
    previewMode = state.preview_mode;
    updatePreviewModeUI();
  }

  // Update kill switch indicator (unless it was just triggered)
  if (!killSwitchTriggered || state.status !== 'Running') {
    updateKillSwitchState(state.status, state.kill_switch_triggered);
    if (killSwitchTriggered && state.status !== 'Running') {
      killSwitchTriggered = false;
    }
  }

  // Update undo button state
  if (undoBtn) {
    undoBtn.disabled = !canUndo;
    undoBtn.title = lastUndoableAction ? `Undo: ${lastUndoableAction}` : 'Nothing to undo';
    if (canUndo) {
      undoBtn.classList.remove('hidden');
    } else {
      undoBtn.classList.add('hidden');
    }
  }

  // Update status indicator
  let statusLabel = 'Ready';
  if (statusDot) {
    statusDot.className = 'status-dot';
    switch (state.status) {
      case 'Running':
        if (previewMode) {
          statusDot.classList.add('preview');
          statusLabel = 'Preview';
        } else {
          statusDot.classList.add('running');
          statusLabel = 'Running';
        }
        break;
      case 'Recording':
        statusDot.classList.add('recording');
        if (statusText) statusText.textContent = 'Recording';
        break;
      case 'Completed':
        statusDot.classList.add('completed');
        statusLabel = previewMode ? 'Preview Done' : 'Completed';
        break;
      case 'Error':
        statusDot.classList.add('error');
        statusLabel = 'Error';
        break;
      case 'Paused':
        statusDot.classList.add('paused');
        statusLabel = 'Paused';
        break;
      case 'Retrying':
        statusDot.classList.add('retrying');
        statusLabel = 'Retrying';
        break;
      case 'AwaitingConfirmation':
        statusDot.classList.add('awaiting');
        statusLabel = 'Awaiting Confirmation';
        break;
      default:
        statusLabel = 'Ready';
    }
    if (statusText) statusText.textContent = statusLabel;
  }

  // Announce status changes to screen readers
  if (state.status !== previousStatus) {
    announceStatus(statusLabel);
    previousStatus = state.status;
  }

  // Update metrics with animation
  const newIteration = state.iteration;
  const newTokens = state.total_input_tokens + state.total_output_tokens;

  if (newIteration !== lastIteration) {
    iterationValue.textContent = `${newIteration}/${state.max_iterations}`;
    triggerPulse(iterationValue);
    lastIteration = newIteration;
  }

  if (newTokens !== lastTokens) {
    tokensValue.textContent = newTokens.toLocaleString();
    triggerPulse(tokensValue);
    lastTokens = newTokens;
  }

  // Update progress ring
  updateProgressRing(state);

  speedValue.textContent = state.tokens_per_second > 0
    ? `${state.tokens_per_second.toFixed(1)} tok/s`
    : '-- tok/s';

  // Update retry indicator if retries occurred
  const retryIndicator = document.getElementById('retry-indicator');
  if (retryIndicator) {
    if (state.total_retries > 0) {
      retryIndicator.textContent = `${state.total_retries} retries`;
      retryIndicator.classList.remove('hidden');
    } else {
      retryIndicator.classList.add('hidden');
    }
  }

  // Update thinking display
  if (thinkingContent) {
    if (state.last_reasoning) {
      thinkingContent.textContent = truncateText(state.last_reasoning, 150);
    } else if (state.status === 'Running') {
      thinkingContent.textContent = 'Analyzing screen...';
    } else if (state.status === 'Idle') {
      thinkingContent.textContent = 'Waiting for task...';
    }
  }

  // Update action display with preview mode awareness
  if (actionContent) {
    actionContent.classList.remove('preview-action');
    if (state.last_error) {
      actionContent.textContent = `Error: ${state.last_error}`;
      actionContent.style.color = 'var(--error)';
    } else if (state.last_action) {
      // Check if action has [PREVIEW] prefix
      const isPreviewAction = state.last_action.startsWith('[PREVIEW]');
      let actionText = state.last_action;

      if (isPreviewAction) {
        actionContent.classList.add('preview-action');
        actionText = state.last_action.substring(10).trim(); // Remove "[PREVIEW] "
      }

      try {
        const action = JSON.parse(actionText);
        const formatted = formatAction(action);
        actionContent.textContent = isPreviewAction ? `Would: ${formatted}` : formatted;
        if (!isPreviewAction) {
          actionContent.style.color = '';
        }
      } catch {
        actionContent.textContent = isPreviewAction ? `Would: ${actionText}` : actionText;
        if (!isPreviewAction) {
          actionContent.style.color = '';
        }
      }
    }
  }

  // Update screenshot thumbnail
  if (state.last_screenshot) {
    if (screenshotThumbnail) {
      screenshotThumbnail.src = `data:image/png;base64,${state.last_screenshot}`;
      screenshotThumbnail.classList.add('visible');
    }
    if (previewPlaceholder) previewPlaceholder.classList.add('hidden');
  } else {
    if (screenshotThumbnail) screenshotThumbnail.classList.remove('visible');
    if (previewPlaceholder) previewPlaceholder.classList.remove('hidden');
  }

  // Update action timeline
  if (state.action_history && state.action_history.length > 0) {
    if (actionCount) {
      actionCount.textContent = `${state.action_history.length} action${state.action_history.length === 1 ? '' : 's'}`;
    }
    if (actionsCount) {
      actionsCount.textContent = state.action_history.length;
    }

    // Incremental timeline update: only append new items
    if (timelineList) {
      const currentCount = state.action_history.length;

      if (currentCount < lastRenderedTimelineCount) {
        // State was reset (new session) - clear and rebuild
        timelineList.innerHTML = '';
        lastRenderedTimelineCount = 0;
      }

      const newCount = currentCount - lastRenderedTimelineCount;
      if (newCount > 0) {
        // Remove empty placeholder if present
        const placeholder = timelineList.querySelector('.timeline-empty');
        if (placeholder) placeholder.remove();

        // Append only new items (prepend to show most recent first)
        for (let i = lastRenderedTimelineCount; i < currentCount; i++) {
          const entry = state.action_history[i];
          const item = document.createElement('div');
          item.className = `timeline-item${entry.is_error ? ' error' : ''}`;

          const time = document.createElement('span');
          time.className = 'timeline-time';
          time.textContent = formatTimeOnly(entry.timestamp);

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
          // Prepend so newest appears at top (no reverse copy needed)
          timelineList.insertBefore(item, timelineList.firstChild);
        }

        lastRenderedTimelineCount = currentCount;
      }
    }

    // Update expanded mode action history
    if (actionHistoryList && state.last_action) {
      try {
        const action = JSON.parse(state.last_action);
        const formattedAction = formatAction(action);
        if (actionHistory.length === 0 || actionHistory[0] !== formattedAction) {
          addToActionHistory(action);
        }
      } catch {
        // Skip if not valid JSON
      }
    }

    // Auto-scroll to show newest (at top)
    if (timelineList) timelineList.scrollTop = 0;
  } else {
    if (timelineList) timelineList.innerHTML = '<div class="timeline-empty">Waiting for instruction...</div>';
    if (actionCount) actionCount.textContent = '0 actions';
    lastRenderedTimelineCount = 0;
  }

  // Show/hide control buttons based on state
  const showControls = isRunning || isPaused;
  controlButtons.classList.toggle('hidden', !showControls);
  pauseBtn.classList.toggle('hidden', isPaused);
  resumeBtn.classList.toggle('hidden', !isPaused);

  // Show/hide stop button and recording panel
  stopBtn.classList.toggle('hidden', !isRunning && !isRecording);

  // Show recording panel when recording or when there are recorded actions
  if (isRecording || state.recorded_actions_count > 0) {
    recordingPanel.classList.remove('hidden');
  }

  // Disable input while running, paused, or recording
  const inputDisabled = isRunning || isPaused || isRecording;
  instructionInput.disabled = inputDisabled;
  submitBtn.disabled = inputDisabled;
  if (recordBtn) recordBtn.disabled = inputDisabled;

  // Sync aria-disabled for screen readers
  instructionInput.setAttribute('aria-disabled', inputDisabled.toString());
  submitBtn.setAttribute('aria-disabled', inputDisabled.toString());

  // Disable preview toggle while running
  if (previewToggle) {
    previewToggle.style.pointerEvents = isRunning ? 'none' : 'auto';
    previewToggle.style.opacity = isRunning ? '0.5' : '1';
  }

  // Show/hide retry button
  const retryBtn = document.getElementById('retry-btn');
  if (retryBtn) {
    retryBtn.classList.toggle('hidden',
      !(state.status === 'Error' || state.status === 'Completed') || !lastSubmittedInstruction);
  }

  // Update cost estimation
  const costValue = document.getElementById('cost-value');
  if (costValue && currentConfig) {
    const provider = currentConfig.general?.default_provider || '';
    let model = '';
    if (currentConfig.providers?.[provider]) {
      model = currentConfig.providers[provider].model || '';
    } else if (provider === 'openai-compatible' && currentConfig.providers?.openai_compatible) {
      model = currentConfig.providers.openai_compatible.model || '';
    }
    const cost = estimateCost(provider, model, state.total_input_tokens, state.total_output_tokens);
    costValue.textContent = cost;
  }

  // Update export button visibility
  updateHistoryCount();
}

// Truncate text with ellipsis
function truncateText(text, maxLength) {
  if (!text) return '';
  if (text.length <= maxLength) return text;
  return text.substring(0, maxLength).trim() + '...';
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
    case 'drag':
      return `Drag: (${action.start_x}, ${action.start_y}) → (${action.end_x}, ${action.end_y})`;
    case 'wait_for_element':
      return `⏳ Wait: ${action.description} (${action.timeout_ms || 5000}ms)`;
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

// Format timestamp for timeline display (HH:MM:SS format)
function formatTimeOnly(isoString) {
  const date = new Date(isoString);
  return date.toLocaleTimeString('en-US', {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false
  });
}

// Build config from current UI state and save to backend (no UI side-effects)
async function saveConfigQuiet() {
  const maxIterInput = document.getElementById('max-iterations');
  let maxIterations = parseInt(maxIterInput.value, 10);
  if (isNaN(maxIterations) || maxIterations < 1) maxIterations = 1;
  if (maxIterations > 200) maxIterations = 200;
  maxIterInput.value = maxIterations;

  if (hotkeyError) {
    hotkeyError.style.display = 'none';
  }

  let newHotkey = null;
  if (globalHotkeyInput) {
    newHotkey = globalHotkeyInput.value.trim() || null;
    const currentHotkey = currentConfig?.general?.global_hotkey || null;

    if (newHotkey !== currentHotkey) {
      if (newHotkey) {
        await invoke('set_global_hotkey', { shortcut: newHotkey });
      } else {
        await invoke('unregister_global_hotkey');
      }
    }
  }

  const config = {
    general: {
      default_provider: providerSelect.value,
      max_iterations: maxIterations,
      confirm_dangerous_actions: confirmDangerous.checked,
      show_coordinate_overlay: showOverlay ? showOverlay.checked : false,
      show_visual_feedback: visualFeedback ? visualFeedback.checked : true,
      global_hotkey: newHotkey,
      queue_failure_mode: queueFailureMode ? queueFailureMode.value : 'stop',
      queue_delay_ms: queueDelay ? parseInt(queueDelay.value, 10) || 500 : 500,
      speed_multiplier: speedSlider ? Math.min(3.0, Math.max(0.25, parseFloat(speedSlider.value))) : 1.0,
      temperature: document.getElementById('temperature-slider') ? parseFloat(document.getElementById('temperature-slider').value) : null,
      max_retries: parseInt(document.getElementById('max-retries')?.value, 10) || 3,
      retry_delay_ms: parseInt(document.getElementById('retry-delay-ms')?.value, 10) || 1000,
      enable_self_correction: document.getElementById('enable-self-correction')?.checked !== false,
      connect_timeout_secs: parseInt(document.getElementById('connect-timeout')?.value, 10) || 30,
      response_timeout_secs: parseInt(document.getElementById('response-timeout')?.value, 10) || 300,
      max_tokens_per_task: parseInt(document.getElementById('max-tokens-per-task')?.value, 10) || null,
      onboarding_complete: currentConfig?.general?.onboarding_complete || false,
    },
    providers: {
      ollama: {
        host: document.getElementById('ollama-host').value || 'http://localhost:11434',
        model: document.getElementById('ollama-model').value || 'llava',
        temperature: document.getElementById('temperature-slider') ? parseFloat(document.getElementById('temperature-slider').value) : null,
      },
      anthropic: document.getElementById('anthropic-key').value ? {
        api_key: document.getElementById('anthropic-key').value,
        model: document.getElementById('anthropic-model').value || 'claude-sonnet-4-20250514',
        temperature: document.getElementById('temperature-slider') ? parseFloat(document.getElementById('temperature-slider').value) : null,
      } : null,
      openai: document.getElementById('openai-key').value ? {
        api_key: document.getElementById('openai-key').value,
        model: document.getElementById('openai-model').value || 'gpt-4o',
        temperature: document.getElementById('temperature-slider') ? parseFloat(document.getElementById('temperature-slider').value) : null,
      } : null,
      openrouter: document.getElementById('openrouter-key').value ? {
        api_key: document.getElementById('openrouter-key').value,
        model: document.getElementById('openrouter-model').value || 'anthropic/claude-sonnet-4-20250514',
        temperature: document.getElementById('temperature-slider') ? parseFloat(document.getElementById('temperature-slider').value) : null,
      } : null,
      glm: document.getElementById('glm-key').value ? {
        api_key: document.getElementById('glm-key').value,
        model: document.getElementById('glm-model').value || 'glm-4v',
        temperature: document.getElementById('temperature-slider') ? parseFloat(document.getElementById('temperature-slider').value) : null,
      } : null,
      openai_compatible: document.getElementById('openai-compatible-url').value ? {
        base_url: document.getElementById('openai-compatible-url').value,
        api_key: document.getElementById('openai-compatible-key').value || null,
        model: document.getElementById('openai-compatible-model').value || 'default',
        temperature: document.getElementById('temperature-slider') ? parseFloat(document.getElementById('temperature-slider').value) : null,
      } : null,
    },
  };

  await invoke('save_config', { config });
  currentConfig = config;
  if (agentSpeedValue) agentSpeedValue.textContent = `${config.general.speed_multiplier.toFixed(1)}x`;

  if (config.general.show_visual_feedback) {
    await invoke('show_overlay');
  } else {
    await invoke('hide_overlay');
  }

  return config;
}

// Save settings to backend
async function saveSettings() {
  try {
    await saveConfigQuiet();
    showToast('Settings saved', 'success');
    closeSettings();
  } catch (error) {
    console.error('Failed to save settings:', error);
    if (hotkeyError && error && String(error).toLowerCase().includes('hotkey')) {
      hotkeyError.textContent = error;
      hotkeyError.style.display = 'block';
    } else {
      showToast('Failed to save settings', 'error');
    }
  }
}

// Vision model prefixes (mirrors backend VISION_MODEL_PREFIXES)
const VISION_MODEL_PREFIXES = [
  'llava', 'bakllava', 'llama3.2-vision', 'llama3-vision',
  'moondream', 'minicpm-v', 'nanollava', 'llava-llama3', 'llava-phi3', 'obsidian'
];

function isVisionModel(name) {
  const lower = name.toLowerCase();
  return VISION_MODEL_PREFIXES.some(p => lower.startsWith(p));
}

// Source type labels for display
const SOURCE_TYPE_LABELS = {
  env: 'ENV',
  dotenv: '.env',
  shell: 'Shell',
  config: 'Config',
  service: 'Service',
  unknown: 'Other',
};

// Detect available API credentials from all sources
async function detectSubscriptions() {
  const btn = document.getElementById('detect-subscriptions-btn');
  const resultsContainer = document.getElementById('detect-results');
  const resultsList = document.getElementById('detect-results-list');

  if (!btn || !resultsContainer || !resultsList) return;

  btn.disabled = true;
  btn.textContent = 'Detecting...';
  resultsList.innerHTML = '';
  resultsContainer.classList.add('hidden');

  try {
    const detected = await invoke('detect_credentials');

    if (detected && detected.length > 0) {
      resultsContainer.classList.remove('hidden');

      // Group by provider
      const grouped = {};
      for (const cred of detected) {
        if (!grouped[cred.provider]) grouped[cred.provider] = [];
        grouped[cred.provider].push(cred);
      }

      // Sort providers: those with valid keys first, then alphabetical
      const providerNames = Object.keys(grouped).sort((a, b) => {
        const aValid = grouped[a].some(c => c.is_valid);
        const bValid = grouped[b].some(c => c.is_valid);
        if (aValid !== bValid) return bValid ? 1 : -1;
        return a.localeCompare(b);
      });

      for (const provider of providerNames) {
        const creds = grouped[provider];
        // Sort by priority (lower = better)
        creds.sort((a, b) => a.priority - b.priority);

        const group = document.createElement('div');
        group.className = 'detect-provider-group';

        const header = document.createElement('div');
        header.className = 'detect-provider-header';
        header.textContent = provider;
        group.appendChild(header);

        for (const cred of creds) {
          const item = document.createElement('div');
          item.className = 'detect-result-item';

          const sourceLabel = SOURCE_TYPE_LABELS[cred.source_type] || cred.source_type;
          const badgeClass = 'detect-source-badge source-' + escapeHtml(cred.source_type);
          const validClass = cred.is_valid ? 'valid' : 'invalid';
          const validIcon = cred.is_valid ? '\u2713' : '!';
          const validTitle = cred.is_valid ? 'Valid key format' : 'Key format not recognized';

          let infoHtml = `
            <div class="detect-result-info">
              <div class="detect-result-top-row">
                <span class="detect-result-provider">${escapeHtml(cred.key_preview || '(no key)')}</span>
                <span class="${escapeHtml(badgeClass)}">${escapeHtml(sourceLabel)}</span>
                <span class="detect-validation-icon ${escapeHtml(validClass)}" title="${escapeHtml(validTitle)}">${validIcon}</span>
              </div>
              <div class="detect-source-path" title="${escapeHtml(cred.source)}">${escapeHtml(cred.source)}</div>`;

          // Ollama-specific: show host and model tags
          if (cred.provider === 'ollama') {
            if (cred.host) {
              infoHtml += `<div class="detect-ollama-info">${escapeHtml(cred.host)}</div>`;
            }
            if (cred.available_models && cred.available_models.length > 0) {
              infoHtml += '<div class="detect-ollama-models">';
              for (const m of cred.available_models) {
                const vision = isVisionModel(m);
                const tagClass = vision ? 'detect-ollama-model-tag vision' : 'detect-ollama-model-tag';
                infoHtml += `<span class="${tagClass}">${escapeHtml(m)}</span>`;
              }
              infoHtml += '</div>';
            }
          }

          infoHtml += '</div>';

          item.innerHTML = infoHtml +
            `<button class="detect-result-apply" data-provider="${escapeHtml(cred.provider)}">Apply</button>`;
          group.appendChild(item);
        }

        resultsList.appendChild(group);
      }

      // Attach apply handlers
      resultsList.querySelectorAll('.detect-result-apply').forEach(applyBtn => {
        applyBtn.addEventListener('click', () => {
          applyDetectedCredential(applyBtn.dataset.provider);
        });
      });
    } else {
      resultsContainer.classList.remove('hidden');
      resultsList.innerHTML = `<div class="detect-no-results">
        No credentials detected. Checked:
        <ul class="detect-hint-list">
          <li>Environment variables (ANTHROPIC_API_KEY, OPENAI_API_KEY, etc.)</li>
          <li>.env files in common locations</li>
          <li>Shell config files (~/.bashrc, ~/.zshrc)</li>
          <li>CLI config files (~/.claude.json)</li>
          <li>Running Ollama instances</li>
        </ul>
      </div>`;
    }
  } catch (error) {
    console.error('Failed to detect credentials:', error);
    resultsContainer.classList.remove('hidden');
    resultsList.innerHTML = '<div class="detect-no-results">Detection failed: ' + escapeHtml(String(error)) + '</div>';
  } finally {
    btn.disabled = false;
    btn.innerHTML = `
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="11" cy="11" r="8"></circle>
        <line x1="21" y1="21" x2="16.65" y2="16.65"></line>
      </svg>
      Detect API Subscriptions
    `;
  }
}

// Apply a detected credential via the backend
async function applyDetectedCredential(provider) {
  try {
    await invoke('apply_detected_credential', { provider });
    providerSelect.value = provider;
    showProviderSettings(provider);
    showToast('Applied ' + provider + ' credentials', 'success');
  } catch (error) {
    console.error('Failed to apply credential:', error);
    showToast('Failed to apply: ' + String(error), 'error');
  }
}

// Export session to file
async function exportSession(format) {
  try {
    let content, filename, mimeType;

    if (format === 'json') {
      const includeScreenshotsValue = includeScreenshots?.checked ?? false;
      content = await invoke('export_session_json', { includeScreenshots: includeScreenshotsValue });
      filename = `pia-session-${Date.now()}.json`;
      mimeType = 'application/json';
    } else {
      content = await invoke('export_session_text');
      filename = `pia-session-${Date.now()}.txt`;
      mimeType = 'text/plain';
    }

    // Create download
    const blob = new Blob([content], { type: mimeType });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = filename;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);

    if (exportDialog) exportDialog.classList.add('hidden');
    showToast(`Exported to ${filename}`, 'success');
  } catch (error) {
    console.error('Failed to export session:', error);
    showToast('Failed to export session', 'error');
  }
}

// Check and update history count
async function updateHistoryCount() {
  try {
    const count = await invoke('get_session_history_count');
    hasHistory = count > 0;
    if (exportBtn) exportBtn.classList.toggle('hidden', !hasHistory || isRunning);
  } catch (error) {
    console.error('Failed to get history count:', error);
  }
}

// Model pricing table (per 1M tokens)
const MODEL_PRICING = {
  'claude-sonnet-4-20250514': { input: 3.0, output: 15.0 },
  'claude-opus-4-20250514': { input: 15.0, output: 75.0 },
  'claude-3-5-sonnet-20241022': { input: 3.0, output: 15.0 },
  'claude-3-5-haiku-20241022': { input: 1.0, output: 5.0 },
  'gpt-4o': { input: 2.5, output: 10.0 },
  'gpt-4o-mini': { input: 0.15, output: 0.6 },
  'gpt-4-turbo': { input: 10.0, output: 30.0 },
  'o1': { input: 15.0, output: 60.0 },
  'o1-mini': { input: 3.0, output: 12.0 },
  'anthropic/claude-sonnet-4-20250514': { input: 3.0, output: 15.0 },
  'anthropic/claude-opus-4-20250514': { input: 15.0, output: 75.0 },
  'anthropic/claude-3-5-sonnet-20241022': { input: 3.0, output: 15.0 },
  'openai/gpt-4o': { input: 2.5, output: 10.0 },
};

// Estimate cost from token counts
function estimateCost(provider, model, inputTokens, outputTokens) {
  if (!inputTokens && !outputTokens) return '--';
  const pricing = MODEL_PRICING[model];
  if (!pricing) return '~$?';
  const cost = (inputTokens / 1_000_000) * pricing.input + (outputTokens / 1_000_000) * pricing.output;
  if (cost < 0.01) return `~$${cost.toFixed(4)}`;
  if (cost < 1.0) return `~$${cost.toFixed(3)}`;
  return `~$${cost.toFixed(2)}`;
}

// Update error log panel UI
function updateErrorLogUI() {
  const panel = document.getElementById('error-log-panel');
  const content = document.getElementById('error-log-content');
  const badge = document.getElementById('error-badge');
  if (!panel || !content) return;

  if (errorLog.length === 0) {
    panel.classList.add('hidden');
    if (badge) badge.textContent = '0';
    content.innerHTML = '';
    return;
  }

  panel.classList.remove('hidden');
  if (badge) badge.textContent = errorLog.length.toString();
  content.innerHTML = errorLog.map(entry =>
    `<div style="padding:4px 0;border-bottom:1px solid rgba(255,255,255,0.05);font-size:11px;">
      <span style="color:rgba(255,255,255,0.4);margin-right:6px;">${entry.time}</span>
      <span style="color:#ff453a;">${escapeHtml(entry.message)}</span>
    </div>`
  ).join('');
  content.scrollTop = content.scrollHeight;
}

// Show toast notification with animation
function showToast(message, type = 'info') {
  // Persist errors to error log
  if (type === 'error') {
    const now = new Date();
    const time = now.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit', second: '2-digit', hour12: false });
    errorLog.push({ time, message: String(message) });
    if (errorLog.length > 50) errorLog.shift(); // cap at 50 entries
    updateErrorLogUI();
  }

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

// Announce status changes to screen readers
function announceStatus(status) {
  const announcement = document.createElement('div');
  announcement.setAttribute('role', 'status');
  announcement.setAttribute('aria-live', 'polite');
  announcement.className = 'visually-hidden';
  announcement.textContent = `Agent status: ${status}`;
  document.body.appendChild(announcement);
  setTimeout(() => announcement.remove(), 1000);
}

// Toggle expanded mode
async function toggleExpandedMode() {
  isExpanded = !isExpanded;
  await applyExpandedState();
  localStorage.setItem('pia-expanded-mode', isExpanded.toString());
}

// Apply expanded state to UI and window
async function applyExpandedState() {
  const appWindow = getCurrentWindow();

  if (isExpanded) {
    mainModal.classList.add('expanded');
    expandBtn.classList.add('active');
    expandBtn.title = 'Collapse';
    // Update icon to collapse
    expandBtn.innerHTML = `
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <polyline points="4 14 10 14 10 20"></polyline>
        <polyline points="20 10 14 10 14 4"></polyline>
        <line x1="14" y1="10" x2="21" y2="3"></line>
        <line x1="3" y1="21" x2="10" y2="14"></line>
      </svg>
    `;
    await appWindow.setSize(new LogicalSize(EXPANDED_SIZE.width, EXPANDED_SIZE.height));
  } else {
    mainModal.classList.remove('expanded');
    expandBtn.classList.remove('active');
    expandBtn.title = 'Expand';
    // Update icon to expand
    expandBtn.innerHTML = `
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <polyline points="15 3 21 3 21 9"></polyline>
        <polyline points="9 21 3 21 3 15"></polyline>
        <line x1="21" y1="3" x2="14" y2="10"></line>
        <line x1="3" y1="21" x2="10" y2="14"></line>
      </svg>
    `;
    await appWindow.setSize(new LogicalSize(COMPACT_SIZE.width, COMPACT_SIZE.height));
  }
}

// Restore expanded state on app launch
async function restoreExpandedState() {
  if (isExpanded) {
    await applyExpandedState();
  }
}

// Add action to history
function addToActionHistory(action) {
  const formattedAction = formatAction(action);
  actionHistory.unshift(formattedAction);
  // Keep only last 10 actions
  if (actionHistory.length > 10) {
    actionHistory.pop();
  }
  totalActionsCount++;
  updateActionHistoryUI();
}

// Update action history UI
function updateActionHistoryUI() {
  if (!actionHistoryList) return;

  actionHistoryList.innerHTML = actionHistory
    .map(action => `<div class="action-history-item">${action}</div>`)
    .join('');

  if (actionsCount) {
    actionsCount.textContent = totalActionsCount.toString();
  }
}

// Start elapsed timer
function startElapsedTimer() {
  sessionStartTime = Date.now();
  if (elapsedTimer) clearInterval(elapsedTimer);
  elapsedTimer = setInterval(updateElapsedTime, 1000);
  updateElapsedTime();
}

// Stop elapsed timer
function stopElapsedTimer() {
  if (elapsedTimer) {
    clearInterval(elapsedTimer);
    elapsedTimer = null;
  }
}

// Update elapsed time display
function updateElapsedTime() {
  if (!sessionStartTime || !elapsedValue) return;

  const elapsed = Math.floor((Date.now() - sessionStartTime) / 1000);
  const minutes = Math.floor(elapsed / 60);
  const seconds = elapsed % 60;
  elapsedValue.textContent = `${minutes}:${seconds.toString().padStart(2, '0')}`;
}

// Reset session stats
function resetSessionStats() {
  actionHistory = [];
  totalActionsCount = 0;
  sessionStartTime = null;
  lastRenderedTimelineCount = 0;
  stopElapsedTimer();
  updateActionHistoryUI();
  if (elapsedValue) elapsedValue.textContent = '0:00';
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
  settingsBtn.setAttribute('aria-expanded', 'true');
  // Focus the first focusable element in settings
  providerSelect.focus();
}

// Close settings panel
function closeSettings() {
  settingsPanel.classList.add('hidden');
  mainModal.classList.remove('hidden');
  settingsBtn.setAttribute('aria-expanded', 'false');
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

// Debounced renderQueue for rapid Tauri event updates
function debouncedRenderQueue() {
  if (renderQueueTimer) clearTimeout(renderQueueTimer);
  renderQueueTimer = setTimeout(() => {
    renderQueueTimer = null;
    renderQueue();
  }, 50);
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
          <button class="queue-item-remove" data-queue-id="${item.id}" title="Remove">
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
    debouncedRenderQueue();
  }
}

// Event delegation for queue item remove buttons
queueList.addEventListener('click', (e) => {
  const removeBtn = e.target.closest('.queue-item-remove');
  if (removeBtn) {
    const id = removeBtn.dataset.queueId;
    if (id) removeFromQueue(id);
  }
});

// Window size persistence
const WINDOW_SIZE_KEY = 'pia-window-size';

async function restoreWindowSize() {
  try {
    const saved = localStorage.getItem(WINDOW_SIZE_KEY);
    if (saved) {
      const { width, height } = JSON.parse(saved);
      const appWindow = getCurrentWindow();
      await appWindow.setSize(new LogicalSize(Math.round(width), Math.round(height)));
    }
  } catch (error) {
    console.error('Failed to restore window size:', error);
  }
}

function saveWindowSize(width, height) {
  try {
    localStorage.setItem(WINDOW_SIZE_KEY, JSON.stringify({ width, height }));
  } catch (error) {
    console.error('Failed to save window size:', error);
  }
}

function setupResizeListener() {
  const appWindow = getCurrentWindow();
  appWindow.onResized(({ payload: size }) => {
    // Debounce to avoid excessive saves during drag
    clearTimeout(resizeDebounceTimer);
    resizeDebounceTimer = setTimeout(() => {
      saveWindowSize(size.width, size.height);
    }, 300);
  });
}

// Apply size preset
async function applySizePreset(presetName) {
  const preset = SIZE_PRESETS[presetName];
  if (!preset) return;

  currentSizePreset = presetName;

  // Update window size via Tauri
  try {
    const appWindow = getCurrentWindow();
    await appWindow.setSize(new LogicalSize(preset.width, preset.height));
  } catch (error) {
    console.error('Failed to resize window:', error);
  }

  // Update CSS class on modal
  Object.values(SIZE_PRESETS).forEach(p => {
    mainModal.classList.remove(p.cssClass);
  });
  mainModal.classList.add(preset.cssClass);

  // Update button states
  sizeMiniBtn.classList.toggle('active', presetName === 'mini');
  sizeStandardBtn.classList.toggle('active', presetName === 'standard');
  sizeDetailedBtn.classList.toggle('active', presetName === 'detailed');

  // Persist preference
  localStorage.setItem('pia-size-preset', presetName);
}

// Load saved size preset
async function loadSavedSizePreset() {
  const saved = localStorage.getItem('pia-size-preset');
  if (saved && SIZE_PRESETS[saved]) {
    await applySizePreset(saved);
  } else {
    // Apply default standard preset
    mainModal.classList.add(SIZE_PRESETS.standard.cssClass);
  }
}

// Setup size selector event listeners
function setupSizeSelector() {
  sizeMiniBtn.addEventListener('click', () => applySizePreset('mini'));
  sizeStandardBtn.addEventListener('click', () => applySizePreset('standard'));
  sizeDetailedBtn.addEventListener('click', () => applySizePreset('detailed'));
}

// Setup position menu
function setupPositionMenu() {
  // Toggle dropdown
  positionBtn.addEventListener('click', (e) => {
    e.stopPropagation();
    positionDropdown.classList.toggle('visible');
    positionDropdown.classList.remove('hidden');
  });

  // Close dropdown when clicking outside
  document.addEventListener('click', (e) => {
    if (!positionDropdown.contains(e.target) && e.target !== positionBtn) {
      positionDropdown.classList.remove('visible');
    }
  });

  // Position option clicks
  positionOptions.forEach(option => {
    option.addEventListener('click', async () => {
      const position = option.dataset.position;
      await snapToPosition(position);
      positionDropdown.classList.remove('visible');
    });
  });

  // Update active state based on saved position
  updatePositionActiveState();
}

// Update active state in dropdown
function updatePositionActiveState() {
  positionOptions.forEach(option => {
    option.classList.toggle('active', option.dataset.position === currentPosition);
  });
}

// Calculate snap positions
async function calculateSnapPositions() {
  const appWindow = getCurrentWindow();
  const monitor = await currentMonitor();

  if (!monitor) {
    console.error('No monitor found');
    return null;
  }

  const windowSize = await appWindow.innerSize();
  const screenSize = monitor.size;
  const screenPosition = monitor.position;

  const windowWidth = windowSize.width;
  const windowHeight = windowSize.height;
  const screenWidth = screenSize.width;
  const screenHeight = screenSize.height;
  const screenX = screenPosition.x;
  const screenY = screenPosition.y;

  return {
    'top-left': {
      x: screenX + POSITION_PADDING,
      y: screenY + POSITION_PADDING
    },
    'top-right': {
      x: screenX + screenWidth - windowWidth - POSITION_PADDING,
      y: screenY + POSITION_PADDING
    },
    'bottom-left': {
      x: screenX + POSITION_PADDING,
      y: screenY + screenHeight - windowHeight - POSITION_PADDING
    },
    'bottom-right': {
      x: screenX + screenWidth - windowWidth - POSITION_PADDING,
      y: screenY + screenHeight - windowHeight - POSITION_PADDING
    },
    'center': {
      x: screenX + Math.floor((screenWidth - windowWidth) / 2),
      y: screenY + Math.floor((screenHeight - windowHeight) / 2)
    }
  };
}

// Snap window to position
async function snapToPosition(position) {
  try {
    const positions = await calculateSnapPositions();
    if (!positions || !positions[position]) {
      console.error('Invalid position:', position);
      return;
    }

    const appWindow = getCurrentWindow();
    const { x, y } = positions[position];

    await appWindow.setPosition(new PhysicalPosition(x, y));

    // Save position preference
    currentPosition = position;
    localStorage.setItem('pia-window-position', position);
    updatePositionActiveState();
  } catch (error) {
    console.error('Failed to snap to position:', error);
  }
}

// Restore saved position on startup
async function restoreSavedPosition() {
  if (currentPosition) {
    // Small delay to ensure window is ready
    setTimeout(async () => {
      await snapToPosition(currentPosition);
    }, 100);
  }
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
        settingsPanel.classList.remove('swipe-closing');
        settingsPanel.style.transform = '';
        settingsPanel.style.opacity = '';
        closeSettings();
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
  hint.style.position = 'fixed';
  hint.style.zIndex = '99999';
  hint.style.whiteSpace = 'nowrap';

  document.body.appendChild(hint);

  // Get dimensions after appending to calculate size
  const rect = element.getBoundingClientRect();
  const hintRect = hint.getBoundingClientRect();

  // Calculate center position
  let left = rect.left + rect.width / 2;
  let top = rect.top - 35;

  // Adjust horizontal position if it would overflow
  const hintWidth = hintRect.width;
  if (left - hintWidth / 2 < 5) {
    left = hintWidth / 2 + 5;
  } else if (left + hintWidth / 2 > window.innerWidth - 5) {
    left = window.innerWidth - hintWidth / 2 - 5;
  }

  // Adjust vertical position if it would overflow
  if (top < 5) {
    top = rect.bottom + 5; // Show below instead
  }

  hint.style.left = `${left}px`;
  hint.style.top = `${top}px`;
  hint.style.transform = 'translateX(-50%)';

  setTimeout(() => {
    hint.remove();
  }, 1000);
}

// Initialize the app
init();
