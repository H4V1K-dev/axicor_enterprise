/**
 * @fileoverview advanced_obj_prop.js — Floating panel for configuring new shard properties (orbit, dimensions, populations, department).
 */

import { store } from '../store/store.js';
// Orbit imports removed
import * as THREE from 'three';

export const advancedObjPropConfig = {
  orbit: 1,
  w: 32,
  d: 32,
  h: 16,
  populations: 1,
  dept: 'default',
  onChange: null // Callback when configuration changes
};

export const socketPropConfig = {
  name: 'sock_0',
  width: 4,
  height: 4,
  pitch: 2,
  rotation: 0,
  face: 'auto', // 'auto' | 'top' | 'bottom'
  onChange: null
};

export let activeConfigType = 'shard'; // 'shard' | 'socket'

// Compatibility alias
export const ghostConfig = advancedObjPropConfig;

let panelElement = null;

export function initAdvancedObjPropPanel(type = 'shard') {
  activeConfigType = type;
  if (panelElement) return;

  const placement = store.get('placementData');
  if (!placement) return;

  // Load defaults from store settings
  const settings = store.get('editorSettings') || {};
  advancedObjPropConfig.w = settings.default_shard_w !== undefined ? settings.default_shard_w : 32;
  advancedObjPropConfig.d = settings.default_shard_d !== undefined ? settings.default_shard_d : 32;
  advancedObjPropConfig.h = settings.default_shard_h !== undefined ? settings.default_shard_h : 16;

  socketPropConfig.width = settings.default_socket_w !== undefined ? settings.default_socket_w : 4;
  socketPropConfig.height = settings.default_socket_h !== undefined ? settings.default_socket_h : 4;
  socketPropConfig.pitch = settings.default_socket_pitch !== undefined ? settings.default_socket_pitch : 2;

  // Set default department name and level filtering logic
  const focusedLevelId = store.get('focusedLevelId') ?? 1;
  const allDepts = placement.departments || [];
  const levelDepts = allDepts.filter(d => d.orbit === focusedLevelId);

  // If the currently cached department belongs to a different level, clear it
  const currentDeptObj = allDepts.find(d => d.name === advancedObjPropConfig.dept);
  if (currentDeptObj && currentDeptObj.orbit !== focusedLevelId) {
    advancedObjPropConfig.dept = '';
  }

  // If no department is set for the current level, initialize it
  if (!advancedObjPropConfig.dept) {
    if (levelDepts.length > 0) {
      advancedObjPropConfig.dept = levelDepts[0].name;
    } else {
      advancedObjPropConfig.dept = `l${focusedLevelId}_default`;
    }
  }

  // Set default orbit to first available
  const orbits = placement.orbits || [];
  if (orbits.length > 0) {
    advancedObjPropConfig.orbit = orbits[0].index;
  }

  panelElement = document.createElement('div');
  panelElement.id = 'advanced-obj-prop';
  panelElement.className = 'ax-drawer advanced-obj-prop';

  renderPanel();
  document.body.appendChild(panelElement);
}

/**
 * Destroys the advanced obj prop panel.
 */
export function destroyAdvancedObjPropPanel() {
  if (panelElement) {
    panelElement.remove();
    panelElement = null;
  }
}

export function updateAdvancedPanelHeight() {
  if (!panelElement || !panelElement.classList.contains('open')) return;
  const targetHeight = panelElement.offsetHeight;

  const toolsSidebar = document.getElementById('tools-sidebar');
  if (toolsSidebar && toolsSidebar.classList.contains('morphed')) {
    toolsSidebar.style.setProperty('--advanced-panel-height', `${targetHeight}px`);
  }
}

