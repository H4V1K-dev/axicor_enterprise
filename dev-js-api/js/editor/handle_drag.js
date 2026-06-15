/**
 * @fileoverview handle_drag.js — Coordinates the edge/corner handle dragging for resizing sockets.
 */

import * as THREE from 'three';
import { getActiveCamera, controls, renderer } from '../viewer.js';
import { shardMeshes, socketMeshes, VIS_SCALE } from '../scene_builder.js';
import { store } from '../store/store.js';
import { selectSocket } from './selection.js';
import { rebuildSocket, drawRoutes } from '../scene_builder.js';
import { emit, EVENTS } from '../store/event_bus.js';

let raycaster = new THREE.Raycaster();
let mouse = new THREE.Vector2();

let isDraggingHandle = false;
let draggedSocketGroup = null;
let activeHandleName = null;

let dragPlane = new THREE.Plane();
let dragPlaneIntersection = new THREE.Vector3();

// Handle drag dimensions tracking
let initialSocketWidth = 8;
let initialSocketHeight = 8;
let initialSocketPitch = 1;
let initialSocketOffset = { x: 0, y: 0 };
let initialLocalX = 0;
let initialLocalY = 0;
let initialSocketState = null;

export function isDragging() {
  return isDraggingHandle;
}

export function onPointerDown(event) {
  // Only LMB is used for dragging handles
  if (event.button !== 0) return false;

  if (event.target.closest('#hud') || event.target.closest('#sidebar') || event.target.closest('#tooltip') || event.target.closest('#save-all-btn')) {
    return false;
  }

  if (renderer && renderer.domElement) {
    const rect = renderer.domElement.getBoundingClientRect();
    mouse.x = ((event.clientX - rect.left) / rect.width) * 2 - 1;
    mouse.y = -((event.clientY - rect.top) / rect.height) * 2 + 1;
  } else {
    mouse.x = (event.clientX / window.innerWidth) * 2 - 1;
    mouse.y = -(event.clientY / window.innerHeight) * 2 + 1;
  }
  const activeCamera = getActiveCamera();
  activeCamera.updateMatrixWorld();
  raycaster.setFromCamera(mouse, activeCamera);

  // Check if click hit a resizer handle (which is visible)
  const handlesList = [];
  for (const group of Object.values(socketMeshes)) {
    group.traverse(child => {
      if (child.name && child.name.startsWith("handle_") && child.visible) {
        handlesList.push(child);
      }
    });
  }

  const handleHits = raycaster.intersectObjects(handlesList);
  if (handleHits.length > 0) {
    const hitHandle = handleHits[0].object;
    isDraggingHandle = true;
    activeHandleName = hitHandle.name;
    draggedSocketGroup = hitHandle.parent;
    controls.enabled = false; // Disable camera OrbitControls
    
    // Store initial dimensions and offsets
    initialSocketWidth = draggedSocketGroup.userData.width;
    initialSocketHeight = draggedSocketGroup.userData.height;
    initialSocketPitch = draggedSocketGroup.userData.pitch;
    initialSocketOffset = { ...draggedSocketGroup.userData.originalOffset };

    const placementData = store.get('placementData');
    if (placementData) {
      const { shardKey, socketName } = draggedSocketGroup.userData;
      const shard = placementData.shards.find(s => s.key === shardKey);
      if (shard && shard.sockets) {
        const socket = shard.sockets.find(s => s.name === socketName);
        if (socket) {
          initialSocketState = JSON.parse(JSON.stringify(socket));
        }
      }
    }
    
    // Intersection plane perpendicular to world Y (socket face height)
    const worldPos = new THREE.Vector3();
    hitHandle.getWorldPosition(worldPos);
    dragPlane.setFromNormalAndCoplanarPoint(new THREE.Vector3(0, 1, 0), worldPos);
    
    // Get initial local intersection coordinate
    if (raycaster.ray.intersectPlane(dragPlane, dragPlaneIntersection)) {
      const shardMesh = shardMeshes[draggedSocketGroup.userData.shardKey];
      initialLocalX = (dragPlaneIntersection.x - shardMesh.position.x) / VIS_SCALE;
      initialLocalY = -(dragPlaneIntersection.z - shardMesh.position.z) / VIS_SCALE;
    }
    return true;
  }
  return false;
}

