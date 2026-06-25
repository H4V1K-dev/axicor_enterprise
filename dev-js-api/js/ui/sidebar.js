/**
 * @fileoverview sidebar.js — Sidebar inspector rendering and interactions (shard/socket details, layers DND, coordinate updates, save changes).
 */

import * as THREE from 'three';
import { shardMeshes, socketMeshes, VIS_SCALE, shardDataMap } from '../scene_builder.js';
import { deselectAll, selectSocket, checkShardCollision, updateSelectedSocket } from '../editor.js';
import { showToast } from './toast.js';
import { store } from '../store/store.js';
import { emit, EVENTS } from '../store/event_bus.js';
import { renderer } from '../viewer.js';
import { historyManager } from '../store/history_manager.js';
import { historyToRust } from '../editor/coordinate_adapter.js';

let sidebar = null;

function getSidebarElement() {
  if (!sidebar) {
    sidebar = document.getElementById('sidebar');
    if (!sidebar) {
      sidebar = document.createElement('div');
      sidebar.id = 'sidebar';
      sidebar.className = 'ax-panel';
      document.body.appendChild(sidebar);
    }
  }
  return sidebar;
}

function getSidebarInnerElement() {
  const container = getSidebarElement();
  let inner = document.getElementById('sidebar-inner');
  if (!inner) {
    inner = document.createElement('div');
    inner.id = 'sidebar-inner';
    container.appendChild(inner);
  }
  return inner;
}

export function showSidebar(type, data) {
  const el = getSidebarElement();
  el.style.display = 'flex';

  if (type === 'shard') {
    renderShardSidebar(data);
  } else if (type === 'socket') {
    renderSocketSidebar(data);
  }
}

export function hideSidebar() {
  const el = getSidebarElement();
  el.style.display = 'none';
}

export function renderLayersListItems(data) {
  if (!data.layers) return '';
  return data.layers.map((l, index) => {
    const thickness = Math.round(data.size.h * l.height_pct);
    const pct = (l.height_pct * 100).toFixed(1);
    return `
      <div class="ax-list-item layer-list-item" draggable="true" data-name="${l.name}" data-index="${index}" style="cursor: grab; justify-content: space-between; width: 100%;">
        <div style="display:flex; align-items:center; gap:8px;">
          <span style="color: var(--ax-text-faint); font-size:14px; cursor:grab;">☰</span>
          <div style="display:flex; flex-direction:column; gap:2px; align-items:flex-start;">
            <span style="color: var(--ax-text); font-weight:600; font-size:13px;">${l.name}</span>
            <span style="color: var(--ax-text-faint); font-size:10px;">Плотность: ${l.density || 1.0}</span>
          </div>
        </div>
        <span style="color: var(--ax-accent); font-family:var(--ax-font-mono); font-weight:bold; font-size:12px;">${thickness} vx (${pct}%)</span>
      </div>
    `;
  }).join('');
}

