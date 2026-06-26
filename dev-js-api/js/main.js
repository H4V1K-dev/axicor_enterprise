import * as THREE from 'three';
import { initViewer, animateViewer, scene, getActiveCamera, renderer, controls } from './viewer.js';
import { buildSceneData, drawRoutes, shardMeshes, socketMeshes } from './scene_builder.js';
import { initEditor, deselectAll, transformControls, updateHandlesScale, modeManager } from './editor.js';
import { initUI } from './ui.js';
import { store } from './store/store.js';
import { on, emit, EVENTS } from './store/event_bus.js';
import { showProjectSelector } from './ui/project_hub.js';
import { historyManager } from './store/history_manager.js';
import { resolveRaycastHit } from './editor/collision_manager.js';
import { toThreeCoords } from './editor/coordinate_adapter.js';
import { api } from './services/api.js';

async function loadData() {
  const projectName = store.get('projectName') || 'octopus';
  const [placement, routes] = await Promise.all([
    api.loadPlacement(projectName),
    api.loadRoutes(projectName)
  ]);
  return { placement, routes };
}

function updateHUD(data) {
  const legendsDiv = document.getElementById('orbit-legends');
  legendsDiv.innerHTML = ''; // Legends cleared since orbits/levels are disabled

  document.getElementById('stat-depts').textContent = data.departments.length;
  document.getElementById('stat-shards').textContent = data.shards.length;
  document.getElementById('stat-conns').textContent = data.connections.length;

  const hud = document.getElementById('top-left-container');
  if (hud) hud.style.display = 'flex';
}

// Hover state variables
const raycaster = new THREE.Raycaster();
const mouse = new THREE.Vector2();
const tooltip = document.getElementById('tooltip');

function setupHoverTooltip() {
  window.addEventListener('mousemove', (e) => {
    const editorSettings = store.get('editorSettings') || {};
    if (store.get('modalActive') || editorSettings.show_tooltips === false) {
      tooltip.style.display = 'none';
      return;
    }
    // Ignore hover when interacting with UI outside the WebGL canvas
    const canvasContainer = document.getElementById('canvas-container');
    if (canvasContainer && !canvasContainer.contains(e.target)) {
      tooltip.style.display = 'none';
      return;
    }

    if (renderer && renderer.domElement) {
      const rect = renderer.domElement.getBoundingClientRect();
      mouse.x = ((e.clientX - rect.left) / rect.width) * 2 - 1;
      mouse.y = -((e.clientY - rect.top) / rect.height) * 2 + 1;
    } else {
      mouse.x = (e.clientX / window.innerWidth) * 2 - 1;
      mouse.y = -(e.clientY / window.innerHeight) * 2 + 1;
    }

    const activeCamera = getActiveCamera();
    activeCamera.updateMatrixWorld();
    raycaster.setFromCamera(mouse, activeCamera);
    const bestHit = resolveRaycastHit(raycaster);

    if (bestHit && bestHit.type === 'shard') {
      const key = bestHit.key;
      const placement = store.get('placementData');
      const data = placement ? placement.shards.find(s => s.key === key) : null;

      if (data) {
        tooltip.style.display = 'block';
        tooltip.style.left = (e.clientX + 16) + 'px';
        tooltip.style.top = (e.clientY + 16) + 'px';
        document.getElementById('tt-title').textContent = data.key;
        document.getElementById('tt-dept').textContent = data.dept;
        document.getElementById('tt-orbit').textContent = 'Global';
        document.getElementById('tt-radius').textContent = 'N/A';
        document.getElementById('tt-size').textContent = `${data.size.w}×${data.size.d}×${data.size.h}`;
        document.getElementById('tt-sockets').textContent = data.sockets ? data.sockets.length : 0;
        return;
      }
    }
    tooltip.style.display = 'none';
  });
}

