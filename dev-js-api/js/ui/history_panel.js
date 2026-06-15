/**
 * @fileoverview history_panel.js — Selective state history panel, navigation, and sandboxed card previews.
 */

import { store } from '../store/store.js';
import { historyManager } from '../store/history_manager.js';
import { on, EVENTS } from '../store/event_bus.js';

let panelContainer = null;
let dropdownPanel = null;
let updateInterval = null;

/**
 * Helper to calculate human readable elapsed time (Ago hh:mm:ss).
 * @param {number} timestamp
 * @returns {string}
 */
function formatElapsedTime(timestamp) {
  const diffMs = Date.now() - timestamp;
  const diffSecs = Math.max(0, Math.floor(diffMs / 1000));
  const secs = diffSecs % 60;
  const mins = Math.floor(diffSecs / 60) % 60;
  const hours = Math.floor(diffSecs / 3600);

  const pad = (num) => String(num).padStart(2, '0');
  return `Ago ${pad(hours)}:${pad(mins)}:${pad(secs)}`;
}

/**
 * Updates only the elapsed time text on the cards periodically.
 */
function updateElapsedTimesOnly() {
  if (!dropdownPanel) return;
  const els = dropdownPanel.querySelectorAll('.elapsed-time-text');
  els.forEach(el => {
    const ts = parseInt(el.dataset.timestamp);
    if (!isNaN(ts)) {
      el.textContent = formatElapsedTime(ts);
    }
  });
}

/**
 * Initializes the history control panel widget and events.
 */
export function initHistoryPanel() {
  if (document.getElementById('history-panel-container')) return;

  // Create main control bar
  panelContainer = document.createElement('div');
  panelContainer.id = 'history-panel-container';
  panelContainer.className = 'ax-panel'; // Glassmorphic backing
  panelContainer.innerHTML = `
    <button id="history-undo-btn" class="ax-btn" title="Отменить последнее действие (Ctrl+Z)">←</button>
    <button id="history-toggle-btn" class="ax-btn">Глобальная история</button>
    <button id="history-redo-btn" class="ax-btn" title="Повторить отмененное действие (Ctrl+Y)">→</button>
  `;

  // Append history panel container to #top-left-container as the last element
  const topLeft = document.getElementById('top-left-container');
  if (topLeft) {
    topLeft.appendChild(panelContainer);
  } else {
    document.body.appendChild(panelContainer);
  }



  // Create horizontal scrollable action dropdown list inside panelContainer (for absolute positioning alignment)
  dropdownPanel = document.createElement('div');
  dropdownPanel.id = 'history-dropdown-panel';
  panelContainer.appendChild(dropdownPanel);

  // Wire horizontal mouse wheel scrolling
  dropdownPanel.addEventListener('wheel', (e) => {
    if (e.deltaY !== 0) {
      e.preventDefault();
      dropdownPanel.scrollLeft += e.deltaY;
    }
  });

  // Wire buttons events
  const undoBtn = document.getElementById('history-undo-btn');
  const redoBtn = document.getElementById('history-redo-btn');
  const toggleBtn = document.getElementById('history-toggle-btn');

  undoBtn.addEventListener('click', () => {
    if (historyManager.previewActive) {
      historyManager.resetPreview();
    }
    historyManager.undoGlobal();
    updateHistoryUI();
  });

  redoBtn.addEventListener('click', () => {
    if (historyManager.previewActive) {
      historyManager.resetPreview();
    }
    historyManager.redoGlobal();
    updateHistoryUI();
  });

  toggleBtn.addEventListener('click', (e) => {
    e.stopPropagation();
    if (historyManager.previewActive) {
      // Commit preview
      historyManager.applyPreview();
      closeDropdown();
      updateHistoryUI();
    } else {
      toggleDropdown();
    }
  });

  // Close dropdown on click outside
  window.addEventListener('pointerdown', (e) => {
    if (
      panelContainer.contains(e.target) ||
      dropdownPanel.contains(e.target) ||
      e.target.closest('#ax-confirm-modal') ||
      e.target.closest('#ax-settings-modal')
    ) {
      return;
    }
    if (historyManager.previewActive) {
      historyManager.resetPreview();
      updateHistoryUI();
    }
    closeDropdown();
  });

  // Listen to reactives updates
  store.on('historyUpdated', () => {
    updateHistoryUI();
  });

  // Auto update on focus/selection change
  on(EVENTS.SELECTION_CHANGED, () => {
    if (historyManager.previewActive) {
      historyManager.resetPreview();
    }
    updateHistoryUI();
  });

  // Escape key handler
  window.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
      if (historyManager.previewActive) {
        historyManager.resetPreview();
        updateHistoryUI();
      }
      closeDropdown();
    }
  });

  updateHistoryUI();
}

function toggleDropdown() {
  const isOpen = dropdownPanel.classList.contains('open');
  if (isOpen) {
    closeDropdown();
  } else {
    openDropdown();
  }
}