export function onPointerMove(event) {
  if (renderer && renderer.domElement) {
    const rect = renderer.domElement.getBoundingClientRect();
    mouse.x = ((event.clientX - rect.left) / rect.width) * 2 - 1;
    mouse.y = -((event.clientY - rect.top) / rect.height) * 2 + 1;
  } else {
    mouse.x = (event.clientX / window.innerWidth) * 2 - 1;
    mouse.y = -(event.clientY / window.innerHeight) * 2 + 1;
  }
  const activeCamera = getActiveCamera();
  activeCamera.updateMatrixWorld();
  raycaster.setFromCamera(mouse, activeCamera);

  if (!isDraggingHandle || !draggedSocketGroup) return;

  if (raycaster.ray.intersectPlane(dragPlane, dragPlaneIntersection)) {
    const shardMesh = shardMeshes[draggedSocketGroup.userData.shardKey];
    if (shardMesh) {
      const currentLocalX = (dragPlaneIntersection.x - shardMesh.position.x) / VIS_SCALE;
      const currentLocalY = -(dragPlaneIntersection.z - shardMesh.position.z) / VIS_SCALE;
      
      const deltaX = currentLocalX - initialLocalX;
      const deltaY = currentLocalY - initialLocalY;
      
      const pitch = initialSocketPitch;
      
      // Calculate pin steps snapped to grid
      const stepsX = Math.round(deltaX / pitch);
      const stepsY = Math.round(deltaY / pitch);
      
      let newWidth = initialSocketWidth;
      let newHeight = initialSocketHeight;
      let newOffset = { ...initialSocketOffset };

      if (activeHandleName === 'handle_R') {
        newWidth = Math.max(2, initialSocketWidth + stepsX);
        newOffset.x = initialSocketOffset.x + (newWidth - initialSocketWidth) * pitch / 2;
      } else if (activeHandleName === 'handle_L') {
        newWidth = Math.max(2, initialSocketWidth - stepsX);
        newOffset.x = initialSocketOffset.x - (newWidth - initialSocketWidth) * pitch / 2;
      } else if (activeHandleName === 'handle_T') {
        newHeight = Math.max(2, initialSocketHeight + stepsY);
        newOffset.y = initialSocketOffset.y + (newHeight - initialSocketHeight) * pitch / 2;
      } else if (activeHandleName === 'handle_B') {
        newHeight = Math.max(2, initialSocketHeight - stepsY);
        newOffset.y = initialSocketOffset.y - (newHeight - initialSocketHeight) * pitch / 2;
      } else if (activeHandleName === 'handle_TR') {
        newWidth = Math.max(2, initialSocketWidth + stepsX);
        newHeight = Math.max(2, initialSocketHeight + stepsY);
        newOffset.x = initialSocketOffset.x + (newWidth - initialSocketWidth) * pitch / 2;
        newOffset.y = initialSocketOffset.y + (newHeight - initialSocketHeight) * pitch / 2;
      } else if (activeHandleName === 'handle_TL') {
        newWidth = Math.max(2, initialSocketWidth - stepsX);
        newHeight = Math.max(2, initialSocketHeight + stepsY);
        newOffset.x = initialSocketOffset.x - (newWidth - initialSocketWidth) * pitch / 2;
        newOffset.y = initialSocketOffset.y + (newHeight - initialSocketHeight) * pitch / 2;
      } else if (activeHandleName === 'handle_BR') {
        newWidth = Math.max(2, initialSocketWidth + stepsX);
        newHeight = Math.max(2, initialSocketHeight - stepsY);
        newOffset.x = initialSocketOffset.x + (newWidth - initialSocketWidth) * pitch / 2;
        newOffset.y = initialSocketOffset.y - (newHeight - initialSocketHeight) * pitch / 2;
      } else if (activeHandleName === 'handle_BL') {
        newWidth = Math.max(2, initialSocketWidth - stepsX);
        newHeight = Math.max(2, initialSocketHeight - stepsY);
        newOffset.x = initialSocketOffset.x - (newWidth - initialSocketWidth) * pitch / 2;
        newOffset.y = initialSocketOffset.y - (newHeight - initialSocketHeight) * pitch / 2;
      }

      // Limit bounds: check if size goes beyond shard width/depth
      const shardW = shardMesh.geometry.parameters.width / VIS_SCALE;
      const shardD = shardMesh.geometry.parameters.height / VIS_SCALE;
      
      const backingW = newWidth * pitch;
      const backingH = newHeight * pitch;

      const halfShardW = shardW / 2;
      const halfShardD = shardD / 2;

      const fits = (newOffset.x - backingW / 2 >= -halfShardW) &&
                   (newOffset.x + backingW / 2 <= halfShardW) &&
                   (newOffset.y - backingH / 2 >= -halfShardD) &&
                   (newOffset.y + backingH / 2 <= halfShardD);
      
      if (fits) {
        if (newWidth !== draggedSocketGroup.userData.width || 
            newHeight !== draggedSocketGroup.userData.height || 
            newOffset.x !== draggedSocketGroup.userData.originalOffset.x ||
            newOffset.y !== draggedSocketGroup.userData.originalOffset.y) {
          updateSelectedSocket(newWidth, newHeight, pitch, newOffset);
        }
      }
    }
  }
}

