import * as THREE from 'three';
import { scene, fitCameraToScene } from './viewer.js';
import { store } from './store/store.js';
import { 
  initSharedResources, 
  makeTextSprite, 
  rebuildSocket
} from './rendering/mesh_factory.js';
import { THEME } from './rendering/theme.js';

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

        if (focusedLevelId !== null) {
          const isCurrentLevel = (lvlId === focusedLevelId);
          lvlMesh.material.transparent = true;
          lvlMesh.material.opacity = isCurrentLevel ? THEME.levelWireframe.activeOpacity : THEME.levelWireframe.inactiveOpacity;
        } else {
          lvlMesh.material.transparent = true;
          lvlMesh.material.opacity = THEME.levelWireframe.defaultOpacity;
        }
        lvlMesh.material.needsUpdate = true;
      }
    });
  }

  // 2. Department boundary visibility & opacity
  if (deptsGroup) {
    deptsGroup.children.forEach(deptMesh => {
      const lvlId = deptMesh.userData?.orbit;
      if (lvlId !== undefined) {
        const isHidden = hiddenLevelIds.has(lvlId);
        if (isHidden) {
          deptMesh.visible = false;
          return;
        }

        deptMesh.visible = true;

        if (focusedLevelId !== null) {
          const isCurrentLevel = (lvlId === focusedLevelId);
          deptMesh.material.transparent = true;
          deptMesh.material.opacity = isCurrentLevel ? THEME.deptWireframe.activeOpacity : THEME.deptWireframe.inactiveOpacity;
        } else {
          deptMesh.material.transparent = true;
          deptMesh.material.opacity = THEME.deptWireframe.defaultOpacity;
        }
        deptMesh.material.needsUpdate = true;
      }
    });
  }
}

// Scene elements tracking
export let shardMeshes = {};        // key -> mesh
export let shardDataMap = {};       // mesh.uuid -> raw data
export let socketMeshes = {};       // socketKey -> THREE.Group containing instanced mesh & backing
export let VIS_SCALE = 1.0;

export const SOMA_COLORS = {
  "bio/sensory/photoreceptor": 0x38bdf8,
  "bio/sensory/hair_cell": 0xfb7185,
  "bio/cortex/pyramidal_exc": 0x34d399,
  "bio/cortex/basket_inh": 0xfbbf24,
  "bio/motor/purkinje": 0xa78bfa,
  "bio/motor/spinal_motor": 0xf472b6
};

// Reusable groups to prevent memory leaks and duplicate objects
let shardsGroup = null;
let levelsGroup = null;
let deptsGroup = null;

// Re-export rebuildSocket for consumer modules
export { rebuildSocket };

/**
 * Builds the 3D visual scene objects (shards, levels, and departments) from placement data.
 * @param {import("./contracts/types.js").PlacementData} data 
 */