// Live reloading without full page refresh
async function reloadVisualizer() {
  try {
    const data = await loadData();
    const threePlacement = toThreeCoords(data.placement);
    store.set('placementData', threePlacement);
    store.set('routesData', data.routes);

    // Load history cache
    const projectName = store.get('projectName') || 'octopus';
    let historyData = null;
    try {
      historyData = await api.loadHistoryCache(projectName);
    } catch (e) {
      console.warn('Failed to load history cache:', e);
    }
    historyManager.deserializeHistory(historyData);
    store.set('historyUpdated', Date.now());

    // Deselect active controls to prevent attachment state bugs
    deselectAll();

    // Reload updated placement and curves statically from server
    buildSceneData(threePlacement, true);
    drawRoutes(data.routes);
    updateHUD(threePlacement);

    emit(EVENTS.DATA_RELOADED);
    emit(EVENTS.VALIDATION_REQ);
    console.log('Visualizer data updated dynamically.');
  } catch (err) {
    console.error('Error reloading visualizer data:', err);
  }
}

// Expose reloadVisualizer globally for debugging/console and listen for Event Bus trigger
window.reloadVisualizer = reloadVisualizer;
on(EVENTS.RELOAD_REQ, reloadVisualizer);
on('GRID_CONFIG_CHANGED', () => {
  const placement = store.get('placementData');
  if (placement) {
    buildSceneData(placement, true);
  }
});

async function loadProject(project) {
  store.set('projectName', project);

  // Update name in CAD panel
  const modelNameSpan = document.getElementById('cad-model-name');
  if (modelNameSpan) {
    modelNameSpan.textContent = project.toUpperCase();
  }

  const loading = document.getElementById('loading');
  loading.style.display = 'block';
  loading.textContent = `Загрузка структуры ${project}...`;

  try {
    const data = await loadData();
    const threePlacement = toThreeCoords(data.placement);
    store.set('placementData', threePlacement);
    store.set('routesData', data.routes);
    store.set('connectionMode', 1); // Default connection mode

    // Load history cache
    let historyData = null;
    try {
      historyData = await api.loadHistoryCache(project);
    } catch (e) {
      console.warn('Failed to load history cache:', e);
    }
    historyManager.deserializeHistory(historyData);
    store.set('historyUpdated', Date.now());

    // Deselect active controls to prevent attachment state bugs
    deselectAll();

    buildSceneData(threePlacement);
    drawRoutes(data.routes);
    updateHUD(threePlacement);

    loading.style.display = 'none';
    emit(EVENTS.VALIDATION_REQ);
    emit(EVENTS.DATA_RELOADED); // Trigger panels update with new project data

    console.log(`%c🧠 Axicor Visualizer project "${project}" ready`, 'color: #8b5cf6; font-size: 14px;');
  } catch (err) {
    loading.textContent = `Ошибка загрузки: ${err.message}`;
    console.error(err);
  }
}

async function init() {
  // Set dark theme on body for editor layout
  document.body.dataset.theme = 'dark';

  const container = document.getElementById('canvas-container');

  // 1. Initialize core viewport
  initViewer(container);

  // 2. Initialize interactions & UI
  initEditor();
  initUI();
  setupHoverTooltip();

  // Expose elements for debugging console
  window.scene = scene;
  window.camera = getActiveCamera();
  window.controls = controls;
  window.shardMeshes = shardMeshes;
  window.socketMeshes = socketMeshes;
  window.transformControls = transformControls;

  // Wire up summon hub button click handler to re-open selector and switch project
  const hubBtn = document.getElementById('summon-hub-flat-btn');
  if (hubBtn) {
    hubBtn.onclick = async () => {
      const newProj = await showProjectSelector();
      if (newProj && newProj !== store.get('projectName')) {
        await loadProject(newProj);
      }
    };
  }

  // Load 3D model assets for custom sockets
  const { loadPinModels } = await import('./rendering/mesh_factory.js');
  try {
    await loadPinModels();
  } catch (e) {
    console.error("Failed to load custom pin models, using fallbacks:", e);
  }

  // 3. Start the requestAnimationFrame rendering loop
  const clock = new THREE.Clock();
  animateViewer(() => {
    const dt = clock.getDelta();
    if (modeManager) {
      modeManager.update(dt);
    }
    // Dynamic handles scaling based on camera distance
    updateHandlesScale();
  });

  // 4. Select the project first and then load
  const project = await showProjectSelector();
  if (project) {
    await loadProject(project);
  }
}

// Start everything on DOM load
window.addEventListener('DOMContentLoaded', init);
