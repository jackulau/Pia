import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getActionIcon } from './icons/action-icons.js';
import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';
// CSS is inlined in index.html for transparent window support

// DOM Elements
const mainModal = document.getElementById('main-modal');
const settingsPanel = document.getElementById('settings-panel');
const instructionInput = document.getElementById('instruction-input');
const submitBtn = document.getElementById('submit-btn');
const stopBtn = document.getElementById('stop-btn');
const exportBtn = document.getElementById('export-btn');
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
const showOverlay = document.getElementById('show-overlay');
const globalHotkeyInput = document.getElementById('global-hotkey-input');
const clearHotkeyBtn = document.getElementById('clear-hotkey-btn');
const hotkeyError = document.getElementById('hotkey-error');

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

// History elements
const historyBtn = document.getElementById('history-btn');
const historyDropdown = document.getElementById('history-dropdown');
const historyList = document.getElementById('history-list');
const historyClearBtn = document.getElementById('history-clear-btn');

// State
let isRunning = false;
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

// Window sizes
const COMPACT_SIZE = { width: 420, height: 280 };
const EXPANDED_SIZE = { width: 500, height: 450 };

// Initialize
async function init() {
  await loadConfig();
  await loadHistory();
  setupEventListeners();
  setupTauriListeners();
  setupKeyboardNavigation();
  await restoreExpandedState();

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

  // Set safety settings
  confirmDangerous.checked = currentConfig.general.confirm_dangerous_actions;

  // Set debug settings
  if (showOverlay) {
    showOverlay.checked = currentConfig.general.show_coordinate_overlay || false;
  }

  // Set global hotkey
  if (globalHotkeyInput) {
    globalHotkeyInput.value = currentConfig.general.global_hotkey || '';
  }
  if (hotkeyError) {
    hotkeyError.style.display = 'none';
  }

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

  // Retry info notification
  await listen('retry-info', (event) => {
    showToast(event.payload, 'info');
  });

  // Parse error notification
  await listen('parse-error', (event) => {
    showToast(event.payload, 'error');
  });

  // Instruction completed - save to history
  await listen('instruction-completed', async (event) => {
    const { instruction, success } = event.payload;
    try {
      await invoke('add_to_history', { instruction, success });
      await loadHistory();
    } catch (error) {
      console.error('Failed to save to history:', error);
    }
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
  const wasRunning = isRunning;
  isRunning = state.status === 'Running';

  // Start timer when agent starts running
  if (isRunning && !wasRunning) {
    startElapsedTimer();
  }

  // Stop timer when agent stops running
  if (!isRunning && wasRunning) {
    stopElapsedTimer();
  }

  // Update status indicator
  statusDot.className = 'status-dot';
  let statusLabel = 'Ready';
  switch (state.status) {
    case 'Running':
      statusDot.classList.add('running');
      statusLabel = 'Running';
      break;
    case 'Completed':
      statusDot.classList.add('completed');
      statusLabel = 'Completed';
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
  }
  statusText.textContent = statusLabel;

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

  speedValue.textContent = state.tokens_per_second > 0
    ? `${state.tokens_per_second.toFixed(1)} tok/s`
    : '-- tok/s';

  // Update action timeline
  if (state.action_history && state.action_history.length > 0) {
    if (actionCount) {
      actionCount.textContent = `${state.action_history.length} action${state.action_history.length === 1 ? '' : 's'}`;
    }
    if (actionsCount) {
      actionsCount.textContent = state.action_history.length;
    }

    // Clear and rebuild timeline
    if (timelineList) {
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

  // Update export button visibility
  updateHistoryCount();
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
    // Get max iterations with validation
    const maxIterInput = document.getElementById('max-iterations');
    let maxIterations = parseInt(maxIterInput.value, 10);
    if (isNaN(maxIterations) || maxIterations < 1) maxIterations = 1;
    if (maxIterations > 200) maxIterations = 200;
    maxIterInput.value = maxIterations;

    // Handle hotkey change first (if changed)
    if (hotkeyError) {
      hotkeyError.style.display = 'none';
    }

    let newHotkey = null;
    if (globalHotkeyInput) {
      newHotkey = globalHotkeyInput.value.trim() || null;
      const currentHotkey = currentConfig?.general?.global_hotkey || null;

      if (newHotkey !== currentHotkey) {
        if (newHotkey) {
          try {
            await invoke('set_global_hotkey', { shortcut: newHotkey });
          } catch (error) {
            if (hotkeyError) {
              hotkeyError.textContent = error;
              hotkeyError.style.display = 'block';
            }
            return;
          }
        } else {
          await invoke('unregister_global_hotkey');
        }
      }
    }

    // Build config object
    const config = {
      general: {
        default_provider: providerSelect.value,
        max_iterations: maxIterations,
        confirm_dangerous_actions: confirmDangerous.checked,
        show_coordinate_overlay: showOverlay ? showOverlay.checked : false,
        global_hotkey: newHotkey,
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

// Initialize the app
init();
