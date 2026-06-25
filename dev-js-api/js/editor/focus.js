/**
 * @fileoverview focus.js — Focus system dimming inactive elements and highlighting active selections.
 */

import * as THREE from 'three';
import {
  shardMeshes,
  socketMeshes,
  shardDataMap
} from '../scene_builder.js';
import { store } from '../store/store.js';
import { THEME, RENDER_BINS } from '../rendering/theme.js';

// Pre-cached materials for shard body and wireframe to prevent GC thrashing and shader recompilation
const bodyMaterials = {
  active: new THREE.MeshStandardMaterial({
    color: 0x6366f1,
    transparent: false,
    opacity: 1.0,
    roughness: 0.6,
    metalness: 0.1,
  }),
  grey50: new THREE.MeshStandardMaterial({
    color: 0x555555,
    transparent: true,
    opacity: 0.5,
    depthWrite: false,
    roughness: 0.6,
    metalness: 0.1,
  }),
  grey20: new THREE.MeshStandardMaterial({
    color: 0x555555,
    transparent: true,
    opacity: 0.2,
    depthWrite: false,
    roughness: 0.6,
    metalness: 0.1,
  }),
  grey5: new THREE.MeshStandardMaterial({
    color: 0x555555,
    transparent: true,
    opacity: 0.05,
    depthWrite: false,
    roughness: 0.6,
    metalness: 0.1,
  }),
  invisible: new THREE.MeshBasicMaterial({
    color: 0x000000,
    transparent: true,
    opacity: 0.0,
    depthWrite: false
  }),
};

const wireMaterials = {
  active: new THREE.LineBasicMaterial({
    color: 0x6366f1,
    transparent: true,
    opacity: 0.85,
  }),
  grey50: new THREE.LineBasicMaterial({
    color: 0x555555,
    transparent: true,
    opacity: 0.5,
    depthWrite: false,
  }),
  grey20: new THREE.LineBasicMaterial({
    color: 0x555555,
    transparent: true,
    opacity: 0.2,
    depthWrite: false,
  }),
  grey5: new THREE.LineBasicMaterial({
    color: 0x555555,
    transparent: true,
    opacity: 0.05,
    depthWrite: false,
  }),
};

const greenWireMaterial = new THREE.LineBasicMaterial({
  color: 0x10b981, // Emerald Green
  transparent: true,
  opacity: 0.95
});

/**
 * Applies opacity and highlight filters to shard meshes based on current cascading selection.
 */
