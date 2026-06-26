import * as THREE from 'three';
import { scene, fitCameraToScene } from './viewer.js';
import { sceneManager } from './rendering/scene_manager.js';
import { store } from './store/store.js';
import { 
  initSharedResources, 
  rebuildSocket
} from './rendering/mesh_factory.js';
import { THEME, RENDER_BINS } from './rendering/theme.js';
import { recomputeSpatialLayout, levelAABBs, deptAABBs } from './algorithms/placement/spatial_manager.js';
import { 
  createShard3D, 
  updateShardTransform as updateShardTransform3D, 
  updateShardDragging as updateShardDragging3D 
} from './rendering/shard_renderer.js';
import { 
  createLevelWire, 
  createDeptWire, 
  updateLevelsVisibility as updateStructVisibility, 
  updateContainerWires as updateStructWires 
} from './rendering/structure_renderer.js';

// Stub drawRoutes since connections and routes are disabled in Composition mode
export function drawRoutes() {}

/**
 * Updates 3D meshes visibility based on store's hiddenLevelIds and soloLevelId.
 */
export function updateLevelsVisibility() {
  const data = store.get('placementData');
  const hiddenLevelIds = store.get('hiddenLevelIds') || new Set();
  const focusedLevelId = store.get('focusedLevelId');
  const selectedDeptName = store.get('selectedDeptName');
  const selectedShardKey = store.get('selectedShardKey');

  updateStructVisibility(
    levelsGroup,
    deptsGroup,
    hiddenLevelIds,
    focusedLevelId,
    selectedDeptName,
    selectedShardKey,
    data
  );
}

// Scene elements tracking (bound to SceneManager collections)
export const shardMeshes = sceneManager.shardMeshes;
export const shardDataMap = sceneManager.shardDataMap;
export const socketMeshes = sceneManager.socketMeshes;
export const shardsByLevel = sceneManager.shardsByLevel;
export const shardsByDept = sceneManager.shardsByDept;
export const socketsByLevel = sceneManager.socketsByLevel;
export const socketsByDept = sceneManager.socketsByDept;
export const levelsMeshes = sceneManager.levelsMeshes;
export const deptsMeshes = sceneManager.deptsMeshes;

export let VIS_SCALE = 1.0;

export const SOMA_COLORS = {
  "bio/sensory/photoreceptor": 0x38bdf8,
  "bio/sensory/hair_cell": 0xfb7185,
  "bio/cortex/pyramidal_exc": 0x34d399,
  "bio/cortex/basket_inh": 0xfbbf24,
  "bio/motor/purkinje": 0xa78bfa,
  "bio/motor/spinal_motor": 0xf472b6
};

// Reusable groups bound to SceneManager
const shardsGroup = sceneManager.shardsGroup;
const levelsGroup = sceneManager.levelsGroup;
const deptsGroup = sceneManager.deptsGroup;

// Re-export rebuildSocket for consumer modules
export { rebuildSocket };

let unitGeo = null;
let unitEdgeGeo = null;

function initUnitGeometry() {
  if (!unitGeo) {
    unitGeo = new THREE.BoxGeometry(1, 1, 1);
    unitEdgeGeo = new THREE.EdgesGeometry(unitGeo);
  }
}

/**
 * Recursively disposes geometries, materials, and textures within a Three.js hierarchy.
 * @param {THREE.Object3D} obj
 */
export function disposeHierarchy(obj) {
  sceneManager.disposeHierarchy(obj);
}

/**
 * Builds the 3D visual scene objects (shards, levels, and departments) from placement data.
 * @param {import("./contracts/types.js").PlacementData} data 
 */
export function buildSceneData(data, preserveCamera = false) {
  // Clear any existing groups to avoid duplication and GPU leaks
  sceneManager.clearScene();
  
  // Calculate dynamic VIS_SCALE from shards bounding box to fit the camera cleanly
  let maxCoord = 1.0;
  if (data.shards && data.shards.length > 0) {
    data.shards.forEach(sd => {
      maxCoord = Math.max(
        maxCoord, 
        Math.abs(sd.position.x) + sd.size.w, 
        Math.abs(sd.position.z) + sd.size.d, // Three.js Z (depth)
        Math.abs(sd.position.y) + sd.size.h  // Three.js Y (height)
      );
    });
  }
  VIS_SCALE = 35.0 / Math.max(maxCoord, 1.0);
  sceneManager.visScale = VIS_SCALE;
  store.set('visScale', VIS_SCALE);
  initSharedResources(VIS_SCALE);
  initUnitGeometry();

  const levels = data.levels || [];
  const depts = data.departments || [];
  const levelsMap = new Map();
  levels.forEach(lvl => {
    levelsMap.set(Number(lvl.id), lvl);
  });

  // Calculate dynamic AABBs for levels and departments using spatial_manager
  recomputeSpatialLayout(data, VIS_SCALE);

  // Draw 3D bounds for Levels (using single unitEdgeGeo box scaled)
  levels.forEach(lvl => {
    const box = levelAABBs.get(lvl.id);
    if (!box) return;

    const wire = createLevelWire(lvl, box, unitEdgeGeo);
    levelsGroup.add(wire);
    levelsMeshes.set(lvl.id, wire);
  });

  // Draw 3D bounds for Departments (using cloned unitEdgeGeo boxes)
  depts.forEach(dept => {
    const lvl = levelsMap.get(dept.orbit);
    if (!lvl) return;

    const key = `${dept.name}@${dept.orbit}`;
    const box = deptAABBs.get(key);
    if (!box) return;

    const wire = createDeptWire(dept, box, unitEdgeGeo, VIS_SCALE);
    deptsGroup.add(wire);
    deptsMeshes.set(dept.name, wire);
  });

  // Build shards
  data.shards.forEach(sd => {
    const shardGroup = createShard3D(sd, VIS_SCALE);
    shardsGroup.add(shardGroup);

    // Track for rendering and raycasting
    shardMeshes.set(sd.key, shardGroup);
    shardDataMap.set(shardGroup.uuid, sd);

    // Cache in flat maps
    if (!shardsByLevel.has(sd.orbit)) shardsByLevel.set(sd.orbit, []);
    shardsByLevel.get(sd.orbit).push(shardGroup);

    if (!shardsByDept.has(sd.dept)) shardsByDept.set(sd.dept, []);
    shardsByDept.get(sd.dept).push(shardGroup);
  });

  // Compute auto-camera fit
  const bbox = new THREE.Box3();
  data.shards.forEach(sd => {
    const minX = sd.position.x * VIS_SCALE;
    const minY = sd.position.y * VIS_SCALE;
    const minZ = sd.position.z * VIS_SCALE;

    const maxX = (sd.position.x + sd.size.w) * VIS_SCALE;
    const maxY = (sd.position.y + sd.size.h) * VIS_SCALE;
    const maxZ = (sd.position.z + sd.size.d) * VIS_SCALE;

    bbox.expandByPoint(new THREE.Vector3(minX, minY, minZ));
    bbox.expandByPoint(new THREE.Vector3(maxX, maxY, maxZ));
  });

  if (!preserveCamera) {
    fitCameraToScene(bbox);
  }

  // Apply hidden/solo visibility filters
  updateLevelsVisibility();
}

