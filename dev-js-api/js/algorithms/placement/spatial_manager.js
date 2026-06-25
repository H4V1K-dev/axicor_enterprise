/**
 * @fileoverview spatial_manager.js — Pure math calculations of levels and departments spatial bounds (AABBs) in Three.js coordinates.
 */

import { store } from '../../store/store.js';

export const levelAABBs = new Map(); // levelId -> { x, y, z, w, h, d }
export const deptAABBs = new Map();  // deptName@orbit -> { x, y, z, w, h, d }

/**
 * Re-computes dynamic spatial AABB boundaries for levels and departments.
 * Keeps computed coordinates decoupled from Three.js Object3D hierarchy.
 * 
 * @param {any} placementData - The raw placement data containing levels, departments, and shards.
 * @param {number} VIS_SCALE - Visual scaling factor.
 */
export function recomputeSpatialLayout(placementData, VIS_SCALE = 1.0) {
  levelAABBs.clear();
  deptAABBs.clear();

  if (!placementData) return;

  const levels = placementData.levels || [];
  const depts = placementData.departments || [];
  const shards = placementData.shards || [];

  const levelsMap = new Map();
  levels.forEach(lvl => {
    levelsMap.set(Number(lvl.id), lvl);
  });

  // 1. Group shards by Level and by Department compound keys
  const shardsByLvlId = new Map();
  const shardsByDeptKey = new Map();

  shards.forEach(s => {
    const lvlId = Number(s.orbit);
    const deptKey = `${s.dept}@${lvlId}`;

    if (!shardsByLvlId.has(lvlId)) shardsByLvlId.set(lvlId, []);
    shardsByLvlId.get(lvlId).push(s);

    if (!shardsByDeptKey.has(deptKey)) shardsByDeptKey.set(deptKey, []);
    shardsByDeptKey.get(deptKey).push(s);
  });

  // 2. Compute Level AABBs
  levels.forEach(lvl => {
    const lvlId = Number(lvl.id);
    const lvlShards = shardsByLvlId.get(lvlId) || [];

    if (lvlShards.length === 0) {
      // Fallback or skip empty level
      return;
    }

    let xMin = Infinity, xMax = -Infinity;
    let zMin = Infinity, zMax = -Infinity;

    lvlShards.forEach(s => {
      xMin = Math.min(xMin, s.position.x);
      xMax = Math.max(xMax, s.position.x + s.size.w);
      zMin = Math.min(zMin, s.position.z);
      zMax = Math.max(zMax, s.position.z + s.size.d);
    });

    const w = (xMax - xMin) * VIS_SCALE;
    const h = lvl.height * VIS_SCALE;
    const d = (zMax - zMin) * VIS_SCALE;

    const x = ((xMin + xMax) / 2) * VIS_SCALE;
    const y = (lvl.z_start + lvl.height / 2) * VIS_SCALE;
    const z = ((zMin + zMax) / 2) * VIS_SCALE;

    levelAABBs.set(lvlId, { x, y, z, w, h, d });
  });

  // 3. Compute Department AABBs with Fallback for empty ones
  depts.forEach(dept => {
    const lvlId = Number(dept.orbit);
    const lvl = levelsMap.get(lvlId);
    if (!lvl) return;

    const deptKey = `${dept.name}@${lvlId}`;
    const deptShards = shardsByDeptKey.get(deptKey) || [];

    if (deptShards.length === 0) {
      // Fallback logic for empty departments to prevent collapse and keep raycaster target available
      const w = 16 * VIS_SCALE; // Default 16-voxel width placeholder
      const h = lvl.height * VIS_SCALE;
      const d = 16 * VIS_SCALE; // Default 16-voxel depth placeholder

      // Center in level or use last known position if preserved
      let x = 0;
      let z = 0;

      if (dept.position && dept.position.x !== undefined && (dept.position.z !== undefined || dept.position.y !== undefined)) {
        const depthCoord = dept.position.z !== undefined ? dept.position.z : dept.position.y;
        x = (dept.position.x + 8) * VIS_SCALE;
        z = (depthCoord + 8) * VIS_SCALE;
      } else {
        // Place in center of level bounds if available, or just at origin
        const lvlAABB = levelAABBs.get(lvlId);
        if (lvlAABB) {
          x = lvlAABB.x;
          z = lvlAABB.z;
        } else {
          x = 0;
          z = 0;
        }
      }

      const y = (lvl.z_start + lvl.height / 2) * VIS_SCALE;

      deptAABBs.set(deptKey, { x, y, z, w, h, d });
    } else {
      // Compute actual AABB bounding box for filled departments
      let xMin = Infinity, xMax = -Infinity;
      let zMin = Infinity, zMax = -Infinity;

      deptShards.forEach(s => {
        xMin = Math.min(xMin, s.position.x);
        xMax = Math.max(xMax, s.position.x + s.size.w);
        zMin = Math.min(zMin, s.position.z);
        zMax = Math.max(zMax, s.position.z + s.size.d);
      });

      const w = (xMax - xMin) * VIS_SCALE;
      const h = lvl.height * VIS_SCALE;
      const d = (zMax - zMin) * VIS_SCALE;

      const x = ((xMin + xMax) / 2) * VIS_SCALE;
      const y = (lvl.z_start + lvl.height / 2) * VIS_SCALE;
      const z = ((zMin + zMax) / 2) * VIS_SCALE;

      deptAABBs.set(deptKey, { x, y, z, w, h, d });
    }
  });
}

