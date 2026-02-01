import { listen } from '@tauri-apps/api/event';

const container = document.getElementById('overlay-container');

// Listen for action indicator events
listen('show-action-indicator', (event) => {
  const { action, x, y, label, text, key, modifiers, direction } = event.payload;

  // Clear old indicators for most actions
  if (action !== 'type') {
    clearIndicators();
  }

  switch (action) {
    case 'click':
    case 'left':
      showClickIndicator(x, y, label, 'click-indicator');
      break;

    case 'right':
      showClickIndicator(x, y, label, 'click-indicator right-click');
      break;

    case 'double_click':
      showClickIndicator(x, y, label, 'click-indicator double-click');
      break;

    case 'move':
      // Optionally show a subtle move indicator
      showClickIndicator(x, y, label, 'click-indicator');
      break;

    case 'scroll':
      showClickIndicator(x, y, `scroll ${direction}`, 'click-indicator scroll');
      break;

    case 'type':
      showTypeIndicator(text);
      break;

    case 'key':
      showKeyIndicator(key, modifiers);
      break;
  }
});

function showClickIndicator(x, y, label, className) {
  // Create click indicator circle
  const indicator = document.createElement('div');
  indicator.className = className;
  indicator.style.left = `${x}px`;
  indicator.style.top = `${y}px`;
  container.appendChild(indicator);

  // Create label below indicator
  if (label) {
    const labelEl = document.createElement('div');
    labelEl.className = 'action-label';
    labelEl.textContent = label;
    labelEl.style.left = `${x}px`;
    labelEl.style.top = `${y}px`;
    container.appendChild(labelEl);
  }

  // Auto-remove after animation completes
  setTimeout(() => {
    indicator.remove();
    if (label) {
      const labels = container.querySelectorAll('.action-label');
      labels.forEach(l => l.remove());
    }
  }, 700);
}

function showTypeIndicator(text) {
  // Remove existing type indicator
  const existing = container.querySelector('.type-indicator');
  if (existing) {
    existing.remove();
  }

  const indicator = document.createElement('div');
  indicator.className = 'type-indicator';

  // Truncate long text
  const displayText = text.length > 50 ? text.substring(0, 50) + '...' : text;
  indicator.textContent = `Typing: "${displayText}"`;
  container.appendChild(indicator);

  // Auto-remove after a delay
  setTimeout(() => {
    indicator.remove();
  }, 2000);
}

function showKeyIndicator(key, modifiers) {
  // Remove existing key indicator
  const existing = container.querySelector('.key-indicator');
  if (existing) {
    existing.remove();
  }

  const indicator = document.createElement('div');
  indicator.className = 'key-indicator';

  let keyDisplay = '';
  if (modifiers && modifiers.length > 0) {
    keyDisplay = modifiers.map(m => m.charAt(0).toUpperCase() + m.slice(1)).join('+') + '+';
  }
  keyDisplay += key.toUpperCase();

  indicator.textContent = keyDisplay;
  container.appendChild(indicator);

  // Auto-remove after a delay
  setTimeout(() => {
    indicator.remove();
  }, 1500);
}

function clearIndicators() {
  // Clear click indicators and labels
  const clickIndicators = container.querySelectorAll('.click-indicator');
  const labels = container.querySelectorAll('.action-label');
  clickIndicators.forEach(el => el.remove());
  labels.forEach(el => el.remove());
}
