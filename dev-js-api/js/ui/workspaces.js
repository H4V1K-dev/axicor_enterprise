/**
 * @fileoverview workspaces.js — Workspace Layout switcher and manager.
 * Handles switching between the five core AxiCAD workspaces.
 */

import { store } from '../store/store.js';
import { emit, on, EVENTS } from '../store/event_bus.js';
import { deselectAll } from '../editor.js';

export function initWorkspaces() {
  const tabsContainer = document.getElementById('workspace-tabs');
  const tabs = document.querySelectorAll('.workspace-tab');
  
  // Playback panels
  const growthPanel = document.getElementById('growth-playback-panel');
  const inferencePanel = document.getElementById('inference-playback-panel');
  
  // Sidebar tool buttons
  const toolInspect = document.getElementById('tool-inspect');
  const toolSelect = document.getElementById('tool-select');
  const toolTranslate = document.getElementById('tool-translate');
  const toolResize = document.getElementById('tool-resize');
  const toolAddSocket = document.getElementById('tool-add-socket-btn');
  const toolAddRoute = document.getElementById('tool-add-route-btn');
  const toolAddShard = document.getElementById('add-shard-btn');

  // Bottom toolbar panels & docks
  const modeSwitchPanel = document.getElementById('mode-switch-panel');
  const physicsToggle = document.getElementById('physics-toggle-btn');
  const layersToggle = document.getElementById('layers-toggle-btn');
  const validatorToggle = document.getElementById('validator-toggle-btn');

  if (!tabsContainer || tabs.length === 0) {
    console.warn('Workspace tabs markup not found.');
    return;
  }

  // Define allowed tools for each workspace
  const workspaceTools = {
    'model-composition': [toolInspect, toolSelect, toolTranslate, toolResize, toolAddShard],
    'neuron-lab': [toolInspect, toolSelect],
    'connectom-editor': [toolInspect, toolSelect, toolTranslate, toolResize, toolAddSocket, toolAddRoute],
    'growth-simulator': [toolInspect],
    'inference-mode': [toolInspect]
  };

  // Define allowed bottom panels for each workspace
  const workspaceBottomPanels = {
    'model-composition': { modeSwitch: false, physics: true, layers: true, validator: true },
    'neuron-lab': { modeSwitch: false, physics: true, layers: false, validator: false },
    'connectom-editor': { modeSwitch: true, physics: false, layers: true, validator: true },
    'growth-simulator': { modeSwitch: false, physics: false, layers: false, validator: false },
    'inference-mode': { modeSwitch: false, physics: false, layers: false, validator: false }
  };

  // Switch workspace function
  function switchWorkspace(workspaceName) {
    // Update active class on tabs
    tabs.forEach(tab => {
      if (tab.dataset.workspace === workspaceName) {
        tab.classList.add('active');
      } else {
        tab.classList.remove('active');
      }
    });

    // Update store
    store.set('activeWorkspace', workspaceName);

    // Apply UI state changes
    applyWorkspaceUI(workspaceName);

    // Emit event
    emit(EVENTS.WORKSPACE_CHANGED, { workspace: workspaceName });
  }

  // Update layout UI elements based on active workspace
  function applyWorkspaceUI(workspaceName) {
    // 1. Force deselect all to avoid weird editing states across workspaces
    deselectAll();

    // 2. Hide/show tool buttons in tools-sidebar
    const allowedTools = workspaceTools[workspaceName] || [];
    const allTools = [toolInspect, toolSelect, toolTranslate, toolResize, toolAddSocket, toolAddRoute, toolAddShard];
    
    allTools.forEach(tool => {
      if (!tool) return;
      if (allowedTools.includes(tool)) {
        tool.style.display = '';
      } else {
        tool.style.display = 'none';
      }
    });

    // 3. Fallback activeMode if currently selected mode is not allowed in new workspace
    // Safely check modeManager and activeMode
    import('../editor.js').then(({ modeManager }) => {
      if (modeManager && modeManager.activeModeName) {
        const allowedModeNames = [];
        if (allowedTools.includes(toolInspect)) allowedModeNames.push('inspect');
        if (allowedTools.includes(toolSelect)) allowedModeNames.push('select');
        if (allowedTools.includes(toolTranslate)) allowedModeNames.push('translate');
        if (allowedTools.includes(toolResize)) allowedModeNames.push('resize');
        if (allowedTools.includes(toolAddSocket)) allowedModeNames.push('add_socket');
        if (allowedTools.includes(toolAddRoute)) allowedModeNames.push('add_route');
        if (allowedTools.includes(toolAddShard)) allowedModeNames.push('add_shard');

        if (!allowedModeNames.includes(modeManager.activeModeName)) {
          modeManager.setMode('inspect');
        }
      }
    });

    // 4. Show/hide Playback Panels
    if (workspaceName === 'growth-simulator') {
      if (growthPanel) growthPanel.style.display = 'flex';
      if (inferencePanel) inferencePanel.style.display = 'none';
    } else if (workspaceName === 'inference-mode') {
      if (growthPanel) growthPanel.style.display = 'none';
      if (inferencePanel) inferencePanel.style.display = 'flex';
    } else {
      if (growthPanel) growthPanel.style.display = 'none';
      if (inferencePanel) inferencePanel.style.display = 'none';
    }

    // 5. Hide/show bottom panel docks
    const panelsConfig = workspaceBottomPanels[workspaceName] || {};
    if (modeSwitchPanel) modeSwitchPanel.style.display = panelsConfig.modeSwitch ? '' : 'none';
    if (physicsToggle) physicsToggle.style.display = panelsConfig.physics ? '' : 'none';
    if (layersToggle) layersToggle.style.display = panelsConfig.layers ? '' : 'none';
    if (validatorToggle) validatorToggle.style.display = panelsConfig.validator ? '' : 'none';

    // 6. Close bottom drawers if they shouldn't be visible
    if (!panelsConfig.physics) {
      const drawer = document.getElementById('physics-drawer');
      if (drawer && drawer.classList.contains('open')) {
        drawer.classList.remove('open');
        if (physicsToggle) physicsToggle.classList.remove('active');
      }
    }
    if (!panelsConfig.layers) {
      const drawer = document.getElementById('layers-drawer');
      if (drawer && drawer.classList.contains('open')) {
        drawer.classList.remove('open');
        if (layersToggle) layersToggle.classList.remove('active');
      }
    }
    if (!panelsConfig.validator) {
      const drawer = document.getElementById('validator-drawer');
      if (drawer && drawer.classList.contains('open')) {
        drawer.classList.remove('open');
        if (validatorToggle) validatorToggle.classList.remove('active');
      }
    }
  }

  // Hook up tab click listeners
  tabs.forEach(tab => {
    tab.addEventListener('click', () => {
      const ws = tab.dataset.workspace;
      switchWorkspace(ws);
    });
  });

  // Watch store project loading to display the workspace tabs bar
  store.on('projectName', (name) => {
    if (name) {
      tabsContainer.style.display = 'flex';
      // Default back to first tab on new project load
      switchWorkspace('model-composition');
    } else {
      tabsContainer.style.display = 'none';
    }
  });

  // Timeline / Playback actions placeholders
  setupPlaybackListeners();
}