function openDropdown() {
  updateHistoryUI();
  dropdownPanel.classList.add('open');
  // Scroll to the end (newest actions are on the right)
  setTimeout(() => {
    dropdownPanel.scrollLeft = dropdownPanel.scrollWidth;
  }, 50);

  // Start elapsed times ticker
  if (updateInterval) clearInterval(updateInterval);
  updateInterval = setInterval(() => {
    if (dropdownPanel.classList.contains('open')) {
      updateElapsedTimesOnly();
    } else {
      clearInterval(updateInterval);
      updateInterval = null;
    }
  }, 1000);
}

function closeDropdown() {
  dropdownPanel.classList.remove('open');
  if (updateInterval) {
    clearInterval(updateInterval);
    updateInterval = null;
  }
}

/**
 * Re-renders the buttons and dropdown cards list.
 */
function updateHistoryUI() {
  if (!panelContainer || !dropdownPanel) return;

  const undoBtn = document.getElementById('history-undo-btn');
  const redoBtn = document.getElementById('history-redo-btn');
  const toggleBtn = document.getElementById('history-toggle-btn');

  const isFocused = historyManager.isFocusedHistory();
  const stack = historyManager.getActiveStack();

  // Enable/disable arrows
  if (isFocused) {
    // Local history has no linear undo/redo via arrows (those are global), but we keep them disabled/ghost
    undoBtn.disabled = true;
    redoBtn.disabled = true;
    undoBtn.style.opacity = '0.3';
    redoBtn.style.opacity = '0.3';
    undoBtn.style.cursor = 'not-allowed';
    undoBtn.style.cursor = 'not-allowed';
  } else {
    const canUndo = historyManager.globalIndex >= 0;
    const canRedo = historyManager.globalIndex < historyManager.globalStack.length - 1;

    undoBtn.disabled = !canUndo;
    redoBtn.disabled = !canRedo;
    undoBtn.style.opacity = canUndo ? '1' : '0.3';
    redoBtn.style.opacity = canRedo ? '1' : '0.3';
    undoBtn.style.cursor = canUndo ? 'pointer' : 'not-allowed';
    redoBtn.style.cursor = canRedo ? 'pointer' : 'not-allowed';
  }

  // Center button text and class
  if (historyManager.previewActive) {
    toggleBtn.textContent = 'Применить';
    toggleBtn.classList.add('preview-active');
  } else {
    toggleBtn.classList.remove('preview-active');
    if (isFocused) {
      const selShardKey = store.get('selectedShardKey');
      const label = selShardKey ? 'Шард' : 'Сокет';
      toggleBtn.textContent = `${label}: История`;
    } else {
      toggleBtn.textContent = 'Глобальная история';
    }
  }

  // Populate dropdown list
  if (stack.length === 0) {
    dropdownPanel.innerHTML = `
      <div style="color: var(--ax-text-faint); font-size: 11px; padding: 4px 12px; white-space: nowrap;">
        История изменений пуста
      </div>
    `;
    return;
  }

  // Render cards in natural chronological order (oldest on left, newest on right)
  dropdownPanel.innerHTML = stack.map((action, idx) => {
    let cardClass = 'history-card';
    const isPreviewed = historyManager.previewActive && historyManager.previewIndex === idx;

    // Outline for active state / preview
    if (isPreviewed) {
      cardClass += ' previewing';
    } else if (isFocused) {
      cardClass += ' active-state';
    } else {
      // Global stack active/undone coloring
      if (idx <= historyManager.globalIndex) {
        cardClass += ' active-state';
      } else {
        cardClass += ' undone-state';
      }
      
      // Outline current state
      if (idx === historyManager.globalIndex) {
        cardClass += ' current-state';
      }
    }

    // Determine type code & color modifier
    let abbr = 'MOV';
    let typeClass = 'history-card--blue';

    const t = action.type;
    if (t === 'delete' || t === 'delete_pair' || t === 'delete_with_connections') {
      abbr = 'DEL';
      typeClass = 'history-card--red';
    } else if (t === 'disconnect') {
      abbr = 'DIS';
      typeClass = 'history-card--yellow';
    } else if (t === 'resize') {
      abbr = 'RES';
      typeClass = 'history-card--green';
    } else if (t === 'create') {
      abbr = 'NEW';
      typeClass = 'history-card--blue';
    } else if (t === 'move') {
      abbr = 'MOV';
      typeClass = 'history-card--blue';
    }

    cardClass += ' ' + typeClass;

    return `
      <div class="${cardClass}" data-action-id="${action.id}" data-index="${idx}" title="${action.description}">
        <span class="history-card-abbr">${abbr}</span>
        <span class="history-card-time elapsed-time-text" data-timestamp="${action.timestamp}">
          ${formatElapsedTime(action.timestamp)}
        </span>
      </div>
    `;
  }).join('');

  // Bind click on cards for preview
  dropdownPanel.querySelectorAll('.history-card').forEach(card => {
    card.addEventListener('click', (e) => {
      e.stopPropagation();
      const actionId = card.dataset.actionId;
      const idx = parseInt(card.dataset.index);

      if (historyManager.previewActive && historyManager.previewIndex === idx) {
        // Clicked already previewed card -> reset
        historyManager.resetPreview();
        updateHistoryUI();
      } else {
        // Preview clicked card
        historyManager.previewState(actionId);
        updateHistoryUI();
      }
    });
  });
}
