/**
 * @fileoverview ui.js — Main UI entry point coordinating subcomponents (sidebar, toolbar, physics drawer, validator panel, toast notifications).
 */

import { on, emit, EVENTS } from './store/event_bus.js';
import { showSidebar, hideSidebar, saveAllLayoutChanges, renderLayersListItems } from './ui/sidebar.js';
import { initToolbar } from './ui/toolbar.js';
import { initPhysicsPanel } from './ui/physics_panel.js';
import { initLayersPanel } from './ui/layers_panel.js';
import { initValidatorPanel } from './ui/validator_panel.js';
import { initHistoryPanel } from './ui/history_panel.js';
import { initViewCube } from './ui/viewcube.js';
import { showToast } from './ui/toast.js';
import { store } from './store/store.js';
import { modeManager } from './editor.js';
import { initWorkspaces } from './ui/workspaces.js';

export { showToast, showSidebar, hideSidebar, saveAllLayoutChanges };

const SVG_FOLDEROPEN_FLAT = `<svg xmlns="http://www.w3.org/2000/svg" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="margin-right:6px; vertical-align: middle;"><path d="m6 14 1.45-2.9A2 2 0 0 1 9.24 10H20a2 2 0 0 1 1.94 2.5l-1.55 6a2 2 0 0 1-1.94 1.5H4a2 2 0 0 1-2-2V5c0-1.1.9-2 2-2h3.93a2 2 0 0 1 1.66.9l.82 1.2a2 2 0 0 0 1.66.9H18a2 2 0 0 1 2 2v2"/></svg>`;

/**
 * Initializes all modular UI components and sets up global event bus listeners.
 */