function setupPlaybackListeners() {
  // Growth playback selectors
  const gPlayBtn = document.getElementById('growth-play-btn');
  const gStepBtn = document.getElementById('growth-step-btn');
  const gResetBtn = document.getElementById('growth-reset-btn');
  const gLabel = document.getElementById('growth-status-label');

  let growthTimer = null;
  let growthStep = 0;

  if (gPlayBtn) {
    gPlayBtn.onclick = () => {
      if (growthTimer) {
        // Pause
        clearInterval(growthTimer);
        growthTimer = null;
        gPlayBtn.innerHTML = `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="currentColor" stroke="none"><polygon points="6 3 20 12 6 21 6 3"/></svg>`;
        gPlayBtn.classList.remove('active');
      } else {
        // Play
        gPlayBtn.classList.add('active');
        gPlayBtn.innerHTML = `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="currentColor" stroke="none"><rect x="14" y="4" width="4" height="16" rx="1"/><rect x="6" y="4" width="4" height="16" rx="1"/></svg>`;
        growthTimer = setInterval(() => {
          growthStep++;
          if (growthStep > 100) growthStep = 0;
          if (gLabel) gLabel.textContent = `Рост: Шаг ${growthStep} / 100`;
        }, 150);
      }
    };
  }

  if (gStepBtn) {
    gStepBtn.onclick = () => {
      growthStep++;
      if (growthStep > 100) growthStep = 0;
      if (gLabel) gLabel.textContent = `Рост: Шаг ${growthStep} / 100`;
    };
  }

  if (gResetBtn) {
    gResetBtn.onclick = () => {
      if (growthTimer) {
        clearInterval(growthTimer);
        growthTimer = null;
        if (gPlayBtn) {
          gPlayBtn.innerHTML = `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="currentColor" stroke="none"><polygon points="6 3 20 12 6 21 6 3"/></svg>`;
          gPlayBtn.classList.remove('active');
        }
      }
      growthStep = 0;
      if (gLabel) gLabel.textContent = `Рост: Шаг ${growthStep} / 100`;
    };
  }

  // Inference playback selectors
  const iPlayBtn = document.getElementById('inference-play-btn');
  const iStepBtn = document.getElementById('inference-step-btn');
  const iResetBtn = document.getElementById('inference-reset-btn');
  const iLabel = document.getElementById('inference-status-label');

  let infTimer = null;
  let infTick = 0;

  if (iPlayBtn) {
    iPlayBtn.onclick = () => {
      if (infTimer) {
        clearInterval(infTimer);
        infTimer = null;
        iPlayBtn.innerHTML = `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="currentColor" stroke="none"><polygon points="6 3 20 12 6 21 6 3"/></svg>`;
        iPlayBtn.classList.remove('active');
      } else {
        iPlayBtn.classList.add('active');
        iPlayBtn.innerHTML = `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="currentColor" stroke="none"><rect x="14" y="4" width="4" height="16" rx="1"/><rect x="6" y="4" width="4" height="16" rx="1"/></svg>`;
        infTimer = setInterval(() => {
          infTick++;
          if (iLabel) iLabel.textContent = `Инференс: Такт ${infTick}`;
        }, 100);
      }
    };
  }

  if (iStepBtn) {
    iStepBtn.onclick = () => {
      infTick++;
      if (iLabel) iLabel.textContent = `Инференс: Такт ${infTick}`;
    };
  }

  if (iResetBtn) {
    iResetBtn.onclick = () => {
      if (infTimer) {
        clearInterval(infTimer);
        infTimer = null;
        if (iPlayBtn) {
          iPlayBtn.innerHTML = `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="currentColor" stroke="none"><polygon points="6 3 20 12 6 21 6 3"/></svg>`;
          iPlayBtn.classList.remove('active');
        }
      }
      infTick = 0;
      if (iLabel) iLabel.textContent = `Инференс: Такт ${infTick}`;
    };
  }
}