export function buildSceneData(data, preserveCamera = false) {
  // Clear any existing groups to avoid duplication
  if (shardsGroup) scene.remove(shardsGroup);
  if (levelsGroup) scene.remove(levelsGroup);
  if (deptsGroup) scene.remove(deptsGroup);

  shardsGroup = new THREE.Group();
  levelsGroup = new THREE.Group();
  deptsGroup = new THREE.Group();

  scene.add(shardsGroup);
  scene.add(levelsGroup);
  scene.add(deptsGroup);

  shardMeshes = {};
  shardDataMap = {};
  socketMeshes = {};
  
  // Calculate dynamic VIS_SCALE from shards bounding box to fit the camera cleanly
  let maxCoord = 1.0;
  if (data.shards && data.shards.length > 0) {
    data.shards.forEach(sd => {
      maxCoord = Math.max(
        maxCoord, 
        Math.abs(sd.position.x) + sd.size.w, 
        Math.abs(sd.position.y) + sd.size.d, 
        Math.abs(sd.position.z) + sd.size.h
      );
    });
  }
  VIS_SCALE = 35.0 / Math.max(maxCoord, 1.0);
  initSharedResources(VIS_SCALE);

  const levels = data.levels || [];
  const depts = data.departments || [];
  const levelsMap = {};
  levels.forEach(lvl => {
    levelsMap[lvl.id] = lvl;
  });

  // Calculate dynamic AABB for each level based on its shards
  const levelAABB = {}; // levelId -> { xMin, xMax, yMin, yMax }
  if (data.shards) {
    data.shards.forEach(sd => {
      const lvlId = sd.orbit;
      if (!levelAABB[lvlId]) {
        levelAABB[lvlId] = {
          xMin: sd.position.x,
          xMax: sd.position.x + sd.size.w,
          yMin: sd.position.y,
          yMax: sd.position.y + sd.size.d
        };
      } else {
        const box = levelAABB[lvlId];
        box.xMin = Math.min(box.xMin, sd.position.x);
        box.xMax = Math.max(box.xMax, sd.position.x + sd.size.w);
        box.yMin = Math.min(box.yMin, sd.position.y);
        box.yMax = Math.max(box.yMax, sd.position.y + sd.size.d);
      }
    });
  }

  // Draw 3D bounds for Levels (as thin wireframe boxes)
  levels.forEach(lvl => {
    const box = levelAABB[lvl.id];
    if (!box) return; // Skip empty levels

    const x = (box.xMin + box.xMax) / 2 * VIS_SCALE;
    const y = (lvl.z_start + lvl.height / 2) * VIS_SCALE;
    const z = (box.yMin + box.yMax) / 2 * VIS_SCALE;

    const w = (box.xMax - box.xMin) * VIS_SCALE;
    const h = lvl.height * VIS_SCALE;
    const d = (box.yMax - box.yMin) * VIS_SCALE;

    const geo = new THREE.BoxGeometry(w, h, d);
    const edgeGeo = new THREE.EdgesGeometry(geo);
    const lvlColor = new THREE.Color(lvl.color || "#ffffff");
    const mat = new THREE.LineBasicMaterial({
      color: lvlColor,
      transparent: true,
      opacity: 0.18,
    });
    const wire = new THREE.LineSegments(edgeGeo, mat);
    wire.position.set(x, y, z);
    wire.raycast = () => {}; // Disable raycasting interaction
    wire.userData = { levelId: lvl.id };
    levelsGroup.add(wire);
  });

  // Draw 3D bounds for Departments (as dynamic dashed wireframe boxes)
  depts.forEach(dept => {
    const lvl = levelsMap[dept.orbit];
    if (!lvl) return;

    const x = (dept.position.x + dept.size.w / 2) * VIS_SCALE;
    const y = (lvl.z_start + lvl.height / 2) * VIS_SCALE;
    const z = (dept.position.y + dept.size.d / 2) * VIS_SCALE;

    const w = dept.size.w * VIS_SCALE;
    const h = lvl.height * VIS_SCALE;
    const d = dept.size.d * VIS_SCALE;

    const geo = new THREE.BoxGeometry(w, h, d);
    const edgeGeo = new THREE.EdgesGeometry(geo);
    const mat = new THREE.LineDashedMaterial({
      color: 0x8b949e, // Neutral gray dashed line
      dashSize: 0.8 * VIS_SCALE,
      gapSize: 0.4 * VIS_SCALE,
      transparent: true,
      opacity: 0.25
    });
    const wire = new THREE.LineSegments(edgeGeo, mat);
    wire.computeLineDistances();
    wire.position.set(x, y, z);
    wire.raycast = () => {}; // Disable raycasting interaction
    wire.userData = { orbit: dept.orbit };
    deptsGroup.add(wire);
  });

  // 2. Build shards nested inside shardsGroup in global space
  data.shards.forEach(sd => {
    const color = 0x6366f1; // Single beautiful Indigo theme color by default or HSL tailored
    
    // Convert AABB min corner coordinates to geometry center in Three.js coordinates
    // Three X = Rust X, Three Y = Rust Z (height), Three Z = Rust Y (depth)
    const x = (sd.position.x + sd.size.w / 2) * VIS_SCALE;
    const y = (sd.position.z + sd.size.h / 2) * VIS_SCALE; 
    const z = (sd.position.y + sd.size.d / 2) * VIS_SCALE; 
    
    const w = sd.size.w * VIS_SCALE;
    const d = sd.size.d * VIS_SCALE;
    const h = sd.size.h * VIS_SCALE;

    // Shard main mesh
    const geo = new THREE.BoxGeometry(w, h, d);
    const mat = new THREE.MeshStandardMaterial({
      color, transparent: false, opacity: 1.0,
      roughness: 0.6, metalness: 0.1,
    });
    const mesh = new THREE.Mesh(geo, mat);
    mesh.position.set(x, y, z);
    mesh.rotation.set(0, 0, 0);

    shardsGroup.add(mesh);

    // Monolith wireframe (visible when not selected)
    const edgeGeo = new THREE.EdgesGeometry(geo);
    const edgeMat = new THREE.LineBasicMaterial({
      color, transparent: true, opacity: 0.85,
    });
    const mainWire = new THREE.LineSegments(edgeGeo, edgeMat);
    mainWire.name = "main_wireframe";
    mesh.add(mainWire);

    // Label added as child of mesh
    const label = makeTextSprite(sd.key, color);
    label.position.set(0, h / 2 + 1.5, 0); // local Y height axis
    mesh.add(label);

    // Track for rendering and raycasting
    shardMeshes[sd.key] = mesh;
    shardDataMap[mesh.uuid] = sd;

    // Store references in mesh.userData
    mesh.userData = { label, originalColor: color };

    // Build layers as child meshes of the main shard mesh
    const layers = sd.layers && sd.layers.length > 0
      ? sd.layers
      : [{ name: "default", height_pct: 1.0, density: 1.0 }];

    let currentY = -h / 2;
    layers.forEach((layer, idx) => {
      const layer_vis_h = h * layer.height_pct;
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
      layerMesh.scale.set(1.0, layer_vis_h, 1.0);
      layerMesh.visible = false;

      layerMesh.userData = {
        layerName: layer.name,
        height_pct: layer.height_pct,
        layerIndex: idx,
        shardKey: sd.key
      };

      mesh.add(layerMesh);

      // Wired border for the layer
      const edgeGeo = new THREE.EdgesGeometry(layerGeo);
      const edgeMat = new THREE.LineBasicMaterial({
        color: layerColor,
        transparent: true,
        opacity: 0.8,
      });
      const layerWireframe = new THREE.LineSegments(edgeGeo, edgeMat);
      layerWireframe.name = "wireframe";
      layerMesh.add(layerWireframe);

      currentY += layer_vis_h;
    });

    // Build interactive layer dividers if layers >= 2
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
          depthWrite: false
        });
        const divMesh = new THREE.Mesh(divGeo, divMat);
        divMesh.position.set(0, accumY, 0);
        divMesh.rotation.x = -Math.PI / 2; // lie flat in XZ plane
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
          opacity: 0.3
        });
        const divBorder = new THREE.Line(borderGeo, borderMat);
        divBorder.name = "border";
        divMesh.add(divBorder);

        divMesh.visible = false;
        mesh.add(divMesh);
      }
    }
  });

  // 3. Compute auto-camera fit
  const bbox = new THREE.Box3();
  data.shards.forEach(sd => {
    const minX = sd.position.x * VIS_SCALE;
    const minY = sd.position.z * VIS_SCALE; // Three Y (Rust Z height)
    const minZ = sd.position.y * VIS_SCALE; // Three Z (Rust Y depth)

    const maxX = (sd.position.x + sd.size.w) * VIS_SCALE;
    const maxY = (sd.position.z + sd.size.h) * VIS_SCALE;
    const maxZ = (sd.position.y + sd.size.d) * VIS_SCALE;

    bbox.expandByPoint(new THREE.Vector3(minX, minY, minZ));
    bbox.expandByPoint(new THREE.Vector3(maxX, maxY, maxZ));
  });

  if (!preserveCamera) {
    fitCameraToScene(bbox);
  }

  // Apply hidden/solo visibility filters
  updateLevelsVisibility();
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

  levelsGroup.clear();
  deptsGroup.clear();

  const placement = store.get('placementData');
  if (!placement) return;

  const levels = placement.levels || [];
  const levelsMap = {};
  levels.forEach(lvl => {
    levelsMap[lvl.id] = lvl;
  });

  const levelAABB = {}; // levelId -> { xMin, xMax, yMin, yMax }
  const resolvedDepts = {}; // deptName -> { xMin, xMax, yMin, yMax, orbit }

  // Inspect current mesh positions on the scene to calculate actual boundary boxes
  for (const [key, mesh] of Object.entries(shardMeshes)) {
    const sd = shardDataMap[mesh.uuid];
    if (!sd) continue;

    const w = sd.size.w;
    const d = sd.size.d;
    const h = sd.size.h;

    // Decode current AABB min in voxels
    const px = mesh.position.x / VIS_SCALE - w / 2;
    const py = mesh.position.z / VIS_SCALE - d / 2;
    const pz = mesh.position.y / VIS_SCALE - h / 2;

    const lvlId = sd.orbit;

    // Track level bounds
    if (!levelAABB[lvlId]) {
      levelAABB[lvlId] = {
        xMin: px,
        xMax: px + w,
        yMin: py,
        yMax: py + d
      };
    } else {
      const box = levelAABB[lvlId];
      box.xMin = Math.min(box.xMin, px);
      box.xMax = Math.max(box.xMax, px + w);
      box.yMin = Math.min(box.yMin, py);
      box.yMax = Math.max(box.yMax, py + d);
    }

    // Track department bounds
    const dname = sd.dept;
    if (!resolvedDepts[dname]) {
      resolvedDepts[dname] = {
        xMin: px,
        xMax: px + w,
        yMin: py,
        yMax: py + d,
        orbit: lvlId
      };
    } else {
      const dObj = resolvedDepts[dname];
      dObj.xMin = Math.min(dObj.xMin, px);
      dObj.xMax = Math.max(dObj.xMax, px + w);
      dObj.yMin = Math.min(dObj.yMin, py);
      dObj.yMax = Math.max(dObj.yMax, py + d);
    }
  }

  // Render levels
  levels.forEach(lvl => {
    const box = levelAABB[lvl.id];
    if (!box) return;

    const x = (box.xMin + box.xMax) / 2 * VIS_SCALE;
    const y = (lvl.z_start + lvl.height / 2) * VIS_SCALE;
    const z = (box.yMin + box.yMax) / 2 * VIS_SCALE;

    const w = (box.xMax - box.xMin) * VIS_SCALE;
    const h = lvl.height * VIS_SCALE;
    const d = (box.yMax - box.yMin) * VIS_SCALE;

    const geo = new THREE.BoxGeometry(w, h, d);
    const edgeGeo = new THREE.EdgesGeometry(geo);
    const lvlColor = new THREE.Color(lvl.color || "#ffffff");
    const mat = new THREE.LineBasicMaterial({
      color: lvlColor,
      transparent: true,
      opacity: 0.18,
    });
    const wire = new THREE.LineSegments(edgeGeo, mat);
    wire.position.set(x, y, z);
    wire.raycast = () => {};
    levelsGroup.add(wire);
  });

  // Render departments
  Object.entries(resolvedDepts).forEach(([dname, dObj]) => {
    const lvl = levelsMap[dObj.orbit];
    if (!lvl) return;

    const x = (dObj.xMin + dObj.xMax) / 2 * VIS_SCALE;
    const y = (lvl.z_start + lvl.height / 2) * VIS_SCALE;
    const z = (dObj.yMin + dObj.yMax) / 2 * VIS_SCALE;

    const w = (dObj.xMax - dObj.xMin) * VIS_SCALE;
    const h = lvl.height * VIS_SCALE;
    const d = (dObj.yMax - dObj.yMin) * VIS_SCALE;

    const geo = new THREE.BoxGeometry(w, h, d);
    const edgeGeo = new THREE.EdgesGeometry(geo);
    const mat = new THREE.LineDashedMaterial({
      color: 0x8b949e,
      dashSize: 0.8 * VIS_SCALE,
      gapSize: 0.4 * VIS_SCALE,
      transparent: true,
      opacity: 0.25
    });
    const wire = new THREE.LineSegments(edgeGeo, mat);
    wire.computeLineDistances();
    wire.position.set(x, y, z);
    wire.raycast = () => {};
    deptsGroup.add(wire);
  });
}

// Self-subscribe to store changes for rendering visibility/opacity of containers
store.on('focusedLevelId', () => {
  updateLevelsVisibility();
});
store.on('hiddenLevelIds', () => {
  updateLevelsVisibility();
});