/**
 * Adds a new shard to the 3D scene incrementally.
 * @param {any} shardData 
 */
export function addShard(shardData) {
  if (!shardsGroup) return;

  const shardGroup = createShard3D(shardData, VIS_SCALE);
  shardsGroup.add(shardGroup);

  const key = shardData.key;
  shardMeshes.set(key, shardGroup);
  shardDataMap.set(shardGroup.uuid, shardData);

  if (!shardsByLevel.has(shardData.orbit)) shardsByLevel.set(shardData.orbit, []);
  shardsByLevel.get(shardData.orbit).push(shardGroup);

  if (!shardsByDept.has(shardData.dept)) shardsByDept.set(shardData.dept, []);
  shardsByDept.get(shardData.dept).push(shardGroup);

  updateLevelsVisibility();
  updateContainerWires();
}

/**
 * Removes a shard from the 3D scene incrementally.
 * @param {string} shardKey 
 */
export function deleteShard(shardKey) {
  const shardGroup = shardMeshes.get(shardKey);
  if (!shardGroup) return;

  disposeHierarchy(shardGroup);
  if (shardsGroup) {
    shardsGroup.remove(shardGroup);
  }

  const uuid = shardGroup.uuid;
  shardMeshes.delete(shardKey);
  shardDataMap.delete(uuid);

  for (const [lvlId, arr] of shardsByLevel.entries()) {
    shardsByLevel.set(lvlId, arr.filter(m => m !== shardGroup));
  }
  for (const [deptName, arr] of shardsByDept.entries()) {
    shardsByDept.set(deptName, arr.filter(m => m !== shardGroup));
  }

  updateLevelsVisibility();
  updateContainerWires();
}

/**
 * Updates a shard's geometry and internal layers/dividers locally in real-time.
 * @param {{ key: string, position: any, size: any }} payload 
 */
export function updateShardTransform({ key, position, size }) {
  const shardGroup = shardMeshes.get(key);
  if (!shardGroup) return;

  updateShardTransform3D(shardGroup, position, size, VIS_SCALE);

  // Sync back into store.placementData
  const placementData = store.get('placementData');
  if (placementData) {
    const shard = placementData.shards.find(s => s.key === key);
    if (shard) {
      shard.position = JSON.parse(JSON.stringify(position));
      shard.size = JSON.parse(JSON.stringify(size));
    }
  }

  updateContainerWires();
}

/**
 * Updates a shard's position during active drag without altering store state.
 * @param {{ key: string, position: any }} payload 
 */
export function updateShardDragging({ key, position }) {
  const shardGroup = shardMeshes.get(key);
  if (!shardGroup) return;

  const sd = shardDataMap.get(shardGroup.uuid);
  if (!sd) return;

  updateShardDragging3D(shardGroup, position, sd, VIS_SCALE);

  updateContainerWires();
}

export function updateAllSocketVisuals() {
  // Stub - sockets are disabled in Composition mode
}

/**
 * Re-computes and updates the visual wireframe boxes of Levels and Departments.
 * Called dynamically during object transformation/gizmo drag.
 */
export function updateContainerWires() {
  const placement = store.get('placementData');
  updateStructWires(
    levelsGroup,
    deptsGroup,
    levelsMeshes,
    deptsMeshes,
    placement,
    shardMeshes,
    shardDataMap,
    VIS_SCALE
  );
}

// Self-subscribe to store changes for rendering visibility/opacity of containers
store.on('focusedLevelId', () => {
  updateLevelsVisibility();
});
store.on('hiddenLevelIds', () => {
  updateLevelsVisibility();
});
store.on('selectedDeptName', () => {
  updateLevelsVisibility();
});
store.on('selectedShardKey', () => {
  updateLevelsVisibility();
});

// Subscribe to delta events for incremental rendering
import('./store/event_bus.js').then(({ on, EVENTS }) => {
  on(EVENTS.SHARD_ADDED, (sd) => {
    addShard(sd);
  });
  on(EVENTS.SHARD_DELETED, (key) => {
    deleteShard(key);
  });
  on(EVENTS.SHARD_DRAGGING, (payload) => {
    updateShardDragging(payload);
  });
  on(EVENTS.SHARD_TRANSFORMED, (payload) => {
    updateShardTransform(payload);
  });
});
