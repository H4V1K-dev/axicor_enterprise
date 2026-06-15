/**
 * @fileoverview validator.js — Three.js adapter for topology static validation.
 * Follows INV-6 by mapping Three.js meshes to plain objects, then executing
 * the pure validator logic in algorithms/validation.js.
 */

import { validateTopology } from './algorithms/validation.js';

/**
 * Runs validation on routes, shard meshes, and socket meshes.
 *
 * @param {Array} routes - Connection routes data
 * @param {Object} shardMeshes - key -> THREE.Mesh mapping
 * @param {Object} socketMeshes - socketKey -> THREE.Group mapping
 * @param {number} visScale - VIS_SCALE factor
 * @returns {Array} List of validation issues
 */
export function runValidation(routes, shardMeshes, socketMeshes, visScale) {
  if (!routes || !shardMeshes || !socketMeshes) {
    return [];
  }

  // 1. Map shardMeshes to PlainShard format
  const shards = new Map();
  for (const [key, mesh] of Object.entries(shardMeshes)) {
    shards.set(key, {
      key,
      size: {
        w: mesh.geometry.parameters.width / visScale,
        d: mesh.geometry.parameters.height / visScale, // local Y is depth in visualizer
        h: mesh.geometry.parameters.depth / visScale
      }
    });
  }

  // 2. Map socketMeshes to PlainSocket format
  const sockets = new Map();
  for (const [key, group] of Object.entries(socketMeshes)) {
    const shardMesh = shardMeshes[group.userData.shardKey];
    const shardDepth = shardMesh ? shardMesh.geometry.parameters.depth : 0;
    const defaultZ = group.userData.faceSign * (shardDepth / (2 * visScale));
    sockets.set(key, {
      socketKey: key,
      shardKey: group.userData.shardKey,
      socketName: group.userData.socketName,
      width: group.userData.width,
      height: group.userData.height,
      pitch: group.userData.pitch,
      offset: {
        x: group.userData.originalOffset.x,
        y: group.userData.originalOffset.y,
        z: group.userData.originalOffset.z !== undefined ? group.userData.originalOffset.z : defaultZ
      },
      faceSign: group.userData.faceSign
    });
  }

  // 3. Execute pure validation rules
  return validateTopology(routes, shards, sockets);
}
