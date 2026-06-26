import * as THREE from 'three';
import { scene, fitCameraToScene } from './viewer.js';
import { sceneManager } from './rendering/scene_manager.js';
import { store } from './store/store.js';
import { 
  initSharedResources, 
  makeTextSprite, 
  rebuildSocket
} from './rendering/mesh_factory.js';
import { THEME, RENDER_BINS } from './rendering/theme.js';
import { recomputeSpatialLayout, recomputeSpatialLayoutFromMeshes, levelAABBs, deptAABBs } from './algorithms/placement/spatial_manager.js';


// Stub drawRoutes since connections and routes are disabled in Composition mode
export function drawRoutes() {}

/**
 * Updates 3D meshes visibility based on store's hiddenLevelIds and soloLevelId.
 */
export function updateLevelsVisibility() {
  const data = store.get('placementData');
  if (!data) return;

  const hiddenLevelIds = store.get('hiddenLevelIds') || new Set();
  const focusedLevelId = store.get('focusedLevelId');
  const selectedDeptName = store.get('selectedDeptName');
  const selectedShardKey = store.get('selectedShardKey');

  // Determine active level ID from selected shard/dept if level is not explicitly focused
  let activeLvlId = focusedLevelId;
  if (activeLvlId === null && selectedShardKey) {
    const shard = data.shards.find(s => s.key === selectedShardKey);
    if (shard) activeLvlId = shard.orbit;
  }
  if (activeLvlId === null && selectedDeptName) {
    const dept = data.departments.find(d => d.name === selectedDeptName);
    if (dept) activeLvlId = dept.orbit;
  }
  
  // Determine active department name from selected shard if not explicitly selected
  let activeDeptName = selectedDeptName;
  if (activeDeptName === null && selectedShardKey) {
    const shard = data.shards.find(s => s.key === selectedShardKey);
    if (shard) activeDeptName = shard.dept;
  }

  const isLevelFocused = activeLvlId !== null;
  const isDeptFocused = activeDeptName !== null;
  const isShardFocused = selectedShardKey !== null;

  // 1. Level wireframe visibility & opacity
  if (levelsGroup) {
    levelsGroup.children.forEach(lvlMesh => {
      const lvlId = lvlMesh.userData?.levelId;
      if (lvlId !== undefined) {
        const isHidden = hiddenLevelIds.has(lvlId);
        if (isHidden) {
          lvlMesh.visible = false;
          return;
        }

        lvlMesh.visible = true;

        if (!lvlMesh.userData.originalColor) {
          lvlMesh.userData.originalColor = lvlMesh.material.color.clone();
        }

        const isCurrentLevel = (Number(lvlId) === Number(activeLvlId));

        if (isCurrentLevel) {
          lvlMesh.material.color.copy(lvlMesh.userData.originalColor);
          lvlMesh.material.opacity = THEME.levelWireframe.activeOpacity; // 0.85
        } else {
          lvlMesh.material.color.setHex(0x555555); // серый
          if (isShardFocused) {
            lvlMesh.material.opacity = 0.05; // 5%
          } else if (isDeptFocused) {
            lvlMesh.material.opacity = 0.2; // 20%
          } else if (isLevelFocused) {
            lvlMesh.material.opacity = 0.5; // 50%
          } else {
            lvlMesh.material.color.copy(lvlMesh.userData.originalColor);
            lvlMesh.material.opacity = THEME.levelWireframe.defaultOpacity;
          }
        }
        lvlMesh.material.transparent = true;
        lvlMesh.material.needsUpdate = true;
      }
    });
  }

  // 2. Department boundary visibility & opacity
  if (deptsGroup) {
    deptsGroup.children.forEach(deptMesh => {
      const lvlId = deptMesh.userData?.orbit;
      const deptName = deptMesh.userData?.name;
      if (lvlId !== undefined) {
        const isHidden = hiddenLevelIds.has(lvlId);
        if (isHidden) {
          deptMesh.visible = false;
          return;
        }

        deptMesh.visible = true;

        if (!deptMesh.userData.originalColor) {
          deptMesh.userData.originalColor = deptMesh.material.color.clone();
        }

        const isCurrentLevel = (Number(lvlId) === Number(activeLvlId));
        const isCurrentDept = (deptName === activeDeptName);

        if (isCurrentDept) {
          deptMesh.material.color.copy(deptMesh.userData.originalColor);
          deptMesh.material.opacity = THEME.deptWireframe.selectedOpacity; // 0.9
        } else if (isCurrentLevel) {
          // Other depts on active level
          deptMesh.material.color.setHex(0x555555); // серый
          if (isShardFocused) {
            deptMesh.material.opacity = 0.2; // 20%
          } else if (isDeptFocused) {
            deptMesh.material.opacity = 0.5; // 50%
          } else {
            // Level focused but no active dept
            deptMesh.material.color.copy(deptMesh.userData.originalColor);
            deptMesh.material.opacity = THEME.deptWireframe.activeOpacity; // 0.7
          }
        } else {
          // Depts on inactive levels
          deptMesh.material.color.setHex(0x555555); // серый
          if (isShardFocused) {
            deptMesh.material.opacity = 0.05; // 5%
          } else if (isDeptFocused) {
            deptMesh.material.opacity = 0.2; // 20%
          } else if (isLevelFocused) {
            deptMesh.material.opacity = 0.5; // 50%
          } else {
            // No focus at all
            deptMesh.material.color.copy(deptMesh.userData.originalColor);
            deptMesh.material.opacity = THEME.deptWireframe.defaultOpacity;
          }
        }
        deptMesh.material.transparent = true;
        deptMesh.material.needsUpdate = true;
      }
    });
  }
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
 * Creates a THREE.Group representation of a shard with body, wireframe, label, layers and dividers.
 * @param {any} sd 
 * @returns {THREE.Group}
 */
export function createShard3D(sd) {
  const color = 0x6366f1; // Single beautiful Indigo theme color

  // Position is center in Three.js coordinates
  const x = (sd.position.x + sd.size.w / 2) * VIS_SCALE;
  const y = (sd.position.y + sd.size.h / 2) * VIS_SCALE;
  const z = (sd.position.z + sd.size.d / 2) * VIS_SCALE;

  const w = sd.size.w * VIS_SCALE;
  const d = sd.size.d * VIS_SCALE; // Three.js D (depth)
  const h = sd.size.h * VIS_SCALE; // Three.js H (height)

  const shardGroup = new THREE.Group();
  shardGroup.position.set(x, y, z);
  shardGroup.name = `shard_${sd.key}`;

  // Body mesh
  const geo = new THREE.BoxGeometry(w, h, d);
  const mat = new THREE.MeshStandardMaterial({
    color, transparent: false, opacity: 1.0,
    roughness: 0.6, metalness: 0.1,
  });
  const boxMesh = new THREE.Mesh(geo, mat);
  boxMesh.name = "body";
  shardGroup.add(boxMesh);

  // Wireframe
  const edgeGeo = new THREE.EdgesGeometry(geo);
  const edgeMat = new THREE.LineBasicMaterial({
    color, transparent: true, opacity: 0.85,
  });
  const mainWire = new THREE.LineSegments(edgeGeo, edgeMat);
  mainWire.name = "main_wireframe";
  shardGroup.add(mainWire);

  // Text label
  const label = makeTextSprite(sd.key, color);
  label.position.set(0, h / 2 + 1.5, 0);
  shardGroup.add(label);

  // Track key references in group's userData
  shardGroup.userData = {
    label,
    originalColor: color,
    body: boxMesh,
    mainWire: mainWire,
    shardKey: sd.key
  };

  // Build internal layers
  const layers = sd.layers && sd.layers.length > 0
    ? sd.layers
    : [{ name: "default", height_pct: 1.0, density: 1.0 }];

  let currentY = -h / 2;
  layers.forEach((layer, idx) => {
    const layer_vis_h = h * layer.height_pct;
    // Micro-gap to prevent Z-fighting at layer boundaries
    const layer_draw_h = Math.max(0.01 * VIS_SCALE, layer_vis_h - 0.02 * VIS_SCALE);
    const layerGeo = new THREE.BoxGeometry(w, 1.0, d);
    
    const layerColor = new THREE.Color(color);
    if (idx % 2 === 1) {
      layerColor.offsetHSL(0, 0, -0.08);
    } else {
      layerColor.offsetHSL(0, 0, 0.04);
    }

    const layerMat = new THREE.MeshStandardMaterial({
      color: layerColor,
      transparent: true,
      opacity: 0.5,
      roughness: 0.15,
      metalness: 0.1,
    });

    const layerMesh = new THREE.Mesh(layerGeo, layerMat);
    const yCenter = currentY + layer_vis_h / 2;
    layerMesh.position.set(0, yCenter, 0);
    layerMesh.scale.set(1.0, layer_draw_h, 1.0);
    layerMesh.visible = false;
    layerMesh.renderOrder = RENDER_BINS.activeLayers + idx;

    layerMesh.userData = {
      layerName: layer.name,
      height_pct: layer.height_pct,
      layerIndex: idx,
      shardKey: sd.key
    };

    shardGroup.add(layerMesh);

    const layerEdgeGeo = new THREE.EdgesGeometry(layerGeo);
    const layerEdgeMat = new THREE.LineBasicMaterial({
      color: layerColor,
      transparent: true,
      opacity: 0.8,
      polygonOffset: true,
      polygonOffsetFactor: -1,
      polygonOffsetUnits: -1
    });
    const layerWireframe = new THREE.LineSegments(layerEdgeGeo, layerEdgeMat);
    layerWireframe.name = "wireframe";
    layerMesh.add(layerWireframe);

    currentY += layer_vis_h;
  });

  // Build layer dividers
  if (layers.length >= 2) {
    let accumY = -h / 2;
    for (let i = 0; i < layers.length - 1; i++) {
      accumY += h * layers[i].height_pct;

      const divGeo = new THREE.PlaneGeometry(w * 1.02, d * 1.02);
      const divMat = new THREE.MeshBasicMaterial({
        color: 0xffaa00,
        transparent: true,
        opacity: 0.0,
        side: THREE.DoubleSide,
        depthWrite: false,
        polygonOffset: true,
        polygonOffsetFactor: -1,
        polygonOffsetUnits: -1
      });
      const divMesh = new THREE.Mesh(divGeo, divMat);
      divMesh.position.set(0, accumY, 0);
      divMesh.rotation.x = -Math.PI / 2;
      divMesh.name = `divider_${i}`;
      divMesh.userData = {
        isDivider: true,
        dividerIndex: i,
        shardKey: sd.key
      };

      const borderGeo = new THREE.BufferGeometry().setFromPoints([
        new THREE.Vector3(-w/2, -d/2, 0),
        new THREE.Vector3(w/2, -d/2, 0),
        new THREE.Vector3(w/2, d/2, 0),
        new THREE.Vector3(-w/2, d/2, 0),
        new THREE.Vector3(-w/2, -d/2, 0)
      ]);
      const borderMat = new THREE.LineBasicMaterial({
        color: 0xffaa00,
        transparent: true,
        opacity: 0.3,
        polygonOffset: true,
        polygonOffsetFactor: -1,
        polygonOffsetUnits: -1
      });
      const divBorder = new THREE.Line(borderGeo, borderMat);
      divBorder.name = "border";
      divMesh.add(divBorder);

      divMesh.visible = false;
      shardGroup.add(divMesh);
    }
  }

  return shardGroup;
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

    const lvlColor = new THREE.Color(lvl.color || "#ffffff");
    const mat = new THREE.LineBasicMaterial({
      color: lvlColor,
      transparent: true,
      opacity: 0.18,
    });
    const wire = new THREE.LineSegments(unitEdgeGeo, mat);
    wire.position.set(box.x, box.y, box.z);
    wire.scale.set(box.w, box.h, box.d);
    wire.raycast = () => {};
    wire.renderOrder = RENDER_BINS.wireframes;
    wire.userData = { levelId: lvl.id };
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

    const deptGeo = unitEdgeGeo.clone();

    const mat = new THREE.LineDashedMaterial({
      color: 0x8b949e,
      dashSize: 0.8 * VIS_SCALE,
      gapSize: 0.4 * VIS_SCALE,
      transparent: true,
      opacity: 0.25
    });
    const wire = new THREE.LineSegments(deptGeo, mat);
    wire.position.set(box.x, box.y, box.z);
    wire.scale.set(box.w, box.h, box.d);
    wire.computeLineDistances();
    wire.raycast = () => {};
    wire.renderOrder = RENDER_BINS.wireframes;
    wire.userData = { orbit: dept.orbit, name: dept.name };
    deptsGroup.add(wire);
    deptsMeshes.set(dept.name, wire);
  });

  // Build shards
  data.shards.forEach(sd => {
    const shardGroup = createShard3D(sd);
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

  const shardGroup = createShard3D(shardData);
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

  const w = size.w * VIS_SCALE;
  const h = size.h * VIS_SCALE;
  const d = size.d * VIS_SCALE;

  const x = (position.x + size.w / 2) * VIS_SCALE;
  const y = (position.y + size.h / 2) * VIS_SCALE;
  const z = (position.z + size.d / 2) * VIS_SCALE;

  // Move the container group (automatically shifts children)
  shardGroup.position.set(x, y, z);

  // Resize body BoxGeometry
  const boxMesh = shardGroup.userData.body;
  if (boxMesh) {
    boxMesh.geometry.dispose();
    boxMesh.geometry = new THREE.BoxGeometry(w, h, d);
  }

  // Resize main wireframe
  const mainWire = shardGroup.userData.mainWire;
  if (mainWire && boxMesh) {
    mainWire.geometry.dispose();
    mainWire.geometry = new THREE.EdgesGeometry(boxMesh.geometry);
  }

  // Adjust label position
  const label = shardGroup.userData.label;
  if (label) {
    label.position.set(0, h / 2 + 1.5, 0);
  }

  // Scale and shift horizontal layers
  const layerMeshes = [];
  shardGroup.children.forEach(child => {
    if (child.userData && child.userData.layerIndex !== undefined) {
      layerMeshes.push(child);
    }
  });
  layerMeshes.sort((a, b) => a.userData.layerIndex - b.userData.layerIndex);

  let currentY = -h / 2;
  layerMeshes.forEach(layerMesh => {
    const layer_vis_h = h * layerMesh.userData.height_pct;
    const yCenter = currentY + layer_vis_h / 2;
    // Micro-gap to prevent Z-fighting at layer boundaries
    const layer_draw_h = Math.max(0.01 * VIS_SCALE, layer_vis_h - 0.02 * VIS_SCALE);

    layerMesh.position.set(0, yCenter, 0);
    layerMesh.scale.set(1.0, layer_draw_h, 1.0);

    layerMesh.geometry.dispose();
    layerMesh.geometry = new THREE.BoxGeometry(w, 1.0, d);

    const wire = layerMesh.children.find(c => c.name === "wireframe");
    if (wire) {
      wire.geometry.dispose();
      wire.geometry = new THREE.EdgesGeometry(layerMesh.geometry);
    }

    currentY += layer_vis_h;
  });

  // Scale and shift dividers
  const dividers = [];
  shardGroup.children.forEach(child => {
    if (child.userData && child.userData.isDivider) {
      dividers.push(child);
    }
  });
  dividers.sort((a, b) => a.userData.dividerIndex - b.userData.dividerIndex);

  let accumY = -h / 2;
  dividers.forEach((divMesh, idx) => {
    accumY += h * layerMeshes[idx].userData.height_pct;
    divMesh.position.set(0, accumY, 0);

    divMesh.geometry.dispose();
    divMesh.geometry = new THREE.PlaneGeometry(w * 1.02, d * 1.02);

    const border = divMesh.children.find(c => c.name === "border");
    if (border) {
      border.geometry.dispose();
      border.geometry = new THREE.BufferGeometry().setFromPoints([
        new THREE.Vector3(-w / 2, -d / 2, 0),
        new THREE.Vector3(w / 2, -d / 2, 0),
        new THREE.Vector3(w / 2, d/2, 0),
        new THREE.Vector3(-w / 2, d/2, 0),
        new THREE.Vector3(-w / 2, -d / 2, 0)
      ]);
    }
  });

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

  const x = (position.x + sd.size.w / 2) * VIS_SCALE;
  const y = (position.y + sd.size.h / 2) * VIS_SCALE;
  const z = (position.z + sd.size.d / 2) * VIS_SCALE;

  shardGroup.position.set(x, y, z);

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
  if (!levelsGroup || !deptsGroup) return;

  const placement = store.get('placementData');
  if (!placement) return;

  const levels = placement.levels || [];
  const depts = placement.departments || [];

  // Compute boundaries dynamically based on current mesh positions
  recomputeSpatialLayoutFromMeshes(shardMeshes, shardDataMap, levels, depts, VIS_SCALE);

  // Adjust level wireframe scales
  levels.forEach(lvl => {
    const box = levelAABBs.get(lvl.id);
    const wire = levelsMeshes.get(lvl.id);
    if (!box || !wire) {
      if (wire) wire.visible = false;
      return;
    }

    wire.position.set(box.x, box.y, box.z);
    wire.scale.set(box.w, box.h, box.d);
    wire.visible = true;
  });

  // Adjust department wireframe scales and line distances
  depts.forEach(dept => {
    const key = `${dept.name}@${dept.orbit}`;
    const box = deptAABBs.get(key);
    const wire = deptsMeshes.get(dept.name);
    if (!box || !wire) {
      if (wire) wire.visible = false;
      return;
    }

    wire.position.set(box.x, box.y, box.z);
    wire.scale.set(box.w, box.h, box.d);
    wire.computeLineDistances();
    wire.visible = true;
  });
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