export function initUI() {
  // Setup top-left CAD panel and model name
  const topLeftContainer = document.getElementById('top-left-container');
  if (topLeftContainer) {
    topLeftContainer.style.display = 'flex';
    const modelNameSpan = document.getElementById('cad-model-name');
    if (modelNameSpan) {
      modelNameSpan.textContent = (store.get('projectName') || 'octopus').toUpperCase();
    }
  }

  // Populate Flat Folder Open Icon in EXPLORER button
  const hubBtn = document.getElementById('summon-hub-flat-btn');
  if (hubBtn) {
    hubBtn.innerHTML = `${SVG_FOLDEROPEN_FLAT}<span class="explorer-text">EXPLORER</span>`;
  }

  // Statistics overlay trigger positioning and mouseenter/mouseleave auto-dismiss
  const statsTrigger = document.getElementById('stats-trigger-btn');
  const statsOverlay = document.getElementById('stats-overlay');
  if (statsTrigger && statsOverlay) {
    const updateStatsPosition = () => {
      const btnRect = statsTrigger.getBoundingClientRect();
      const overlayWidth = 280;
      const targetLeft = btnRect.left + btnRect.width / 2 - overlayWidth / 2;
      const safeLeft = Math.max(16, Math.min(window.innerWidth - overlayWidth - 16, targetLeft));
      statsOverlay.style.left = safeLeft + 'px';
    };

    let statsCloseTimeout = null;

    const openStats = () => {
      if (statsCloseTimeout) clearTimeout(statsCloseTimeout);
      updateStatsPosition();
      statsOverlay.classList.add('open');
      statsTrigger.classList.add('active');
    };

    const closeStats = () => {
      if (statsCloseTimeout) clearTimeout(statsCloseTimeout);
      statsCloseTimeout = setTimeout(() => {
        statsOverlay.classList.remove('open');
        statsTrigger.classList.remove('active');
      }, 200);
    };

    // Toggle stats only on click
    statsTrigger.addEventListener('click', (e) => {
      e.stopPropagation();
      if (statsOverlay.classList.contains('open')) {
        closeStats();
      } else {
        openStats();
      }
    });

    statsTrigger.addEventListener('mouseleave', closeStats);

    statsOverlay.addEventListener('mouseenter', () => {
      if (statsCloseTimeout) clearTimeout(statsCloseTimeout);
    });
    statsOverlay.addEventListener('mouseleave', closeStats);
  }



  // Folding tools sidebar logic
  const toolsSidebar = document.getElementById('tools-sidebar');
  const toolsToggle = document.getElementById('tools-toggle-btn');
  if (toolsSidebar && toolsToggle) {
    toolsToggle.addEventListener('click', (e) => {
      if (toolsSidebar.classList.contains('morphed')) {
        e.stopPropagation();
        e.preventDefault();
        modeManager.popMode();
      } else {
        toolsSidebar.classList.toggle('closed');
      }
    });
  }
 
  // Interactive Tools selection logic
  const toolInspectBtn = document.getElementById('tool-inspect');
  const toolSelectBtn = document.getElementById('tool-select');
  const toolTranslateBtn = document.getElementById('tool-translate');
  const toolResizeBtn = document.getElementById('tool-resize');
  const toolAddSocketBtn = document.getElementById('tool-add-socket-btn');
  const toolAddRouteBtn = document.getElementById('tool-add-route-btn');
  const addShardBtn = document.getElementById('add-shard-btn');

  const toolButtons = {
    'inspect': toolInspectBtn,
    'select': toolSelectBtn,
    'translate': toolTranslateBtn,
    'resize': toolResizeBtn,
    'add_socket': toolAddSocketBtn,
    'add_route': toolAddRouteBtn,
    'add_shard': addShardBtn
  };

  const triggerMode = (modeName) => {
    if (modeManager.activeModeName === modeName) {
      const modeLabels = {
        'inspect': 'Режим осмотра активен',
        'select': 'Режим выделения активен',
        'translate': 'Режим перемещения (Gizmo) активен',
        'resize': 'Режим изменения размеров активен',
        'add_socket': 'Режим добавления сокета',
        'add_route': 'Режим создания связи',
        'add_shard': 'Режим добавления шарда'
      };
      if (modeLabels[modeName]) {
        showToast(modeLabels[modeName], 'success', 3000);
      }
    } else {
      modeManager.setMode(modeName);
    }
  };

  if (toolInspectBtn) {
    toolInspectBtn.addEventListener('click', () => triggerMode('inspect'));
  }
  if (toolSelectBtn) {
    toolSelectBtn.addEventListener('click', () => triggerMode('select'));
  }
  if (toolTranslateBtn) {
    toolTranslateBtn.addEventListener('click', () => triggerMode('translate'));
  }
  if (toolResizeBtn) {
    toolResizeBtn.addEventListener('click', () => triggerMode('resize'));
  }
  if (toolAddSocketBtn) {
    toolAddSocketBtn.addEventListener('click', () => triggerMode('add_socket'));
  }
  if (toolAddRouteBtn) {
    toolAddRouteBtn.addEventListener('click', () => triggerMode('add_route'));
  }
  if (addShardBtn) {
    addShardBtn.addEventListener('click', () => triggerMode('add_shard'));
  }

  const updateButtonsActiveState = (activeMode) => {
    Object.entries(toolButtons).forEach(([modeName, btn]) => {
      if (btn) {
        if (modeName === activeMode) {
          btn.classList.add('active');
        } else {
          btn.classList.remove('active');
        }
      }
    });
  };

  // Subscribe to mode change events to sync buttons active state
  on(EVENTS.MODE_CHANGED, (payload) => {
    updateButtonsActiveState(payload.mode);

    const modeLabels = {
      'inspect': 'Режим осмотра активен',
      'select': 'Режим выделения активен',
      'translate': 'Режим перемещения (Gizmo) активен',
      'resize': 'Режим изменения размеров активен',
      'add_socket': 'Режим добавления сокета',
      'add_route': 'Режим создания связи',
      'add_shard': 'Режим добавления шарда'
    };
    
    if (modeLabels[payload.mode]) {
      const isTemporary = (modeManager && modeManager.ctrlHeld);
      if (isTemporary) {
        let label = modeLabels[payload.mode];
        if (modeManager.ctrlHeld) {
          label = 'Временное выделение (Ctrl)';
        }
        showToast(label, 'success', null);
      } else {
        showToast(modeLabels[payload.mode], 'success', 3000);
      }
    }
  });

  // Sync initial state if modeManager is already initialized
  if (modeManager && modeManager.activeModeName) {
    updateButtonsActiveState(modeManager.activeModeName);
  }

  // Folding inspector sidebar logic
  const sidebar = document.getElementById('sidebar');
  const sidebarToggle = document.getElementById('sidebar-toggle-btn');
  if (sidebar && sidebarToggle) {
    sidebarToggle.addEventListener('click', () => {
      sidebar.classList.toggle('closed');
      sidebarToggle.textContent = sidebar.classList.contains('closed') ? '◀' : '▶';
    });
  }

  // Subscribe to Event Bus selection changes
  on(EVENTS.SELECTION_CHANGED, (payload) => {
    if (payload.type) {
      // Auto expand sidebar if closed upon selecting an object
      if (sidebar && sidebar.classList.contains('closed')) {
        sidebar.classList.remove('closed');
        if (sidebarToggle) sidebarToggle.textContent = '▶';
      }
      showSidebar(payload.type, payload.data);
    } else {
      hideSidebar();
    }
  });

  // Re-render layers list in inspector when layer boundaries / order changes
  on(EVENTS.LAYERS_CHANGED, (sd) => {
    const container = document.getElementById('layers-list-container');
    if (container) {
      container.innerHTML = renderLayersListItems(sd);
    }
  });

  // Set hasUnsavedChanges flag on physical modifications (no auto-saving)
  on(EVENTS.LAYOUT_CHANGED, (sd) => {
    store.set('hasUnsavedChanges', true);
  });

  // Initialize Bottom Toolbar and get references to drawer toggle buttons
  const { physicsBtn, layersBtn, validatorBtn } = initToolbar();

  // Settings trigger logic (after toolbar creates it in DOM)
  const settingsTrigger = document.getElementById('settings-trigger-btn');
  if (settingsTrigger) {
    import('./ui/settings_panel.js').then(({ initSettingsPanel }) => {
      initSettingsPanel(settingsTrigger);
    });
  }

  // Initialize sub-panels
  initPhysicsPanel(physicsBtn);
  initLayersPanel(layersBtn);
  initValidatorPanel(validatorBtn);
  initHistoryPanel();
  initViewCube();
  initWorkspaces();
}