function renderShardSidebar(data) {
  const mesh = shardMeshes.get(data.key);
  if (!mesh) return;

  const px = data.position.x;
  const py = data.position.y; // Rust Y (depth)
  const pz = data.position.z; // Rust Z (height)

  const placementData = store.get('placementData');
  const deptObj = placementData ? placementData.departments.find(d => d.name === data.dept) : null;
  const deptPos = deptObj && deptObj.position ? deptObj.position : { x: 0, y: 0 };

  // Group sockets for the list
  const topSocks = [];
  const bottomSocks = [];

  if (data.sockets) {
    data.sockets.forEach(sock => {
      const socketKey = `${data.key}.${sock.name}`;
      const socketGroup = socketMeshes.get(socketKey);
      if (socketGroup) {
        if (socketGroup.userData.faceSign === 1) {
          topSocks.push({ name: sock.name, key: socketKey });
        } else {
          bottomSocks.push({ name: sock.name, key: socketKey });
        }
      }
    });
  }

  // Generate HTML list
  let topListHtml = topSocks.map(s => `<li class="ax-list-item socket-item-btn" data-key="${s.key}" style="width:100%">${s.name}</li>`).join('') || '<li class="project-empty" style="list-style:none;">Нет</li>';
  let bottomListHtml = bottomSocks.map(s => `<li class="ax-list-item socket-item-btn" data-key="${s.key}" style="width:100%">${s.name}</li>`).join('') || '<li class="project-empty" style="list-style:none;">Нет</li>';

  let layersHtml = '';
  if (data.layers && data.layers.length > 0) {
    layersHtml = `
      <div class="sb-section" id="shard-layers-section">
        <h4 class="ax-section-title">Слои шарда:</h4>
        <div id="layers-list-container" class="ax-list">
          ${renderLayersListItems(data)}
        </div>
      </div>
    `;
  }

  const levels = placementData ? placementData.levels || [] : [];
  const levelOptions = levels.map(lvl => {
    const selected = lvl.id === data.orbit ? 'selected' : '';
    return `<option value="${lvl.id}" ${selected}>l${lvl.id} (${lvl.name})</option>`;
  }).join('');

  const levelDepts = placementData ? (placementData.departments || []).filter(d => d.orbit === data.orbit) : [];
  let deptOptions = levelDepts.map(d => {
    const selected = d.name === data.dept ? 'selected' : '';
    return `<option value="${d.name}" ${selected}>${d.name}</option>`;
  }).join('');

  if (!levelDepts.some(d => d.name === data.dept) && data.dept) {
    deptOptions = `<option value="${data.dept}" selected>${data.dept}</option>` + deptOptions;
  }

  inner.innerHTML = `
    <h3 class="ax-section-title">${data.shard}</h3>
    <div class="sb-section">
      <div class="sb-row">
        <label>Уровень:</label>
        <select id="shard-orbit-select" class="ax-select" style="width: 160px; background:var(--ax-bg-input); border:1px solid var(--ax-border-subtle); color:var(--ax-text); padding:2px 4px; border-radius:4px; font-size:11px;">
          ${levelOptions}
        </select>
      </div>
      <div class="sb-row">
        <label>Департамент:</label>
        <select id="shard-dept-select" class="ax-select" style="width: 160px; background:var(--ax-bg-input); border:1px solid var(--ax-border-subtle); color:var(--ax-text); padding:2px 4px; border-radius:4px; font-size:11px;">
          ${deptOptions}
          <option value="__new__">+ Создать новый...</option>
        </select>
      </div>
      <div class="sb-row"><label>Толщина:</label> <span>${data.size.h} vx</span></div>
      <div class="sb-row"><label>Размеры:</label> <span>${data.size.w} × ${data.size.d}</span></div>
    </div>
    
    <div class="sb-section">
      <div class="sb-input-group">
        <label>Координаты Шарда (X, Y глубина, Z высота):</label>
        <div class="sb-inputs-row">
          <input type="number" id="shard-px" class="ax-input" value="${px}" step="1">
          <input type="number" id="shard-py" class="ax-input" value="${py}" step="1">
          <input type="number" id="shard-pz" class="ax-input" value="${pz}" step="1">
        </div>
      </div>
    </div>

    <div class="sb-section" style="border-top: 1px solid var(--ax-border-muted); padding-top: 12px; margin-top: 12px;">
      <div class="sb-input-group">
        <label>Положение Департамента (${data.dept}) X, Y:</label>
        <div class="sb-inputs-row">
          <input type="number" id="dept-px" class="ax-input" value="${Math.round(deptPos.x)}" step="1">
          <input type="number" id="dept-py" class="ax-input" value="${Math.round(deptPos.y)}" step="1">
        </div>
      </div>
    </div>

    ${layersHtml}

    <div class="sb-section">
      <h4 class="ax-section-title">Верхние сокеты (Top):</h4>
      <ul class="ax-list">
        ${topListHtml}
      </ul>
      <h4 class="ax-section-title">Нижние сокеты (Bottom):</h4>
      <ul class="ax-list">
        ${bottomListHtml}
      </ul>
    </div>
    
    <button class="ax-btn ax-btn--secondary" id="deselect-btn">Снять выделение</button>
  `;

  let initialShardState = JSON.parse(JSON.stringify(data));

  // Bind coord changes
  const ix = document.getElementById('shard-px');
  const iy = document.getElementById('shard-py');
  const iz = document.getElementById('shard-pz');

  const updateCoords = () => {
    const valX = parseFloat(ix.value);
    const valY = parseFloat(iy.value); // Rust Y (depth, Three Z)
    const valZ = parseFloat(iz.value); // Rust Z (height, Three Y)

    const w = data.size.w;
    const d = data.size.d;
    const h = data.size.h;

    // Convert AABB min coordinates to Three.js center position
    mesh.position.x = (valX + w / 2) * VIS_SCALE;
    mesh.position.z = (valY + d / 2) * VIS_SCALE;
    mesh.position.y = (valZ + h / 2) * VIS_SCALE;

    // Collision check: check overlap and revert if necessary
    if (checkShardCollision(data.key, mesh.position)) {
      mesh.position.copy(mesh.userData.lastValidPosition);
      ix.value = Math.round(mesh.position.x / VIS_SCALE - w / 2);
      iy.value = Math.round(mesh.position.z / VIS_SCALE - d / 2);
      iz.value = Math.round(mesh.position.y / VIS_SCALE - h / 2);
    } else {
      mesh.userData.lastValidPosition.copy(mesh.position);
    }

    // Immediately update placementData so history and visualizer stay in sync
    const pData = store.get('placementData');
    if (pData) {
      const shard = pData.shards.find(s => s.key === data.key);
      if (shard) {
        shard.position.x = Math.round(mesh.position.x / VIS_SCALE - w / 2);
        shard.position.y = Math.round(mesh.position.y / VIS_SCALE - h / 2);
        shard.position.z = Math.round(mesh.position.z / VIS_SCALE - d / 2);

        // Recalculate department position in UI
        const dObj = pData.departments.find(d => d.name === data.dept);
        if (dObj) {
          const deptShards = pData.shards.filter(s => s.dept === data.dept);
          const xMin = Math.min(...deptShards.map(s => s.position.x));
          const zMin = Math.min(...deptShards.map(s => s.position.z));
          dObj.position.x = xMin;
          dObj.position.z = zMin;

          const idx_d = document.getElementById('dept-px');
          const idy_d = document.getElementById('dept-py');
          if (idx_d) idx_d.value = xMin;
          if (idy_d) idy_d.value = zMin;
        }

        store.set('placementData', pData);
      }
    }

    import('../scene_builder.js').then(({ updateContainerWires }) => {
      updateContainerWires();
    });
    emit(EVENTS.VALIDATION_REQ);
  };

  const commitCoordChange = () => {
    const pData = store.get('placementData');
    if (!pData) return;
    const shard = pData.shards.find(s => s.key === data.key);
    if (!shard) return;

    if (initialShardState.position.x !== shard.position.x ||
      initialShardState.position.y !== shard.position.y ||
      initialShardState.position.z !== shard.position.z) {
      const undoState = JSON.parse(JSON.stringify(initialShardState));
      const redoState = JSON.parse(JSON.stringify(shard));

      import('../store/history_manager.js').then(({ historyManager }) => {
        historyManager.pushAction('move', 'shard', data.key, `Перемещение шарда ${data.key}`, undoState, redoState);
      });
      initialShardState = JSON.parse(JSON.stringify(shard));
    }
  };

  ix.addEventListener('change', () => { updateCoords(); commitCoordChange(); });
  iy.addEventListener('change', () => { updateCoords(); commitCoordChange(); });
  iz.addEventListener('change', () => { updateCoords(); commitCoordChange(); });

  // Bind department coord changes
  const idx_d = document.getElementById('dept-px');
  const idy_d = document.getElementById('dept-py');
  if (idx_d && idy_d) {
    let lastDeptX = parseFloat(idx_d.value);
    let lastDeptY = parseFloat(idy_d.value);

    const updateDeptCoords = () => {
      const valX = parseFloat(idx_d.value);
      const valY = parseFloat(idy_d.value);

      const deltaX = valX - lastDeptX;
      const deltaY = valY - lastDeptY;

      if (deltaX === 0 && deltaY === 0) return;

      const pData = store.get('placementData');
      if (!pData) return;

      pData.shards.forEach(s => {
        if (s.dept === data.dept) {
          s.position.x += deltaX;
          s.position.z += deltaY;

          const m = shardMeshes.get(s.key);
          if (m) {
            m.position.x += deltaX * VIS_SCALE;
            m.position.z += deltaY * VIS_SCALE;
            m.userData.lastValidPosition.copy(m.position);
          }
        }
      });

      const dObj = pData.departments.find(d => d.name === data.dept);
      if (dObj) {
        dObj.position.x = valX;
        dObj.position.z = valY;
      }

      store.set('placementData', pData);

      lastDeptX = valX;
      lastDeptY = valY;

      // Update current shard coordinates in inputs
      const currentShard = pData.shards.find(s => s.key === data.key);
      if (currentShard) {
        ix.value = currentShard.position.x;
        iy.value = currentShard.position.y;
      }

      import('../scene_builder.js').then(({ updateContainerWires }) => {
        updateContainerWires();
      });
      emit(EVENTS.VALIDATION_REQ);
    };

    idx_d.addEventListener('change', () => { updateDeptCoords(); store.set('hasUnsavedChanges', true); });
    idy_d.addEventListener('change', () => { updateDeptCoords(); store.set('hasUnsavedChanges', true); });
  }

  // Bind level and department selector event listeners
  const orbitSelect = document.getElementById('shard-orbit-select');
  const deptSelect = document.getElementById('shard-dept-select');

  if (orbitSelect && deptSelect) {
    orbitSelect.addEventListener('change', (e) => {
      const newOrbit = parseInt(e.target.value);
      if (newOrbit === data.orbit) return;

      const pData = store.get('placementData');
      if (!pData) return;

      const shard = pData.shards.find(s => s.key === data.key);
      if (shard) {
        const oldKey = shard.key;
        shard.orbit = newOrbit;

        // Auto-assign to default department of new orbit
        const depts = pData.departments || [];
        const newOrbitDepts = depts.filter(d => d.orbit === newOrbit);

        let newDeptName = '';
        if (newOrbitDepts.length > 0) {
          newDeptName = newOrbitDepts[0].name;
        } else {
          newDeptName = `l${newOrbit}_default`;
          // Add this new department automatically to placementData so layout is resolved
          depts.push({ name: newDeptName, orbit: newOrbit });
        }

        // Rename the shard key to reflect new department
        const shortName = oldKey.split('.').pop() || oldKey;
        const newKey = `${newDeptName}.${shortName}`;

        shard.dept = newDeptName;
        shard.key = newKey;

        // Update connections referencing this shard key
        if (pData.connections) {
          pData.connections.forEach(c => {
            if (c.from === oldKey) c.from = newKey;
            if (c.to === oldKey) c.to = newKey;
          });
        }
        if (pData.deleted_shards) {
          pData.deleted_shards = pData.deleted_shards.map(k => k === oldKey ? newKey : k);
        }

        store.set('placementData', pData);
        store.set('hasUnsavedChanges', true);

        // Record history action
        import('../store/history_manager.js').then(({ historyManager }) => {
          historyManager.pushAction('move_level', 'shard', oldKey, `Перенос шарда ${shortName} на Уровень ${newOrbit}`, initialShardState, JSON.parse(JSON.stringify(shard)));
        });

        // Clear select shard so rebuild works cleanly and re-select new key
        import('../../scene_builder.js').then(({ buildSceneData }) => {
          buildSceneData(pData, true);
          import('../selection.js').then(({ selectShard }) => {
            selectShard(newKey);
          });
        });
      }
    });

    deptSelect.addEventListener('change', (e) => {
      const pData = store.get('placementData');
      if (!pData) return;

      const shard = pData.shards.find(s => s.key === data.key);
      if (shard) {
        const oldKey = shard.key;
        const shortName = oldKey.split('.').pop() || oldKey;

        if (e.target.value === '__new__') {
          const newName = prompt('Введите имя нового департамента:');
          if (newName && newName.trim()) {
            const cleanName = newName.trim();
            const allDepts = pData.departments || [];
            const exists = allDepts.some(d => d.name.toLowerCase() === cleanName.toLowerCase());
            if (exists) {
              alert('Департамент с таким именем уже существует.');
              deptSelect.value = shard.dept;
            } else {
              // Add to placementData.departments
              pData.departments.push({ name: cleanName, orbit: shard.orbit });
              shard.dept = cleanName;

              const newKey = `${cleanName}.${shortName}`;
              shard.key = newKey;

              // Update connections
              if (pData.connections) {
                pData.connections.forEach(c => {
                  if (c.from === oldKey) c.from = newKey;
                  if (c.to === oldKey) c.to = newKey;
                });
              }

              store.set('placementData', pData);
              store.set('hasUnsavedChanges', true);

              import('../../scene_builder.js').then(({ buildSceneData }) => {
                buildSceneData(pData, true);
                import('../selection.js').then(({ selectShard }) => {
                  selectShard(newKey);
                });
              });
            }
          } else {
            deptSelect.value = shard.dept;
          }
        } else {
          const newDeptName = e.target.value;
          shard.dept = newDeptName;
          const newKey = `${newDeptName}.${shortName}`;
          shard.key = newKey;

          // Update connections
          if (pData.connections) {
            pData.connections.forEach(c => {
              if (c.from === oldKey) c.from = newKey;
              if (c.to === oldKey) c.to = newKey;
            });
          }

          store.set('placementData', pData);
          store.set('hasUnsavedChanges', true);

          import('../../scene_builder.js').then(({ buildSceneData }) => {
            buildSceneData(pData, true);
            import('../selection.js').then(({ selectShard }) => {
              selectShard(newKey);
            });
          });
        }
      }
    });
  }

  document.getElementById('deselect-btn').addEventListener('click', deselectAll);

  // Bind click listeners to socket list items
  inner.querySelectorAll('.socket-item-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      const sKey = btn.dataset.key;
      selectSocket(sKey);
    });
  });

  // Bind Drag and Drop events for layers reordering
  const layersListContainer = document.getElementById('layers-list-container');
  if (layersListContainer) {
    let draggedItem = null;

    layersListContainer.addEventListener('dragstart', (e) => {
      draggedItem = e.target.closest('.layer-list-item');
      if (draggedItem) {
        draggedItem.style.opacity = '0.5';
        e.dataTransfer.effectAllowed = 'move';
        e.dataTransfer.setData('text/plain', draggedItem.dataset.name);
      }
    });

    layersListContainer.addEventListener('dragend', (e) => {
      if (draggedItem) {
        draggedItem.style.opacity = '1';
      }
    });

    layersListContainer.addEventListener('dragover', (e) => {
      e.preventDefault();
      e.dataTransfer.dropEffect = 'move';
      const targetItem = e.target.closest('.layer-list-item');
      if (targetItem && targetItem !== draggedItem) {
        const bounding = targetItem.getBoundingClientRect();
        const offset = e.clientY - bounding.top;
        if (offset > bounding.height / 2) {
          targetItem.after(draggedItem);
        } else {
          targetItem.before(draggedItem);
        }
      }
    });

    layersListContainer.addEventListener('drop', (e) => {
      e.preventDefault();

      const items = Array.from(layersListContainer.querySelectorAll('.layer-list-item'));
      const newOrderNames = items.map(item => item.dataset.name);

      const shardMesh = shardMeshes.get(data.key);
      const sd = shardDataMap.get(shardMesh.uuid);
      if (sd && sd.layers) {
        const layerMap = {};
        sd.layers.forEach(l => { layerMap[l.name] = l; });
        sd.layers = newOrderNames.map(name => layerMap[name]);

        // Dynamically update the 3D meshes order
        if (window.updateLayersOrderIn3D) {
          window.updateLayersOrderIn3D(shardMesh, newOrderNames);
        }

        // Re-spawn somas since layers moved!
        if (window.spawnSomasForShard) {
          window.spawnSomasForShard(data.key);
        }

        // Save order change
        store.set('hasUnsavedChanges', true);
      }
    });
  }
}