function renderPanel() {
  if (!panelElement) return;

  if (activeConfigType === 'socket') {
    panelElement.innerHTML = `
      <div style="font-weight: 600; font-size: 14px; border-bottom: 1px solid var(--ax-border-subtle); padding-bottom: 6px; display: flex; align-items: center; gap: 6px;">
        <span>🔌</span>
        <span>Новый сокет</span>
      </div>
      
      <div style="display: flex; flex-direction: column; gap: 8px;">
        <div style="display: flex; flex-direction: column; gap: 4px;">
          <label style="font-size: 11px; opacity: 0.7;">Имя сокета</label>
          <input type="text" id="socket-name-input" class="ax-input" style="width: 100%;" value="${socketPropConfig.name}">
        </div>

        <div style="display: flex; gap: 8px;">
          <div style="flex: 1; display: flex; flex-direction: column; gap: 4px;">
            <label style="font-size: 11px; opacity: 0.7;">Ширина (W)</label>
            <input type="number" id="socket-w-input" class="ax-input" style="width: 100%;" min="1" step="1" value="${socketPropConfig.width}">
          </div>
          <div style="flex: 1; display: flex; flex-direction: column; gap: 4px;">
            <label style="font-size: 11px; opacity: 0.7;">Высота (H)</label>
            <input type="number" id="socket-h-input" class="ax-input" style="width: 100%;" min="1" step="1" value="${socketPropConfig.height}">
          </div>
        </div>

        <div style="display: flex; gap: 8px;">
          <div style="flex: 1; display: flex; flex-direction: column; gap: 4px;">
            <label style="font-size: 11px; opacity: 0.7;">Шаг (Pitch)</label>
            <input type="number" id="socket-pitch-input" class="ax-input" style="width: 100%;" min="1" step="1" value="${socketPropConfig.pitch}">
          </div>
          <div style="flex: 1; display: flex; flex-direction: column; gap: 4px;">
            <label style="font-size: 11px; opacity: 0.7;">Угол</label>
            <select id="socket-rot-select" class="ax-select" style="width: 100%;">
              <option value="0" ${socketPropConfig.rotation === 0 ? 'selected' : ''}>0°</option>
              <option value="90" ${socketPropConfig.rotation === 90 ? 'selected' : ''}>90°</option>
              <option value="180" ${socketPropConfig.rotation === 180 ? 'selected' : ''}>180°</option>
              <option value="270" ${socketPropConfig.rotation === 270 ? 'selected' : ''}>270°</option>
            </select>
          </div>
        </div>

        <div style="display: flex; flex-direction: column; gap: 4px;">
          <label style="font-size: 11px; opacity: 0.7;">Сторона (Face)</label>
          <select id="socket-face-select" class="ax-select" style="width: 100%;">
            <option value="auto" ${socketPropConfig.face === 'auto' ? 'selected' : ''}>Авто</option>
            <option value="top" ${socketPropConfig.face === 'top' ? 'selected' : ''}>Верх (Top)</option>
            <option value="bottom" ${socketPropConfig.face === 'bottom' ? 'selected' : ''}>Низ (Bottom)</option>
          </select>
        </div>
      </div>
    `;

    // Bind socket event listeners
    panelElement.querySelector('#socket-name-input').addEventListener('input', (e) => {
      socketPropConfig.name = e.target.value;
      triggerChange();
    });

    panelElement.querySelector('#socket-w-input').addEventListener('change', (e) => {
      let val = parseInt(e.target.value);
      if (isNaN(val) || val < 1) val = 1;
      socketPropConfig.width = val;
      e.target.value = val;
      triggerChange();
    });

    panelElement.querySelector('#socket-h-input').addEventListener('change', (e) => {
      let val = parseInt(e.target.value);
      if (isNaN(val) || val < 1) val = 1;
      socketPropConfig.height = val;
      e.target.value = val;
      triggerChange();
    });

    panelElement.querySelector('#socket-pitch-input').addEventListener('change', (e) => {
      let val = parseInt(e.target.value);
      if (isNaN(val) || val < 1) val = 1;
      socketPropConfig.pitch = val;
      e.target.value = val;
      triggerChange();
    });

    panelElement.querySelector('#socket-rot-select').addEventListener('change', (e) => {
      socketPropConfig.rotation = parseInt(e.target.value);
      triggerChange();
    });

    panelElement.querySelector('#socket-face-select').addEventListener('change', (e) => {
      socketPropConfig.face = e.target.value;
      triggerChange();
    });

    return;
  }

  // Shard rendering (default)
  const placement = store.get('placementData');
  const focusedLevelId = store.get('focusedLevelId') ?? 1;
  const depts = placement ? (placement.departments || []).filter(d => d.orbit === focusedLevelId) : [];
  const orbits = placement ? placement.orbits || [] : [];

  const orbitOptions = '';

  const hasCurrentDept = depts.some(d => d.name === advancedObjPropConfig.dept);
  let deptOptions = depts.map(d => {
    const selected = d.name === advancedObjPropConfig.dept ? 'selected' : '';
    return `<option value="${d.name}" ${selected}>${d.name}</option>`;
  }).join('');

  if (!hasCurrentDept && advancedObjPropConfig.dept) {
    deptOptions = `<option value="${advancedObjPropConfig.dept}" selected>${advancedObjPropConfig.dept}</option>` + deptOptions;
  }

  panelElement.innerHTML = `
    <div style="font-weight: 600; font-size: 14px; border-bottom: 1px solid var(--ax-border-subtle); padding-bottom: 6px; display: flex; align-items: center; gap: 6px;">
      <span>🧊</span>
      <span>Новый шард</span>
    </div>
    
    <div style="display: flex; flex-direction: column; gap: 8px;">
      <!-- Orbit selector removed -->

      <div style="display: flex; flex-direction: column; gap: 4px;">
        <label style="font-size: 11px; opacity: 0.7;">Департамент</label>
        <select id="ghost-dept-select" class="ax-select" style="width: 100%;">
          ${deptOptions}
          <option value="__new__">+ Создать новый...</option>
        </select>
      </div>

      <div style="display: flex; gap: 8px;">
        <div style="flex: 1; display: flex; flex-direction: column; gap: 4px;">
          <label style="font-size: 11px; opacity: 0.7;">Ширина (W)</label>
          <input type="number" id="ghost-w-input" class="ax-input" style="width: 100%;" min="1" step="1" value="${advancedObjPropConfig.w}">
        </div>
        <div style="flex: 1; display: flex; flex-direction: column; gap: 4px;">
          <label style="font-size: 11px; opacity: 0.7;">Глубина (D)</label>
          <input type="number" id="ghost-d-input" class="ax-input" style="width: 100%;" min="1" step="1" value="${advancedObjPropConfig.d}">
        </div>
        <div style="flex: 1; display: flex; flex-direction: column; gap: 4px;">
          <label style="font-size: 11px; opacity: 0.7;">Высота (H)</label>
          <input type="number" id="ghost-h-input" class="ax-input" style="width: 100%;" min="1" step="1" value="${advancedObjPropConfig.h}">
        </div>
      </div>

      <div style="display: flex; flex-direction: column; gap: 4px;">
        <label style="font-size: 11px; opacity: 0.7;">Количество популяций (слоёв)</label>
        <input type="number" id="ghost-pop-input" class="ax-input" style="width: 100%;" min="1" max="10" step="1" value="${advancedObjPropConfig.populations}">
      </div>
    </div>
  `;

  // Bind change events
  // Orbit change listener removed

  const deptSelect = panelElement.querySelector('#ghost-dept-select');
  deptSelect.addEventListener('change', (e) => {
    if (e.target.value === '__new__') {
      const newName = prompt('Введите имя нового департамента:');
      if (newName && newName.trim()) {
        const cleanName = newName.trim();
        // Check if already exists globally
        const allDepts = placement ? (placement.departments || []) : [];
        const exists = allDepts.some(d => d.name.toLowerCase() === cleanName.toLowerCase());
        if (exists) {
          alert('Департамент с таким именем уже существует.');
          deptSelect.value = advancedObjPropConfig.dept;
        } else {
          // Add to temporary list in placementData to render it
          const focusedLevelId = store.get('focusedLevelId') ?? 1;
          depts.push({ name: cleanName, orbit: focusedLevelId });
          advancedObjPropConfig.dept = cleanName;
          renderPanel();
          triggerChange();
        }
      } else {
        deptSelect.value = advancedObjPropConfig.dept;
      }
    } else {
      advancedObjPropConfig.dept = e.target.value;
      triggerChange();
    }
  });

  const wInput = panelElement.querySelector('#ghost-w-input');
  wInput.addEventListener('change', (e) => {
    let val = parseInt(e.target.value);
    if (isNaN(val) || val < 1) val = 1;
    advancedObjPropConfig.w = val;
    e.target.value = val;
    triggerChange();
  });

  const dInput = panelElement.querySelector('#ghost-d-input');
  dInput.addEventListener('change', (e) => {
    let val = parseInt(e.target.value);
    if (isNaN(val) || val < 1) val = 1;
    advancedObjPropConfig.d = val;
    e.target.value = val;
    triggerChange();
  });

  const hInput = panelElement.querySelector('#ghost-h-input');
  hInput.addEventListener('change', (e) => {
    let val = parseInt(e.target.value);
    if (isNaN(val) || val < 1) val = 1;
    advancedObjPropConfig.h = val;
    e.target.value = val;
    triggerChange();
  });

  const popInput = panelElement.querySelector('#ghost-pop-input');
  popInput.addEventListener('change', (e) => {
    let val = parseInt(e.target.value);
    if (isNaN(val) || val < 1) val = 1;
    if (val > 10) val = 10;
    advancedObjPropConfig.populations = val;
    e.target.value = val;
    triggerChange();
  });
}

function triggerChange() {
  const cfg = activeConfigType === 'socket' ? socketPropConfig : advancedObjPropConfig;
  if (cfg.onChange) {
    cfg.onChange();
  }
  updateAdvancedPanelHeight();
}
