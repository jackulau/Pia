import { listen } from '@tauri-apps/api/event';

const cursorIndicator = document.getElementById('cursor-indicator');

async function init() {
  await listen('show-cursor-indicator', (event) => {
    const { x, y, action_type } = event.payload;
    showIndicator(x, y, action_type);
  });

  await listen('hide-cursor-indicator', () => {
    hideIndicator();
  });

  await listen('update-cursor-position', (event) => {
    const { x, y } = event.payload;
    updatePosition(x, y);
  });
}

function showIndicator(x, y, actionType) {
  cursorIndicator.style.left = `${x}px`;
  cursorIndicator.style.top = `${y}px`;

  cursorIndicator.classList.remove('hidden', 'hiding', 'click', 'double_click', 'move', 'scroll');
  cursorIndicator.classList.add('showing', actionType || 'click');

  setTimeout(() => {
    cursorIndicator.classList.remove('showing');
  }, 200);
}

function hideIndicator() {
  cursorIndicator.classList.add('hiding');

  setTimeout(() => {
    cursorIndicator.classList.add('hidden');
    cursorIndicator.classList.remove('hiding');
  }, 150);
}

function updatePosition(x, y) {
  cursorIndicator.style.left = `${x}px`;
  cursorIndicator.style.top = `${y}px`;
}

init();