function renderSocketSidebar(data) {
  const el = getSidebarElement();
  const inner = getSidebarInnerElement();
  inner.innerHTML = `
    <h3 class="ax-section-title">Сокет: ${data.socketName}</h3>
    <div class="sb-section">
      <div class="sb-row"><label>Родительский шард:</label> <span>${data.shardKey}</span></div>
      <div class="sb-row"><label>Направление грани:</label> <span>${data.faceSign === 1 ? 'СВЕРХУ (Top)' : 'СНИЗУ (Bottom)'}</span></div>
      <div class="sb-row"><label>Привязка Z (Слой):</label> <span id="sock-entry-z-display" style="font-weight: 600; color: var(--ax-accent);">${data.entry_z || (data.faceSign === 1 ? 'top' : 'bottom')}</span></div>
    </div>

    <div class="sb-section">
      <div class="sb-input-group">
        <label>Матрица пикселей (Ряды × Колонки):</label>
        <div class="sb-inputs-row">
          <input type="number" id="sock-w" class="ax-input" value="${data.width}" min="2" max="64">
          <input type="number" id="sock-h" class="ax-input" value="${data.height}" min="2" max="64">
        </div>
      </div>
    </div>

    <div class="sb-section">
      <div class="sb-input-group">
        <label>Интервал пинов (Pitch):</label>
        <div class="sb-inputs-row" style="gap:4px">
          <button class="ax-btn ax-btn--secondary pitch-btn ${data.pitch === 1 ? 'ax-btn--primary' : ''}" data-pitch="1">x1</button>
          <button class="ax-btn ax-btn--secondary pitch-btn ${data.pitch === 2 ? 'ax-btn--primary' : ''}" data-pitch="2">x2</button>
          <button class="ax-btn ax-btn--secondary pitch-btn ${data.pitch === 3 ? 'ax-btn--primary' : ''}" data-pitch="3">x3</button>
          <button class="ax-btn ax-btn--secondary pitch-btn ${data.pitch === 4 ? 'ax-btn--primary' : ''}" data-pitch="4">x4</button>
        </div>
      </div>
    </div>

    <div class="sb-section">
      <div class="sb-input-group">
        <label>Вращение сокета:</label>
        <div class="sb-inputs-row">
          <select id="sock-rot" class="ax-select">
            <option value="0" ${data.rotation === 0 ? 'selected' : ''}>0°</option>
            <option value="90" ${data.rotation === 90 ? 'selected' : ''}>90°</option>
            <option value="180" ${data.rotation === 180 ? 'selected' : ''}>180°</option>
            <option value="270" ${data.rotation === 270 ? 'selected' : ''}>270°</option>
          </select>
        </div>
      </div>
    </div>

    <div class="sb-section">
      <div class="sb-input-group">
        <label>Сторона расположения:</label>
        <div class="sb-inputs-row">
          <select id="sock-face" class="ax-select">
            <option value="1" ${data.faceSign === 1 ? 'selected' : ''}>СВЕРХУ (Top)</option>
            <option value="-1" ${data.faceSign === -1 ? 'selected' : ''}>СНИЗУ (Bottom)</option>
          </select>
        </div>
      </div>
    </div>

    <div class="sb-section">
      <div class="sb-input-group">
        <label>Смещение сокета на грани (X, Y):</label>
        <div class="sb-inputs-row">
          <input type="number" id="sock-ox" class="ax-input" value="${Math.round(data.originalOffset.x)}" step="0.5">
          <input type="number" id="sock-oy" class="ax-input" value="${Math.round(data.originalOffset.y)}" step="0.5">
        </div>
      </div>
    </div>

    <button class="ax-btn ax-btn--secondary" id="deselect-btn">Снять выделение</button>
  `;

  let initialSocketState = JSON.parse(JSON.stringify(data));

  const sw = document.getElementById('sock-w');
  const sh = document.getElementById('sock-h');
  const ox = document.getElementById('sock-ox');
  const oy = document.getElementById('sock-oy');
  const rot = document.getElementById('sock-rot');
  const faceSelect = document.getElementById('sock-face');
  let currentPitch = data.pitch;

  const triggerUpdate = () => {
    const w = parseInt(sw.value);
    const h = parseInt(sh.value);
    const r = parseInt(rot.value);
    const fs = parseInt(faceSelect.value);

    // Read current offset.z from placementData to avoid wiping it out
    const placementData = store.get('placementData');
    let zVal = 0;
    if (placementData) {
      const shard = placementData.shards.find(s => s.key === data.shardKey);
      if (shard && shard.sockets) {
        const socket = shard.sockets.find(s => s.name === data.socketName);
        if (socket && socket.offset && socket.offset.z !== undefined) {
          zVal = socket.offset.z;
        }
      }
    } else {
      zVal = (data.originalOffset && data.originalOffset.z !== undefined)
        ? data.originalOffset.z
        : ((data.offset && data.offset.z !== undefined) ? data.offset.z : 0);
    }

    const offset = { x: parseFloat(ox.value), y: parseFloat(oy.value), z: zVal };
    updateSelectedSocket(w, h, currentPitch, offset, r, fs);
  };

  const commitSocketChange = () => {
    const placementData = store.get('placementData');
    if (!placementData) return;
    const shard = placementData.shards.find(s => s.key === data.shardKey);
    if (!shard || !shard.sockets) return;
    const socket = shard.sockets.find(s => s.name === data.socketName);
    if (!socket) return;

    const initOffset = initialSocketState.offset || initialSocketState.originalOffset || { x: 0, y: 0, z: 0 };
    const currOffset = socket.offset || { x: 0, y: 0, z: 0 };

    const initZ = initOffset.z !== undefined ? initOffset.z : 0;
    const currZ = currOffset.z !== undefined ? currOffset.z : 0;

    if (initialSocketState.width !== socket.width ||
      initialSocketState.height !== socket.height ||
      initialSocketState.pitch !== socket.pitch ||
      initOffset.x !== currOffset.x ||
      initOffset.y !== currOffset.y ||
      initZ !== currZ ||
      initialSocketState.rotation !== socket.rotation ||
      initialSocketState.faceSign !== socket.faceSign) {

      const undoState = JSON.parse(JSON.stringify(initialSocketState));
      const redoState = JSON.parse(JSON.stringify(socket));
      const socketKey = `${data.shardKey}.${data.socketName}`;

      let actionType = 'resize';
      let actionDesc = `Изменение параметров сокета ${data.socketName}`;

      const onlyCoordsChanged =
        initialSocketState.width === socket.width &&
        initialSocketState.height === socket.height &&
        initialSocketState.pitch === socket.pitch &&
        initialSocketState.rotation === socket.rotation &&
        initialSocketState.faceSign === socket.faceSign &&
        (initOffset.x !== currOffset.x || initOffset.y !== currOffset.y || initZ !== currZ);

      if (onlyCoordsChanged) {
        actionType = 'move';
        actionDesc = `Перемещение сокета ${data.socketName}`;
      }

      import('../store/history_manager.js').then(({ historyManager }) => {
        historyManager.pushAction(actionType, 'socket', socketKey, actionDesc, undoState, redoState);
      });
      initialSocketState = JSON.parse(JSON.stringify(socket));
    }
  };

  sw.addEventListener('change', () => { triggerUpdate(); commitSocketChange(); });
  sh.addEventListener('change', () => { triggerUpdate(); commitSocketChange(); });
  ox.addEventListener('change', () => { triggerUpdate(); commitSocketChange(); });
  oy.addEventListener('change', () => { triggerUpdate(); commitSocketChange(); });
  rot.addEventListener('change', () => { triggerUpdate(); commitSocketChange(); });
  faceSelect.addEventListener('change', () => { triggerUpdate(); commitSocketChange(); });

  // Pitch buttons listeners
  inner.querySelectorAll('.pitch-btn').forEach(btn => {
    btn.addEventListener('click', (e) => {
      inner.querySelectorAll('.pitch-btn').forEach(b => b.classList.remove('ax-btn--primary'));
      btn.classList.add('ax-btn--primary');
      currentPitch = parseInt(btn.dataset.pitch);
      triggerUpdate();
      commitSocketChange();
    });
  });

  document.getElementById('deselect-btn').addEventListener('click', deselectAll);
}

