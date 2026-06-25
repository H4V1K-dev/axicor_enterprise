/**
 * @fileoverview divider_drag.js — Coordinates dragging horizontal dividers between cortical layers.
 */

import * as THREE from 'three';
import { getActiveCamera, controls, renderer } from '../viewer.js';
import { shardMeshes, shardDataMap, VIS_SCALE } from '../scene_builder.js';
import { store } from '../store/store.js';
import { emit, EVENTS } from '../store/event_bus.js';

let raycaster = new THREE.Raycaster();
let mouse = new THREE.Vector2();

let isDraggingDivider = false;
let draggedDivider = null;

let dragPlane = new THREE.Plane();
let dragPlaneIntersection = new THREE.Vector3();

export function isDragging() {
  return isDraggingDivider;
}

export function onPointerDown(event) {
  // Only LMB is used for dragging dividers
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

  // Check if click hit a layer divider
  const dividersList = [];
  for (const mesh of shardMeshes.values()) {
    mesh.traverse(child => {
      if (child.userData && child.userData.isDivider && child.visible) {
        dividersList.push(child);
      }
    });
  }

  const divHits = raycaster.intersectObjects(dividersList);
  if (divHits.length > 0) {
    const hitDivider = divHits[0].object;
    isDraggingDivider = true;
    draggedDivider = hitDivider;
    controls.enabled = false; // Disable camera OrbitControls
    
    hitDivider.updateMatrixWorld(true);
    const worldPos = new THREE.Vector3();
    hitDivider.getWorldPosition(worldPos);
    
    // Intersection plane perpendicular to cam look direction (XZ projected)
    const camDir = new THREE.Vector3();
    getActiveCamera().getWorldDirection(camDir);
    const normal = new THREE.Vector3(camDir.x, 0, camDir.z).normalize().negate();
    dragPlane.setFromNormalAndCoplanarPoint(normal, worldPos);

    // Record initial intersection local coordinates and position to prevent jumping
    if (raycaster.ray.intersectPlane(dragPlane, dragPlaneIntersection)) {
      const shardMesh = shardMeshes.get(hitDivider.userData.shardKey);
      if (shardMesh) {
        shardMesh.updateMatrixWorld(true);
        const localIntersection = shardMesh.worldToLocal(dragPlaneIntersection.clone());
        hitDivider.userData.initialLocalZIntersection = localIntersection.z;
        hitDivider.userData.initialPositionZ = hitDivider.position.z;
      }
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

  if (!isDraggingDivider || !draggedDivider) return;

  if (raycaster.ray.intersectPlane(dragPlane, dragPlaneIntersection)) {
    const shardMesh = shardMeshes.get(draggedDivider.userData.shardKey);
    if (shardMesh) {
      const sd = shardDataMap.get(shardMesh.uuid);
      if (!sd) return;
      const idx = draggedDivider.userData.dividerIndex;
      
      // Collect all dividers for this shard to find neighbors
      const dividers = [];
      shardMesh.traverse(child => {
        if (child.userData && child.userData.isDivider) {
          dividers.push(child);
        }
      });
      dividers.sort((a, b) => a.userData.dividerIndex - b.userData.dividerIndex);
      
      const h = sd.size.h * VIS_SCALE;
      
      // Lower limit (Z min in Three.js units, corresponding to 1 voxel minimum layer thickness)
      let minZ = -h / 2 + VIS_SCALE;
      if (idx > 0) {
        const prevDiv = dividers.find(d => d.userData.dividerIndex === idx - 1);
        if (prevDiv) {
          minZ = prevDiv.position.z + VIS_SCALE;
        }
      }
      
      // Upper limit (Z max in Three.js units, corresponding to 1 voxel minimum layer thickness)
      let maxZ = h / 2 - VIS_SCALE;
      if (idx < dividers.length - 1) {
        const nextDiv = dividers.find(d => d.userData.dividerIndex === idx + 1);
        if (nextDiv) {
          maxZ = nextDiv.position.z - VIS_SCALE;
        }
      }
      
      // Project ray intersection into shard local space
      const localIntersection = shardMesh.worldToLocal(dragPlaneIntersection.clone());
      
      // Calculate delta relative to start of drag
      const initialIntersectionZ = draggedDivider.userData.initialLocalZIntersection !== undefined
        ? draggedDivider.userData.initialLocalZIntersection
        : localIntersection.z;
      const initialPositionZ = draggedDivider.userData.initialPositionZ !== undefined
        ? draggedDivider.userData.initialPositionZ
        : draggedDivider.position.z;
        
      const deltaZ = localIntersection.z - initialIntersectionZ;
      let newLocalZ = initialPositionZ + deltaZ;
      
      // Snap to voxel grid (1 voxel = VIS_SCALE)
      let zInVoxels = Math.round(newLocalZ / VIS_SCALE);
      newLocalZ = zInVoxels * VIS_SCALE;
      
      // Clamp within limits
      newLocalZ = Math.max(minZ, Math.min(maxZ, newLocalZ));
      
      draggedDivider.position.z = newLocalZ;
      updateLayersFromDividers(shardMesh);
    }
  }
}

export function onPointerUp() {
  if (isDraggingDivider && draggedDivider) {
    const shardMesh = shardMeshes.get(draggedDivider.userData.shardKey);
    if (shardMesh) {
      const sd = shardDataMap.get(shardMesh.uuid);
      if (sd && sd.layers) {
        const idx = draggedDivider.userData.dividerIndex;
        
        // 1. Collect all dividers for this shard to find neighbors
        const dividers = [];
        shardMesh.traverse(child => {
          if (child.userData && child.userData.isDivider) {
            dividers.push(child);
          }
        });
        dividers.sort((a, b) => a.userData.dividerIndex - b.userData.dividerIndex);
        
        const h = sd.size.h * VIS_SCALE;
        
        // Get prev_z and next_z
        let prev_z = -h / 2;
        if (idx > 0) {
          const prevDiv = dividers.find(d => d.userData.dividerIndex === idx - 1);
          if (prevDiv) prev_z = prevDiv.position.z;
        }
        
        let next_z = h / 2;
        if (idx < dividers.length - 1) {
          const nextDiv = dividers.find(d => d.userData.dividerIndex === idx + 1);
          if (nextDiv) next_z = nextDiv.position.z;
        }
        
        const current_div_z = draggedDivider.position.z;
        
        // Calculate layer thicknesses in voxels
        const t1_voxels = Math.round((current_div_z - prev_z) / VIS_SCALE);
        const t2_voxels = Math.round((next_z - current_div_z) / VIS_SCALE);
        
        let layerAnnihilated = false;
        
        if (t1_voxels < 2) {
          // Annihilate layer idx (the one below divider)
          // Add its height_pct to layer idx + 1
          sd.layers[idx + 1].height_pct = parseFloat((sd.layers[idx + 1].height_pct + sd.layers[idx].height_pct).toFixed(4));
          sd.layers.splice(idx, 1);
          layerAnnihilated = true;
        } else if (t2_voxels < 2) {
          // Annihilate layer idx + 1 (the one above divider)
          // Add its height_pct to layer idx
          sd.layers[idx].height_pct = parseFloat((sd.layers[idx].height_pct + sd.layers[idx + 1].height_pct).toFixed(4));
          sd.layers.splice(idx + 1, 1);
          layerAnnihilated = true;
        }
        
        // Emit layout changed event
        emit(EVENTS.LAYOUT_CHANGED, sd);
      }
    }

    isDraggingDivider = false;
    draggedDivider = null;
    controls.enabled = true; // Re-enable camera OrbitControls
    document.body.style.cursor = 'auto';
  }
}

export function updateLayersFromDividers(shardMesh) {
  const sd = shardDataMap.get(shardMesh.uuid);
  if (!sd) return;

  const h = sd.size.h * VIS_SCALE;

  // 1. Collect dividers
  const dividers = [];
  shardMesh.traverse(child => {
    if (child.userData && child.userData.isDivider) {
      dividers.push(child);
    }
  });
  dividers.sort((a, b) => a.userData.dividerIndex - b.userData.dividerIndex);

  // 2. Build local Z coordinates of layer boundaries (bounds are [-h/2, h/2])
  const zBounds = [-h / 2];
  dividers.forEach(div => {
    zBounds.push(div.position.z);
  });
  zBounds.push(h / 2);

  // 3. Collect layer meshes
  const layerMeshes = [];
  shardMesh.traverse(child => {
    if (child.userData && child.userData.layerIndex !== undefined) {
      layerMeshes.push(child);
    }
  });
  layerMeshes.sort((a, b) => a.userData.layerIndex - b.userData.layerIndex);

  // 4. Update layer scales, positions and calculate new proportions
  const newProportions = {};
  layerMeshes.forEach((layerMesh, idx) => {
    const layer_vis_h = zBounds[idx + 1] - zBounds[idx];
    const zCenter = zBounds[idx] + layer_vis_h / 2;

    layerMesh.position.z = zCenter;
    layerMesh.scale.z = layer_vis_h;

    const newPct = layer_vis_h / h;
    layerMesh.userData.height_pct = newPct;
    newProportions[layerMesh.userData.layerName] = parseFloat(newPct.toFixed(4));
  });

  // 5. Update parent shard raw data map
  sd.layers.forEach(l => {
    if (newProportions[l.name] !== undefined) {
      l.height_pct = newProportions[l.name];
    }
  });

  // 6. If this shard is currently inspected, update sidebar UI
  if (store.get('selectedShardKey') === sd.key) {
    emit(EVENTS.LAYERS_CHANGED, sd);
  }
}

export function updateLayersOrderIn3D(shardMesh, newOrder) {
  const sd = shardDataMap.get(shardMesh.uuid);
  if (!sd) return;
  const h = sd.size.h * VIS_SCALE;

  // 1. Collect layers and dividers
  const layerMeshes = [];
  const dividers = [];
  shardMesh.traverse(child => {
    if (child.userData && child.userData.layerIndex !== undefined) {
      layerMeshes.push(child);
    }
    if (child.userData && child.userData.isDivider) {
      dividers.push(child);
    }
  });

  // 2. Update indices
  layerMeshes.forEach(mesh => {
    const newIdx = newOrder.indexOf(mesh.userData.layerName);
    if (newIdx !== -1) {
      mesh.userData.layerIndex = newIdx;
    }
  });
  layerMeshes.sort((a, b) => a.userData.layerIndex - b.userData.layerIndex);

  // 3. Build new local Z bounds
  const zBounds = [-h / 2];
  let currentZ = -h / 2;
  layerMeshes.forEach(mesh => {
    const thickness = h * mesh.userData.height_pct;
    zBounds.push(currentZ + thickness);
    currentZ += thickness;
  });

  // 4. Update layer positions
  layerMeshes.forEach((mesh, idx) => {
    const zCenter = zBounds[idx] + (zBounds[idx + 1] - zBounds[idx]) / 2;
    mesh.position.z = zCenter;
  });

  // 5. Update divider positions
  dividers.sort((a, b) => a.userData.dividerIndex - b.userData.dividerIndex);
  dividers.forEach((div, idx) => {
    div.position.z = zBounds[idx + 1];
  });

  // 6. Update UI sidebar
  emit(EVENTS.LAYERS_CHANGED, sd);
}
