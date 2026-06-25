/**
 * @fileoverview mesh_factory.js — Constructs Three.js geometries, materials, sprites, and socket/layer objects.
 */

import * as THREE from 'three';
import { GLTFLoader } from 'three/addons/loaders/GLTFLoader.js';
import { shardMeshes, socketMeshes, VIS_SCALE } from '../scene_builder.js';
import { store } from '../store/store.js';
import { SOCKET_COLORS } from './socket_styles.js';

// Shared visual resources
export let pinGeo = null;
export let inputPinMat = null;
export let outputPinMat = null;
export let backingMat = null;
export let pinH = 0;

const gltfLoader = new GLTFLoader();

export const pinModels = {
  yellow: null,
  green: null,
  red: null,
  surprise: null
};

export function loadPinModels() {
  const modelNames = ['yellow', 'green', 'red', 'surprise'];
  const promises = modelNames.map(name => {
    return new Promise((resolve) => {
      gltfLoader.load(`./pin_${name}.glb`, (gltf) => {
        const meshes = [];
        gltf.scene.traverse(child => {
          if (child.isMesh) {
            meshes.push({
              geometry: child.geometry.clone(),
              material: child.material.clone(),
              position: child.position.clone(),
              rotation: child.rotation.clone(),
              scale: child.scale.clone()
            });
          }
        });
        pinModels[name] = meshes;
        console.log(`Loaded pin model: ${name} (${meshes.length} parts)`);
        resolve();
      }, undefined, (err) => {
        console.warn(`Failed to load pin_${name}.glb (using standard cube fallback):`, err);
        resolve();
      });
    });
  });
  return Promise.all(promises);
}

/**
 * Initializes reusable standard Three.js materials and geometries.
 * @param {number} scale 
 */
export function initSharedResources(scale) {
  const pinW = 0.5 * scale;
  const pinD = 0.5 * scale;
  pinH = 0.15 * scale;
  pinGeo = new THREE.BoxGeometry(pinW, pinD, pinH);

  inputPinMat = new THREE.MeshStandardMaterial({
    color: 0x00d2ff,
    emissive: 0x00d2ff,
    emissiveIntensity: 0.8,
    roughness: 0.2,
    metalness: 0.1
  });

  outputPinMat = new THREE.MeshStandardMaterial({
    color: 0xff7700,
    emissive: 0xff7700,
    emissiveIntensity: 0.8,
    roughness: 0.2,
    metalness: 0.1
  });

  backingMat = new THREE.MeshBasicMaterial({
    color: 0x050508,
    transparent: true,
    opacity: 0.7,
    side: THREE.DoubleSide
  });
}

/**
 * Appends corner and edge drag resize handles to a socket group.
 * @param {THREE.Group} socketGroup 
 * @param {number} backingW 
 * @param {number} backingH 
 */
export function addResizerHandles(socketGroup, backingW, backingH) {
  const handleSize = 0.3 * VIS_SCALE;
  const handleGeo = new THREE.BoxGeometry(handleSize, handleSize, handleSize);
  const handleMat = new THREE.MeshBasicMaterial({
    color: 0xff7700,
    transparent: true,
    opacity: 0.95,
    depthTest: false
  });

  const isSelected = (socketGroup.userData && socketGroup.userData.socketKey === store.get('selectedSocketKey'));
  const isResizeMode = (store.get('activeMode') === 'resize');
  const handlesVisible = isSelected && isResizeMode;

  const handlePositions = [
    { name: 'handle_T', x: 0, y: backingH / 2 },
    { name: 'handle_B', x: 0, y: -backingH / 2 },
    { name: 'handle_R', x: backingW / 2, y: 0 },
    { name: 'handle_L', x: -backingW / 2, y: 0 },
    { name: 'handle_TR', x: backingW / 2, y: backingH / 2 },
    { name: 'handle_TL', x: -backingW / 2, y: backingH / 2 },
    { name: 'handle_BR', x: backingW / 2, y: -backingH / 2 },
    { name: 'handle_BL', x: -backingW / 2, y: -backingH / 2 }
  ];

  handlePositions.forEach(pos => {
    const handle = new THREE.Mesh(handleGeo, handleMat);
    handle.position.set(pos.x, pos.y, 0.05 * VIS_SCALE);
    handle.name = pos.name;
    handle.visible = handlesVisible;
    socketGroup.add(handle);
  });
}