/**
 * Re-computes dynamic spatial AABB boundaries for levels and departments based on 3D mesh positions.
 * Called during active gizmo/pointer translation to allow real-time wireframe adjustment.
 * 
 * @param {Map<string, THREE.Group>} shardMeshes - Active 3D meshes map.
 * @param {Map<string, any>} shardDataMap - Mesh UUID to raw shard data map.
 * @param {Array} levels - Levels list.
 * @param {Array} depts - Departments list.
 * @param {number} VIS_SCALE - Visual scaling factor.
 */
export function recomputeSpatialLayoutFromMeshes(shardMeshes, shardDataMap, levels, depts, VIS_SCALE = 1.0) {
  levelAABBs.clear();
  deptAABBs.clear();

  const levelsMap = new Map();
  levels.forEach(lvl => {
    levelsMap.set(Number(lvl.id), lvl);
  });

  const levelBounds = new Map(); // lvlId -> { xMin, xMax, zMin, zMax }
  const deptBounds = new Map();  // deptKey -> { xMin, xMax, zMin, zMax }

  // Inspect current mesh positions on the scene to calculate actual boundary boxes
  for (const [key, shardGroup] of shardMeshes.entries()) {
    const sd = shardDataMap.get(shardGroup.uuid);
    if (!sd) continue;

    const w = sd.size.w;
    const d = sd.size.d;
    const h = sd.size.h;

    // Decode current AABB min in voxels
    const px = shardGroup.position.x / VIS_SCALE - w / 2;
    const pz = shardGroup.position.z / VIS_SCALE - d / 2; // Three.js Z is depth

    const lvlId = Number(sd.orbit);
    const deptKey = `${sd.dept}@${lvlId}`;

    // Update level bounds
    if (!levelBounds.has(lvlId)) {
      levelBounds.set(lvlId, { xMin: px, xMax: px + w, zMin: pz, zMax: pz + d });
    } else {
      const box = levelBounds.get(lvlId);
      box.xMin = Math.min(box.xMin, px);
      box.xMax = Math.max(box.xMax, px + w);
      box.zMin = Math.min(box.zMin, pz);
      box.zMax = Math.max(box.zMax, pz + d);
    }

    // Update dept bounds
    if (!deptBounds.has(deptKey)) {
      deptBounds.set(deptKey, { xMin: px, xMax: px + w, zMin: pz, zMax: pz + d });
    } else {
      const box = deptBounds.get(deptKey);
      box.xMin = Math.min(box.xMin, px);
      box.xMax = Math.max(box.xMax, px + w);
      box.zMin = Math.min(box.zMin, pz);
      box.zMax = Math.max(box.zMax, pz + d);
    }
  }

  // Set computed levels AABBs
  levels.forEach(lvl => {
    const lvlId = Number(lvl.id);
    const box = levelBounds.get(lvlId);
    if (!box) return;

    const w = (box.xMax - box.xMin) * VIS_SCALE;
    const h = lvl.height * VIS_SCALE;
    const d = (box.zMax - box.zMin) * VIS_SCALE;

    const x = ((box.xMin + box.xMax) / 2) * VIS_SCALE;
    const y = (lvl.z_start + lvl.height / 2) * VIS_SCALE;
    const z = ((box.zMin + box.zMax) / 2) * VIS_SCALE;

    levelAABBs.set(lvlId, { x, y, z, w, h, d });
  });

  // Set computed departments AABBs with Fallback for empty ones
  depts.forEach(dept => {
    const lvlId = Number(dept.orbit);
    const lvl = levelsMap.get(lvlId);
    if (!lvl) return;

    const deptKey = `${dept.name}@${lvlId}`;
    const box = deptBounds.get(deptKey);

    if (!box) {
      // Fallback logic for empty departments
      const w = 16 * VIS_SCALE;
      const h = lvl.height * VIS_SCALE;
      const d = 16 * VIS_SCALE;

      let x = 0;
      let z = 0;

      if (dept.position && dept.position.x !== undefined && (dept.position.z !== undefined || dept.position.y !== undefined)) {
        const depthCoord = dept.position.z !== undefined ? dept.position.z : dept.position.y;
        x = (dept.position.x + 8) * VIS_SCALE;
        z = (depthCoord + 8) * VIS_SCALE;
      } else {
        const lvlAABB = levelAABBs.get(lvlId);
        if (lvlAABB) {
          x = lvlAABB.x;
          z = lvlAABB.z;
        }
      }
      const y = (lvl.z_start + lvl.height / 2) * VIS_SCALE;

      deptAABBs.set(deptKey, { x, y, z, w, h, d });
    } else {
      const w = (box.xMax - box.xMin) * VIS_SCALE;
      const h = lvl.height * VIS_SCALE;
      const d = (box.zMax - box.zMin) * VIS_SCALE;

      const x = ((box.xMin + box.xMax) / 2) * VIS_SCALE;
      const y = (lvl.z_start + lvl.height / 2) * VIS_SCALE;
      const z = ((box.zMin + box.zMax) / 2) * VIS_SCALE;

      deptAABBs.set(deptKey, { x, y, z, w, h, d });
    }
  });
}

