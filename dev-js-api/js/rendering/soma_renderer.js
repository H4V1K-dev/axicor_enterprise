/**
 * @fileoverview soma_renderer.js — Spawns and manages 3D voxel neuron populations (somas) for selected shards.
 */

import * as THREE from 'three';
import { shardMeshes, shardDataMap, VIS_SCALE, SOMA_COLORS } from '../scene_builder.js';
import { store } from '../store/store.js';

export let somaGroup = null;

// Seeded random helper
function seededRandom(seed) {
  let x = Math.sin(seed) * 10000;
  return x - Math.floor(x);
}

function hashStringToInt(str) {
  let hash = 0;
  for (let i = 0; i < str.length; i++) {
    hash = str.charCodeAt(i) + ((hash << 5) - hash);
  }
  return Math.abs(hash);
}

function getSomaColor(type) {
  if (SOMA_COLORS[type] !== undefined) return SOMA_COLORS[type];
  const hash = hashStringToInt(type);
  const r = (hash & 0xFF0000) >> 16;
  const g = (hash & 0x00FF00) >> 8;
  const b = hash & 0x0000FF;
  return new THREE.Color(r/255, g/255, b/255).offsetHSL(0, 0.1, 0.1).getHex();
}

/**
 * Spawns soma InstancedMesh elements inside the layer segments of a selected shard.
 * @param {string} shardKey 
 */
export function spawnSomasForShard(shardKey) {
  clearSomas();

  const shardMesh = shardMeshes[shardKey];
  if (!shardMesh) return;

  const sd = shardDataMap[shardMesh.uuid];
  if (!sd) return;

  somaGroup = new THREE.Group();
  somaGroup.name = "somas";
  shardMesh.add(somaGroup);

  const w = sd.size.w * VIS_SCALE;
  const d = sd.size.d * VIS_SCALE;
  const h = sd.size.h * VIS_SCALE;

  // Collect layer meshes to know their boundaries
  const layerMeshes = [];
  shardMesh.traverse(child => {
    if (child.userData && child.userData.layerIndex !== undefined) {
      layerMeshes.push(child);
    }
  });
  layerMeshes.sort((a, b) => a.userData.layerIndex - b.userData.layerIndex);

  // Set up geometry (voxels soma representation)
  const somaSize = 0.22 * VIS_SCALE;
  const somaGeo = new THREE.BoxGeometry(somaSize, somaSize, somaSize);

  let currentY = -h / 2;
  layerMeshes.forEach((layerMesh) => {
    const layer_vis_h = h * layerMesh.userData.height_pct;
    const layerName = layerMesh.userData.layerName;

    // Find populations for this layer
    const pops = sd.populations ? sd.populations.filter(p => p[0] === layerName) : [];
    
    pops.forEach(pop => {
      const popType = pop[1];
      const colorHex = getSomaColor(popType);
      const mat = new THREE.MeshStandardMaterial({
        color: colorHex,
        roughness: 0.3,
        metalness: 0.1,
        transparent: true,
        opacity: 0.9
      });

      // Seeding
      const placementData = store.get('placementData');
      const projectSeed = (placementData && placementData.seed !== undefined) ? placementData.seed : 42;
      let seed = hashStringToInt(projectSeed.toString() + shardKey + layerName + popType);

      // Determine count based on voxel volume
      const layer_voxel_h = sd.size.h * layerMesh.userData.height_pct;
      const voxel_vol = sd.size.w * sd.size.d * layer_voxel_h;
      const count = Math.max(15, Math.min(50, Math.floor(voxel_vol * 0.0002)));

      const instancedMesh = new THREE.InstancedMesh(somaGeo, mat, count);
      const dummy = new THREE.Object3D();

      for (let i = 0; i < count; i++) {
        // Generate within layer bounds with margin
        const localX = (seededRandom(seed++) - 0.5) * (w - 1.5 * VIS_SCALE);
        const localZ = (seededRandom(seed++) - 0.5) * (d - 1.5 * VIS_SCALE);
        const localY = currentY + 0.1 * layer_vis_h + seededRandom(seed++) * (layer_vis_h * 0.8);

        dummy.position.set(localX, localY, localZ);
        dummy.updateMatrix();
        instancedMesh.setMatrixAt(i, dummy.matrix);
      }
      instancedMesh.instanceMatrix.needsUpdate = true;
      somaGroup.add(instancedMesh);
    });

    currentY += layer_vis_h;
  });
}

/**
 * Removes and disposes existing somas from the scene.
 */
export function clearSomas() {
  if (somaGroup) {
    if (somaGroup.parent) {
      somaGroup.parent.remove(somaGroup);
    }
    somaGroup.traverse(child => {
      if (child.geometry) child.geometry.dispose();
      if (child.material) {
        if (Array.isArray(child.material)) {
          child.material.forEach(m => m.dispose());
        } else {
          child.material.dispose();
        }
      }
    });
    somaGroup = null;
  }
}