/**
 * Creates a translucent background layer plane.
 * @param {number} width 
 * @param {number} depth 
 * @param {number} yPos 
 * @param {number} color 
 * @returns {THREE.Group}
 */
export function createLayerPlane(width, depth, yPos, color) {
  const group = new THREE.Group();
  group.position.set(0, yPos, 0);

  const planeGeo = new THREE.PlaneGeometry(width * 1.05, depth * 1.05);
  const planeMat = new THREE.MeshStandardMaterial({
    color, transparent: true, opacity: 0.02,
    roughness: 0.3, metalness: 0.8,
    side: THREE.DoubleSide, depthWrite: false,
  });
  const planeMesh = new THREE.Mesh(planeGeo, planeMat);
  planeMesh.rotation.x = Math.PI / 2;
  planeMesh.position.set(0, 0, 0);
  group.add(planeMesh);

  // Outline border
  const halfW = width * 1.05 / 2;
  const halfD = depth * 1.05 / 2;
  const borderGeo = new THREE.BufferGeometry().setFromPoints([
    new THREE.Vector3(-halfW, 0, -halfD),
    new THREE.Vector3(halfW, 0, -halfD),
    new THREE.Vector3(halfW, 0, halfD),
    new THREE.Vector3(-halfW, 0, halfD),
    new THREE.Vector3(-halfW, 0, -halfD)
  ]);
  const borderMat = new THREE.LineBasicMaterial({
    color, transparent: true, opacity: 0.2,
  });
  const border = new THREE.Line(borderGeo, borderMat);
  border.position.set(0, 0, 0);
  group.add(border);

  // positioning grid
  const editorSettings = store.get('editorSettings') || {};
  const gridStep = (editorSettings.grid_step || 100) * VIS_SCALE;
  const gridPoints = [];

  // Vertical lines (Z axis)
  for (let x = -Math.floor(halfW / gridStep) * gridStep; x <= halfW; x += gridStep) {
    gridPoints.push(new THREE.Vector3(x, 0.005 * VIS_SCALE, -halfD));
    gridPoints.push(new THREE.Vector3(x, 0.005 * VIS_SCALE, halfD));
  }
  // Horizontal lines (X axis)
  for (let z = -Math.floor(halfD / gridStep) * gridStep; z <= halfD; z += gridStep) {
    gridPoints.push(new THREE.Vector3(-halfW, 0.005 * VIS_SCALE, z));
    gridPoints.push(new THREE.Vector3(halfW, 0.005 * VIS_SCALE, z));
  }

  const gridGeo = new THREE.BufferGeometry().setFromPoints(gridPoints);
  const gridMat = new THREE.LineBasicMaterial({
    color, transparent: true, opacity: 0.12,
  });
  const grid = new THREE.LineSegments(gridGeo, gridMat);
  group.add(grid);

  return group;
}

/**
 * Creates a sprite with text content on a canvas.
 * @param {string} text 
 * @param {number} color 
 * @returns {THREE.Sprite}
 */
export function makeTextSprite(text, color) {
  const canvas = document.createElement('canvas');
  const size = 512;
  canvas.width = size;
  canvas.height = 128;
  const ctx = canvas.getContext('2d');

  ctx.font = 'bold 36px Segoe UI, system-ui, sans-serif';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';

  const hexColor = '#' + new THREE.Color(color).getHexString();
  ctx.shadowColor = hexColor;
  ctx.shadowBlur = 10;
  ctx.fillStyle = hexColor;
  ctx.fillText(text, size / 2, 64);
  ctx.shadowBlur = 0;
  ctx.fillStyle = '#ffffff';
  ctx.globalAlpha = 0.65;
  ctx.fillText(text, size / 2, 64);

  const tex = new THREE.CanvasTexture(canvas);
  tex.minFilter = THREE.LinearFilter;
  const mat = new THREE.SpriteMaterial({ map: tex, transparent: true, depthWrite: false });
  const sprite = new THREE.Sprite(mat);
  sprite.scale.set(4, 1, 1);
  sprite.raycast = () => {};
  return sprite;
}

