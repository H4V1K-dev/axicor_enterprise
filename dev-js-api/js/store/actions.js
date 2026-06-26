/**
 * @fileoverview actions.js — Centralized state mutators and side-effects (Actions).
 */

import { store } from './store.js';
import { historyManager } from './history_manager.js';
import { emit, EVENTS } from './event_bus.js';
import { api } from '../services/api.js';
import { renderer } from '../viewer.js';
import { historyToRust } from '../editor/coordinate_adapter.js';
import { shardMeshes, socketMeshes, shardDataMap, VIS_SCALE } from '../scene_builder.js';
import { showToast } from '../ui/toast.js';

/**
 * Collect overrides and save layout changes to backend.
 * @returns {Promise<void>}
 */
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
    const currentH = Math.round(mesh.geometry.parameters.height / VIS_SCALE); // height is h (Three Y)
    const currentD = Math.round(mesh.geometry.parameters.depth / VIS_SCALE);  // depth is d (Three Z)

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
    const resData = await api.saveLayout(payload);
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

/**
 * Helper to parse a socket key into its shard key and socket name components.
 * @param {string} socketKey 
 * @returns {{ shardKey: string, socketName: string }}
 */
function parseSocketKey(socketKey) {
  const lastDot = socketKey.lastIndexOf('.');
  const shardKey = socketKey.substring(0, lastDot);
  const socketName = socketKey.substring(lastDot + 1);
  return { shardKey, socketName };
}

/**
 * Select a shard (atomic update of state).
 * @param {string} key 
 * @param {boolean} isMulti 
 */
export function selectShardAction(key, isMulti = false) {
  const currentKeys = store.get('selectedShardKeys') || new Set();
  let newKeys;

  if (isMulti) {
    newKeys = new Set(currentKeys);
    if (newKeys.has(key)) {
      newKeys.delete(key);
    } else {
      newKeys.add(key);
    }
  } else {
    newKeys = new Set([key]);
  }

  const activeKey = newKeys.size > 0 ? Array.from(newKeys)[newKeys.size - 1] : null;

  store.setMultiple({
    selectedShardKeys: newKeys,
    selectedShardKey: activeKey,
    selectedSocketKey: null,
    connectionMode: 1
  });
}

/**
 * Select a socket (atomic update of state).
 * @param {string} key 
 */
export function selectSocketAction(key) {
  store.setMultiple({
    selectedSocketKey: key,
    selectedShardKey: null,
    selectedShardKeys: new Set(),
    connectionMode: 2
  });
}

/**
 * Deselect all entities.
 */
export function deselectAllAction() {
  store.setMultiple({
    selectedShardKey: null,
    selectedShardKeys: new Set(),
    selectedSocketKey: null,
    selectedRouteKey: null,
    connectionMode: 1
  });
}

/**
 * Drill-down into a specific shard.
 * @param {Object} shard 
 */
export function drillDownToShardAction(shard) {
  store.setMultiple({
    focusedLevelId: shard.orbit,
    selectedDeptName: shard.dept,
    focusedShardKey: shard.key,
    selectedShardKey: shard.key,
    selectedShardKeys: new Set([shard.key])
  });
}

/**
 * Focus on a specific department.
 * @param {string} deptName 
 * @param {number} orbit 
 */
export function focusDeptAction(deptName, orbit) {
  store.setMultiple({
    focusedLevelId: orbit,
    selectedDeptName: deptName,
    focusedShardKey: null,
    selectedShardKey: null,
    selectedShardKeys: new Set(),
    selectedSocketKey: null,
    selectedRouteKey: null
  });
}

/**
 * Focus on a specific level.
 * @param {number} levelId 
 */
export function focusLevelAction(levelId) {
  store.setMultiple({
    focusedLevelId: levelId,
    selectedDeptName: null,
    focusedShardKey: null,
    selectedShardKey: null,
    selectedShardKeys: new Set(),
    selectedSocketKey: null,
    selectedRouteKey: null
  });
}

/**
 * Drill-up from current focus level/dept/shard.
 */
export function drillUpAction() {
  const focusedShardKey = store.get('focusedShardKey');
  const selectedDeptName = store.get('selectedDeptName');
  const focusedLevelId = store.get('focusedLevelId');

  if (focusedShardKey) {
    store.set('focusedShardKey', null);
  } else if (selectedDeptName) {
    store.set('selectedDeptName', null);
  } else if (focusedLevelId !== null) {
    store.set('focusedLevelId', null);
  }
}