export function onPointerUp() {
  if (isDraggingHandle) {
    const selSocketKey = store.get('selectedSocketKey');
    if (selSocketKey && initialSocketState) {
      const placementData = store.get('placementData');
      if (placementData) {
        const lastDot = selSocketKey.lastIndexOf('.');
        const shardKey = selSocketKey.substring(0, lastDot);
        const socketName = selSocketKey.substring(lastDot + 1);
        const shard = placementData.shards.find(s => s.key === shardKey);
        if (shard && shard.sockets) {
          const socket = shard.sockets.find(s => s.name === socketName);
          if (socket) {
            const initOffset = initialSocketState.offset || initialSocketState.originalOffset || { x: 0, y: 0, z: 0 };
            const currOffset = socket.offset || { x: 0, y: 0, z: 0 };

            const initZ = initOffset.z !== undefined ? initOffset.z : 0;
            const currZ = currOffset.z !== undefined ? currOffset.z : 0;

            if (socket.width !== initialSocketState.width ||
                socket.height !== initialSocketState.height ||
                currOffset.x !== initOffset.x ||
                currOffset.y !== initOffset.y ||
                currZ !== initZ) {
              
              const undoState = JSON.parse(JSON.stringify(initialSocketState));
              const redoState = JSON.parse(JSON.stringify(socket));

              import('../store/history_manager.js').then(({ historyManager }) => {
                historyManager.pushAction(
                  'resize', 
                  'socket', 
                  selSocketKey, 
                  `Изменение размеров сокета ${socketName}`, 
                  undoState, 
                  redoState
                );
              });
            }
          }
        }
      }
    }

    isDraggingHandle = false;
    draggedSocketGroup = null;
    initialSocketState = null;
    controls.enabled = true; // Re-enable camera OrbitControls
    document.body.style.cursor = 'auto';
  }
}

// Modify socket properties (width, height, pitch, offset, rotation, faceSign)
export function updateSelectedSocket(width, height, pitch, offset, rotation, faceSign) {
  const selSocketKey = store.get('selectedSocketKey');
  if (!selSocketKey) return;
  const group = socketMeshes[selSocketKey];
  if (!group) return;

  const { shardKey, socketName } = group.userData;

  const finalOffset = offset !== undefined ? offset : group.userData.originalOffset;
  const finalRotation = rotation !== undefined ? rotation : (group.userData.rotation || 0);
  const finalFaceSign = faceSign !== undefined ? faceSign : group.userData.faceSign;

  // Dynamic mesh update
  rebuildSocket(shardKey, socketName, width, height, pitch, finalOffset, finalFaceSign, finalRotation);
  
  // Update placementData in store so it stays in sync
  const placementData = store.get('placementData');
  if (placementData) {
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
        store.set('placementData', placementData);
      }
    }
  }

  // Reselect newly built socket to preserve highlights and controls
  selectSocket(selSocketKey);

  // Redraw routes and trigger validation check
  const routes = store.get('routesData');
  if (routes) drawRoutes(routes);
  emit(EVENTS.VALIDATION_REQ);
}

// Dynamic handles scaling based on camera distance
export function updateHandlesScale() {
  const selSocketKey = store.get('selectedSocketKey');
  if (!selSocketKey) return;
  const group = socketMeshes[selSocketKey];
  if (!group) return;

  const camPos = getActiveCamera().position;
  const minD = 40.0;
  const maxD = 250.0;
  const minScale = 1.0;
  const maxScale = 5.0;

  group.traverse(child => {
    if (child.name && child.name.startsWith("handle_")) {
      const worldPos = new THREE.Vector3();
      child.getWorldPosition(worldPos);
      const distWorld = camPos.distanceTo(worldPos);
      const distVoxels = distWorld / VIS_SCALE;

      let ratio = (distVoxels - minD) / (maxD - minD);
      ratio = Math.max(0, Math.min(1, ratio));
      const s = minScale + ratio * (maxScale - minScale);

      child.scale.set(s, s, s);
    }
  });
}