/**
 * Rebuilds the socket's InstancedMesh and backing plane representation.
 * @param {string} shardKey 
 * @param {string} socketName 
 * @param {number} width 
 * @param {number} height 
 * @param {number} pitch 
 * @param {{x: number, y: number}} offset 
 * @param {1|-1} faceSign 
 * @param {number} rotation 
 */
export function rebuildSocket(shardKey, socketName, width, height, pitch, offset, faceSign, rotation = 0) {
  const socketKey = `${shardKey}.${socketName}`;
  const shardMesh = shardMeshes.get(shardKey);
  if (!shardMesh) return;

  const h = shardMesh.geometry.parameters.depth; // local depth

  // Clean up old socket group and dispose its WebGL resources to prevent memory leaks
  const oldGroup = shardMesh.getObjectByName(`socket_${socketName}`);
  if (oldGroup) {
    oldGroup.traverse(child => {
      if (child.geometry) child.geometry.dispose();
      if (child.material) {
        if (Array.isArray(child.material)) child.material.forEach(m => m.dispose());
        else child.material.dispose();
      }
    });
    shardMesh.remove(oldGroup);
  }

  const socketGroup = new THREE.Group();
  socketGroup.name = `socket_${socketName}`;

  const spacing = VIS_SCALE * pitch;

  const backingW = width * spacing;
  const backingH = height * spacing;

  // Determine socket visual state
  let state = 'free';
  const problematicSockets = store.get('problematicSockets') || [];
  if (problematicSockets.includes(socketKey)) {
    state = 'error';
  } else {
    const placementData = store.get('placementData');
    const isConnected = placementData && placementData.connections && placementData.connections.some(c =>
      `${c.from}.${c.from_socket}` === socketKey || `${c.to}.${c.to_socket}` === socketKey
    );
    state = isConnected ? 'connected' : 'free';
  }

  // Surprise Easter Egg: 1 in 1,000,000 chance, or 100% if socket has surprise/banana in name
  const isSurprise = (Math.random() < 0.000001) ||
    socketName.toLowerCase().includes("banana") ||
    socketKey.toLowerCase().includes("surprise");

  const activeColors = SOCKET_COLORS[state];

  // Backing box (volumetric slab)
  const backingDepth = 0.05 * VIS_SCALE;
  const backingGeo = new THREE.BoxGeometry(backingW, backingH, backingDepth);
  const backingMeshMat = new THREE.MeshStandardMaterial({
    color: activeColors.backing,
    roughness: 0.5,
    metalness: 0.1
  });
  const backingMesh = new THREE.Mesh(backingGeo, backingMeshMat);
  backingMesh.position.set(0, 0, faceSign * (backingDepth / 2));
  if (state === 'connected') {
    backingMesh.material.visible = false; // Connected sockets have invisible backing box by default
  }
  socketGroup.add(backingMesh);

  const count = width * height;
  const modelParts = isSurprise ? pinModels['surprise'] : null;

  if (modelParts && modelParts.length > 0) {
    const instancedMeshes = [];
    modelParts.forEach(part => {
      const inst = new THREE.InstancedMesh(part.geometry, part.material.clone(), count);
      socketGroup.add(inst);
      instancedMeshes.push({ inst, part });
    });

    const modelScale = VIS_SCALE * 0.5;
    // Align base of surprise model with top of the backing slab
    const zPos = faceSign * (backingDepth + 0.01 * VIS_SCALE);

    // 1. Precompute part matrices once outside the nested grid loops
    const precomputedParts = modelParts.map((part, partIdx) => {
      const partPos = part.position.clone().multiplyScalar(modelScale);
      partPos.z += zPos;

      const partMatrix = new THREE.Matrix4().compose(
        partPos,
        new THREE.Quaternion().setFromEuler(part.rotation),
        part.scale.clone().multiplyScalar(modelScale)
      );

      return {
        inst: instancedMeshes[partIdx].inst,
        partMatrix
      };
    });

    const dummy = new THREE.Object3D();
    const finalMatrix = new THREE.Matrix4();

    for (let row = 0; row < height; row++) {
      for (let col = 0; col < width; col++) {
        const localX = (col - (width - 1) / 2) * spacing;
        const localY = (row - (height - 1) / 2) * spacing;

        dummy.position.set(localX, localY, 0);
        dummy.rotation.set(0, 0, 0);

        if (faceSign === 1) {
          dummy.rotation.x = Math.PI / 2;
        } else {
          dummy.rotation.x = -Math.PI / 2;
        }

        dummy.scale.set(1, 1, 1);
        dummy.updateMatrix();

        const idx = row * width + col;
        
        for (let p = 0; p < precomputedParts.length; p++) {
          const item = precomputedParts[p];
          finalMatrix.multiplyMatrices(dummy.matrix, item.partMatrix);
          item.inst.setMatrixAt(idx, finalMatrix);
        }
      }
    }

    instancedMeshes.forEach(({ inst }) => {
      inst.instanceMatrix.needsUpdate = true;
    });
  } else {
    // Standard cube pins with state-based colors
    const pinMat = new THREE.MeshStandardMaterial({
      color: activeColors.pin,
      emissive: activeColors.pin,
      emissiveIntensity: 0.8,
      roughness: 0.2,
      metalness: 0.1
    });

    const instancedMesh = new THREE.InstancedMesh(pinGeo, pinMat, count);
    socketGroup.add(instancedMesh);

    const dummy = new THREE.Object3D();
    let idx = 0;
    // Position pin centers exactly resting on the surface of the backing slab
    const pinZ = faceSign * (backingDepth + pinH / 2);
    for (let row = 0; row < height; row++) {
      for (let col = 0; col < width; col++) {
        const localX = (col - (width - 1) / 2) * spacing;
        const localY = (row - (height - 1) / 2) * spacing;

        dummy.position.set(localX, localY, pinZ);
        dummy.updateMatrix();
        instancedMesh.setMatrixAt(idx++, dummy.matrix);
      }
    }
    instancedMesh.instanceMatrix.needsUpdate = true;
  }

  const ox = offset.x * VIS_SCALE;
  const oy = offset.y * VIS_SCALE;
  const defaultZ = faceSign * (h / (2 * VIS_SCALE)); // clean boundary in voxels
  const oz_voxels = offset.z !== undefined ? offset.z : defaultZ;
  const oz = oz_voxels * VIS_SCALE + faceSign * 0.01 * VIS_SCALE; // z-fighting offset purely at render-time
  socketGroup.position.set(ox, oy, oz);

  socketGroup.rotation.z = THREE.MathUtils.degToRad(rotation);

  const placementData = store.get('placementData');
  let entry_z = faceSign === 1 ? 'top' : 'bottom';
  if (placementData) {
    const shard = placementData.shards.find(s => s.key === shardKey);
    if (shard && shard.sockets) {
      const socket = shard.sockets.find(s => s.name === socketName);
      if (socket && socket.entry_z) {
        entry_z = socket.entry_z;
      }
    }
  }

  socketGroup.userData = {
    socketKey,
    shardKey,
    socketName,
    faceSign,
    entry_z,
    originalOffset: {
      x: offset.x,
      y: offset.y,
      z: oz_voxels
    },
    width,
    height,
    pitch,
    rotation,
    backingMesh,
    originalBackingColor: activeColors.backing,
    originalBackingVisible: (state !== 'connected')
  };

  addResizerHandles(socketGroup, backingW, backingH);
  shardMesh.add(socketGroup);
  socketMeshes.set(socketKey, socketGroup);
}
