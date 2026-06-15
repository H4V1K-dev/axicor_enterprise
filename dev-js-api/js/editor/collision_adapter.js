import { shardMeshes, shardDataMap, VIS_SCALE } from '../scene_builder.js';
import { checkShardCollision as pureCheckShardCollision } from '../algorithms/collision.js';
import { store } from '../store/store.js';

/**
 * Adapter mapping Three.js shard coordinates to the pure AABB collision check.
 * @param {string} movedKey
 * @param {import("three").Vector3} newPos
 * @returns {boolean} True if there is a collision
 */
export function checkShardCollision(movedKey, newPos, newSize) {
  let w = 10;
  let d = 10;
  let h = 10;
  let orbit = 1;
  let y = 0;

  if (movedKey) {
    const movedMesh = shardMeshes[movedKey];
    if (movedMesh) {
      const movedData = shardDataMap[movedMesh.uuid];
      if (movedData) {
        w = movedData.size.w;
        d = movedData.size.d;
        h = movedData.size.h;
        orbit = movedData.orbit;
        y = movedData.position.y;
      }
    }
  }

  if (newSize) {
    if (newSize.w !== undefined) w = newSize.w;
    if (newSize.d !== undefined) d = newSize.d;
    if (newSize.h !== undefined) h = newSize.h;
    if (newSize.orbit !== undefined) orbit = newSize.orbit;
  }

  const placementData = store.get('placementData');
  const orbitData = placementData ? placementData.orbits.find(o => o.index === orbit) : null;
  const radius = orbitData ? orbitData.radius : 0.0;

  const voxelY = newPos ? (newPos.y / VIS_SCALE) + radius : y;

  const movedBox = {
    x: newPos ? newPos.x / VIS_SCALE : 0,
    y: voxelY,
    z: newPos ? newPos.z / VIS_SCALE : 0,
    w: w,
    d: d,
    h: h,
    orbit: orbit
  };

  const otherBoxes = [];
  for (const [key, mesh] of Object.entries(shardMeshes)) {
    if (key === movedKey) continue;
    const otherData = shardDataMap[mesh.uuid];
    if (!otherData) continue;
    otherBoxes.push({
      key,
      x: mesh.position.x / VIS_SCALE,
      y: otherData.position.y,
      z: mesh.position.z / VIS_SCALE,
      w: otherData.size.w,
      d: otherData.size.d,
      h: otherData.size.h,
      orbit: otherData.orbit
    });
  }

  return pureCheckShardCollision(movedBox, otherBoxes);
}
