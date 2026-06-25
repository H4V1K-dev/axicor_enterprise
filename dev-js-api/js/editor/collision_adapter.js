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
  let movedData = null;

  if (movedKey) {
    const movedMesh = shardMeshes.get(movedKey);
    if (movedMesh) {
      movedData = shardDataMap.get(movedMesh.uuid);
      if (movedData) {
        w = movedData.size.w;
        d = movedData.size.d;
        h = movedData.size.h;
        orbit = movedData.orbit;
      }
    }
  }

  if (newSize) {
    if (newSize.w !== undefined) w = newSize.w;
    if (newSize.d !== undefined) d = newSize.d;
    if (newSize.h !== undefined) h = newSize.h;
    if (newSize.orbit !== undefined) orbit = newSize.orbit;
  }

  // Calculate AABB min coordinates in Three.js system (x=X, y=Y/height, z=Z/depth)
  const movedBox = {
    x: newPos ? (newPos.x / VIS_SCALE) - w / 2 : (movedData ? movedData.position.x : 0),
    y: newPos ? (newPos.y / VIS_SCALE) - h / 2 : (movedData ? movedData.position.y : 0),
    z: newPos ? (newPos.z / VIS_SCALE) - d / 2 : (movedData ? movedData.position.z : 0),
    w: w,
    d: d,
    h: h
  };

  const otherBoxes = [];
  for (const [key, mesh] of shardMeshes.entries()) {
    if (key === movedKey) continue;
    const otherData = shardDataMap.get(mesh.uuid);
    if (!otherData) continue;
    
    // Only check collision with shards on the same orbit level
    if (Number(otherData.orbit) !== Number(orbit)) continue;

    otherBoxes.push({
      key,
      x: otherData.position.x, // Three.js X (AABB min)
      y: otherData.position.y, // Three.js Y (height, AABB min)
      z: otherData.position.z, // Three.js Z (depth, AABB min)
      w: otherData.size.w,
      d: otherData.size.d,
      h: otherData.size.h
    });
  }

  return pureCheckShardCollision(movedBox, otherBoxes);
}