/**
 * Show a level by removing it from hidden levels set.
 * @param {number} levelId 
 */
export function showLevelAction(levelId) {
  const hiddenLevelIds = store.get('hiddenLevelIds') || new Set();
  if (hiddenLevelIds.has(levelId)) {
    const newHidden = new Set(hiddenLevelIds);
    newHidden.delete(levelId);
    store.set('hiddenLevelIds', newHidden);
  }
}

/**
 * Add a new shard to placement data.
 * @param {Object} newShard 
 */
export function addShardAction(newShard) {
  const placementData = store.get('placementData');
  if (placementData) {
    if (!placementData.shards) {
      placementData.shards = [];
    }
    placementData.shards.push(newShard);

    // Ensure the department exists in placementData.departments
    const deptName = newShard.dept;
    const orbitIndex = newShard.orbit;
    if (!placementData.departments) {
      placementData.departments = [];
    }
    const deptExists = placementData.departments.some(d => d.name === deptName && Number(d.orbit) === Number(orbitIndex));
    if (!deptExists) {
      placementData.departments.push({ name: deptName, orbit: orbitIndex });
    }

    store.setMultiple({
      placementData: { ...placementData },
      hasUnsavedChanges: true
    });
  }
}

/**
 * Update a shard's position and record the action in history.
 * @param {string} shardKey 
 * @param {Object} newPosition 
 */
export function updateShardPositionAction(shardKey, newPosition) {
  const placementData = store.get('placementData');
  if (!placementData) return;
  const shard = placementData.shards.find(s => s.key === shardKey);
  if (shard) {
    const undoState = JSON.parse(JSON.stringify(shard));
    shard.position = newPosition;

    store.setMultiple({
      placementData: { ...placementData },
      hasUnsavedChanges: true
    });

    emit(EVENTS.SHARD_TRANSFORMED, {
      key: shardKey,
      position: newPosition,
      size: shard.size
    });

    const redoState = JSON.parse(JSON.stringify(shard));
    historyManager.pushAction('move', 'shard', shardKey, `Перемещение шарда ${shardKey}`, undoState, redoState);
  }
}

/**
 * Update a socket's offset and entry_z, recording the action in history.
 * @param {string} socketKey 
 * @param {Object} newOffset 
 * @param {number|undefined} newEntryZ 
 */
export function updateSocketOffsetAction(socketKey, newOffset, newEntryZ) {
  const placementData = store.get('placementData');
  if (!placementData) return;
  const { shardKey, socketName } = parseSocketKey(socketKey);
  const shard = placementData.shards.find(s => s.key === shardKey);
  if (shard && shard.sockets) {
    const socket = shard.sockets.find(s => s.name === socketName);
    if (socket) {
      const undoState = JSON.parse(JSON.stringify(socket));
      socket.offset = newOffset;
      if (newEntryZ !== undefined) {
        socket.entry_z = newEntryZ;
      }

      store.setMultiple({
        placementData: { ...placementData },
        hasUnsavedChanges: true
      });

      const redoState = JSON.parse(JSON.stringify(socket));
      historyManager.pushAction('move', 'socket', socketKey, `Перемещение сокета ${socketName}`, undoState, redoState);
    }
  }
}

/**
 * Update a socket's dimensions in placementData.
 * @param {string} socketKey 
 * @param {number} width 
 * @param {number} height 
 * @param {number} pitch 
 * @param {Object} finalOffset 
 * @param {number} finalRotation 
 * @param {number} finalFaceSign 
 */
export function updateSocketDimensionsAction(socketKey, width, height, pitch, finalOffset, finalRotation, finalFaceSign) {
  const placementData = store.get('placementData');
  if (!placementData) return;
  const { shardKey, socketName } = parseSocketKey(socketKey);
  const shard = placementData.shards.find(s => s.key === shardKey);
  if (shard && shard.sockets) {
    const socket = shard.sockets.find(s => s.name === socketName);
    if (socket) {
      socket.width = width;
      socket.height = height;
      socket.pitch = pitch;
      socket.offset = finalOffset;
      socket.rotation = finalRotation;
      socket.faceSign = finalFaceSign;

      store.setMultiple({
        placementData: { ...placementData },
        hasUnsavedChanges: true
      });
    }
  }
}

/**
 * Update the active mode in store.
 * @param {string} mode 
 */
export function setActiveModeAction(mode) {
  store.set('activeMode', mode);
}