export function updateFocusVisuals() {
  const selShardKey = store.get('selectedShardKey');
  const selectedShardKeys = store.get('selectedShardKeys');
  const focusedShardKey = store.get('focusedShardKey');
  const focusedLevelId = store.get('focusedLevelId');
  const hiddenLevelIds = store.get('hiddenLevelIds') || new Set();
  const selectedDeptName = store.get('selectedDeptName');
  const placementData = store.get('placementData');

  if (!placementData) return;

  // 1. Resolve active level ID
  let activeLvlId = focusedLevelId;
  if (activeLvlId === null && focusedShardKey) {
    const shard = placementData.shards.find(s => s.key === focusedShardKey);
    if (shard) activeLvlId = shard.orbit;
  }
  if (activeLvlId === null && selectedDeptName) {
    const dept = placementData.departments.find(d => d.name === selectedDeptName);
    if (dept) activeLvlId = dept.orbit;
  }

  // 2. Resolve active department name
  let activeDeptName = selectedDeptName;
  if (activeDeptName === null && focusedShardKey) {
    const shard = placementData.shards.find(s => s.key === focusedShardKey);
    if (shard) activeDeptName = shard.dept;
  }

  const isLevelFocused = activeLvlId !== null;
  const isDeptFocused = activeDeptName !== null;
  const isShardFocused = focusedShardKey !== null;
  const isAnyFocusActive = isLevelFocused || isDeptFocused || isShardFocused;

  // 3. Update Shards Focus using pre-cached materials
  for (const [key, mesh] of shardMeshes.entries()) {
    const sd = shardDataMap.get(mesh.uuid);
    if (!sd) continue;

    const isHidden = hiddenLevelIds.has(sd.orbit);
    if (isHidden) {
      mesh.visible = false;
      continue;
    }
    mesh.visible = true;

    const body = mesh.userData.body;
    const mainWire = mesh.userData.mainWire;
    const label = mesh.userData.label;

    if (!body || !mainWire) continue;

    let showLayers = false;
    let targetBodyMat = bodyMaterials.active;
    let targetWireMat = wireMaterials.active;
    let labelOpacity = THEME.label.activeLevelOpacity;
    let labelVisible = true;
    let targetLayer = 0;
    let targetRenderOrder = RENDER_BINS.activeBody;

    if (isAnyFocusActive) {
      if (isShardFocused) {
        if (focusedShardKey === key) {
          showLayers = true;
          labelOpacity = THEME.label.activeLevelOpacity;
          labelVisible = true;
          targetLayer = 0;
          targetRenderOrder = RENDER_BINS.activeBody;
        } else {
          showLayers = false;
          labelVisible = false;
          if (sd.dept === activeDeptName) {
            targetBodyMat = bodyMaterials.grey50;
            targetWireMat = wireMaterials.grey50;
            targetLayer = 0; // Keep raycast active for shards of the same department
            targetRenderOrder = RENDER_BINS.inactive;
          } else if (Number(sd.orbit) === Number(activeLvlId)) {
            targetBodyMat = bodyMaterials.grey20;
            targetWireMat = wireMaterials.grey20;
            targetLayer = 1;
            targetRenderOrder = RENDER_BINS.inactive;
          } else {
            targetBodyMat = bodyMaterials.grey5;
            targetWireMat = wireMaterials.grey5;
            targetLayer = 1;
            targetRenderOrder = RENDER_BINS.inactive;
          }
        }
      } else if (isDeptFocused) {
        showLayers = false;
        if (sd.dept === activeDeptName) {
          targetBodyMat = bodyMaterials.active;
          targetWireMat = wireMaterials.active;
          labelOpacity = THEME.label.activeLevelOpacity;
          labelVisible = true;
          targetLayer = 0;
          targetRenderOrder = RENDER_BINS.activeBody;
        } else if (Number(sd.orbit) === Number(activeLvlId)) {
          targetBodyMat = bodyMaterials.grey50;
          targetWireMat = wireMaterials.grey50;
          labelVisible = false;
          targetLayer = 1;
          targetRenderOrder = RENDER_BINS.inactive;
        } else {
          targetBodyMat = bodyMaterials.grey20;
          targetWireMat = wireMaterials.grey20;
          labelVisible = false;
          targetLayer = 1;
          targetRenderOrder = RENDER_BINS.inactive;
        }
      } else if (isLevelFocused) {
        showLayers = false;
        if (Number(sd.orbit) === Number(activeLvlId)) {
          targetBodyMat = bodyMaterials.active;
          targetWireMat = wireMaterials.active;
          labelOpacity = THEME.label.activeLevelOpacity;
          labelVisible = true;
          targetLayer = 0;
          targetRenderOrder = RENDER_BINS.activeBody;
        } else {
          targetBodyMat = bodyMaterials.grey50;
          targetWireMat = wireMaterials.grey50;
          labelVisible = false;
          targetLayer = 1;
          targetRenderOrder = RENDER_BINS.inactive;
        }
      }
    } else {
      // Standard view: no active focus selection
      showLayers = false;
      targetBodyMat = bodyMaterials.active;
      targetWireMat = wireMaterials.active;
      labelOpacity = THEME.label.activeLevelOpacity;
      labelVisible = true;
      targetLayer = 0;
      targetRenderOrder = RENDER_BINS.activeBody;
    }

    const isSelected = (selShardKey === key || (selectedShardKeys && selectedShardKeys.has(key)));
    if (isSelected && !showLayers) {
      targetWireMat = greenWireMaterial;
    }

    // Apply visibility layers recursively to handle camera masks
    mesh.layers.set(targetLayer);
    mesh.traverse(child => child.layers.set(targetLayer));

    // Update body/wireframe material, visibility, and render order
    if (showLayers) {
      body.visible = true;
      body.material = bodyMaterials.invisible;
      mainWire.visible = true;
      mainWire.material = greenWireMaterial;
      mainWire.renderOrder = targetRenderOrder;

      // Show internal layers and dividers
      mesh.children.forEach(child => {
        if (child.userData) {
          if (child.userData.layerIndex !== undefined) {
            child.visible = true;
            child.material.opacity = 0.5;
            child.material.needsUpdate = true;
            const wire = child.children.find(c => c.name === "wireframe");
            if (wire) {
              wire.material.opacity = 0.8;
              wire.material.needsUpdate = true;
            }
          } else if (child.userData.isDivider) {
            child.visible = true;
            child.material.opacity = 0.0;
            child.material.needsUpdate = true;
            const border = child.children.find(c => c.name === "border");
            if (border) {
              border.material.opacity = 0.3;
              border.material.needsUpdate = true;
            }
          }
        }
      });
    } else {
      body.visible = true;
      mainWire.visible = true;
      body.material = targetBodyMat;
      mainWire.material = targetWireMat;
      body.renderOrder = targetRenderOrder;
      mainWire.renderOrder = targetRenderOrder;

      // Hide internal layers and dividers
      mesh.children.forEach(child => {
        if (child.userData && (child.userData.layerIndex !== undefined || child.userData.isDivider)) {
          child.visible = false;
        }
      });
    }

    // Apply label opacity/visibility
    if (label) {
      label.visible = labelVisible;
      if (labelVisible) {
        label.material.opacity = labelOpacity;
        label.material.needsUpdate = true;
      }
    }
  }

  // 4. Inactivate all socket visuals (not used in Composition mode)
  for (const [key, group] of socketMeshes.entries()) {
    group.visible = false;
  }
}

// Self-subscribe to store changes
store.on('focusedLevelId', () => {
  updateFocusVisuals();
});
store.on('hiddenLevelIds', () => {
  updateFocusVisuals();
});
store.on('selectedDeptName', () => {
  updateFocusVisuals();
});
store.on('selectedShardKey', () => {
  updateFocusVisuals();
});
store.on('selectedShardKeys', () => {
  updateFocusVisuals();
});
store.on('focusedShardKey', () => {
  updateFocusVisuals();
});
store.on('activeMode', () => {
  updateFocusVisuals();
});
store.on('placementData', () => {
  updateFocusVisuals();
});