export async function saveAllLayoutChanges() {
  const placementData = store.get('placementData');
  const payload = {
    project: store.get('projectName') || 'octopus',
    levels: placementData ? placementData.levels || [] : [],
    shards: {},
    sockets: {},
    connections: placementData ? placementData.connections || [] : [],
    deleted_shards: placementData ? placementData.deleted_shards || [] : [],
    deleted_sockets: placementData ? placementData.deleted_sockets || [] : [],
    deleted_connections: placementData ? placementData.deleted_connections || [] : [],
    simulation: placementData ? placementData.simulation || {} : {},
    world: placementData ? placementData.world || {} : {},
    preview: renderer ? renderer.domElement.toDataURL('image/png') : null,
    history: historyToRust({
      globalStack: historyManager.globalStack,
      globalIndex: historyManager.globalIndex,
      objectHistory: historyManager.objectHistory
    })
  };

  // 1. Gather all shard position, size and layer overrides
  for (const [key, mesh] of shardMeshes.entries()) {
    // Retrieve current size from the modified mesh geometry parameters
    const currentW = Math.round(mesh.geometry.parameters.width / VIS_SCALE);
    const currentH = Math.round(mesh.geometry.parameters.height / VIS_SCALE); // height is now h (Three Y)
    const currentD = Math.round(mesh.geometry.parameters.depth / VIS_SCALE);  // depth is now d (Three Z)

    const sd = shardDataMap.get(mesh.uuid);
    payload.shards[key] = {
      position: {
        x: Math.round(mesh.position.x / VIS_SCALE - currentW / 2),
        y: Math.round(mesh.position.z / VIS_SCALE - currentD / 2), // Rust Y (depth)
        z: Math.round(mesh.position.y / VIS_SCALE - currentH / 2) // Rust Z (height)
      },
      size: {
        w: currentW,
        d: currentD,
        h: currentH
      },
      orbit: sd ? sd.orbit : undefined,
      dept: sd ? sd.dept : undefined,
      shard: sd ? sd.shard : undefined,
      layers: sd ? sd.layers : undefined,
      sockets: []
    };
    if (sd && sd.layers && sd.layers.length > 0) {
      const layerProps = {};
      sd.layers.forEach(l => {
        layerProps[l.name] = Number(l.height_pct.toFixed(4));
      });
      payload.shards[key].layer_proportions = layerProps;
    }
  }

  // 2. Sockets are disabled in Composition mode
  payload.sockets = {};

  showToast('Сохранение топологии...', 'info');

  try {
    const response = await fetch('/api/save', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload)
    });

    if (!response.ok) {
      throw new Error(`Ошибка сервера: ${response.status}`);
    }

    const resData = await response.json();
    showToast('Конфигурация сохранена! Обновление связей...', 'success');

    // Clear deleted trackers
    const pData = store.get('placementData');
    if (pData) {
      pData.deleted_shards = [];
      pData.deleted_sockets = [];
      pData.deleted_connections = [];
      store.set('placementData', pData);
    }

    store.set('hasUnsavedChanges', false);

    // Reload updated placement and curves statically from server
    emit(EVENTS.RELOAD_REQ);

  } catch (err) {
    showToast(`Не удалось сохранить: ${err.message}`, 'error');
    console.error(err);
  }
}

function updateSidebarVisibility() {
  const selShardKey = store.get('selectedShardKey');
  const selSocketKey = store.get('selectedSocketKey');

  if (selShardKey) {
    const mesh = shardMeshes.get(selShardKey);
    const shardData = mesh ? shardDataMap.get(mesh.uuid) : null;
    if (shardData) {
      showSidebar('shard', shardData);
    } else {
      hideSidebar();
    }
  } else if (selSocketKey) {
    const group = socketMeshes.get(selSocketKey);
    if (group && group.userData) {
      showSidebar('socket', group.userData);
    } else {
      hideSidebar();
    }
  } else {
    hideSidebar();
  }
}

store.on('selectedShardKey', updateSidebarVisibility);
store.on('selectedSocketKey', updateSidebarVisibility);

