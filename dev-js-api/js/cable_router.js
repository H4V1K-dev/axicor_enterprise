/**
 * @fileoverview cable_router.js — Three.js adapter for the physical axon routing algorithm.
 * Follows INV-6 by converting Three.js constructs into plain objects for algorithms/cable_physics.js,
 * then wrapping the results back into THREE.CatmullRomCurve3 splines.
 */

import * as THREE from 'three';
import { relaxCables } from './algorithms/cable_physics.js';

// ─── Exported Tuning Parameters ─────────────────────────────────────
const defaults = {
  springK: 0.35,            // Spring tension (pulls cable straight, Laplacian smoothing)
  repulsionRad: 18.0,       // Shard repulsion radius in voxels (pushes cables away from boxes)
  repulsionStrength: 1.2,   // Shard repulsion force magnitude
  attractionRad: 22.0,      // Tract attraction radius in voxels (attracts nearby cables together)
  attractionStrength: 0.15, // Tract attraction force magnitude
  iterations: 45,           // Number of physical relaxation steps
  voxelSegmentLength: 10.0  // Subdivides the path to place a waypoint every N voxels of length
};

let savedConfig = null;
try {
  const saved = localStorage.getItem('axicor_router_config');
  if (saved) savedConfig = JSON.parse(saved);
} catch (e) {
  console.warn('Failed to load router config from localStorage:', e);
}

export const ROUTER_CONFIG = savedConfig ? { ...defaults, ...savedConfig } : defaults;

// ─── Helpers ────────────────────────────────────────────────────────

/**
 * Compute a world-space AABB from just the shard's OWN geometry,
 * ignoring children (sockets, handles, pins, labels, etc.)
 */
function getShardGeometryAABB(mesh) {
  const geo = mesh.geometry;
  if (!geo.boundingBox) geo.computeBoundingBox();
  const box = geo.boundingBox.clone();
  // Transform local AABB corners to world space
  mesh.updateMatrixWorld(true);
  box.applyMatrix4(mesh.matrixWorld);
  return box;
}

/**
 * Get the world-space exit normal for a socket face.
 * Top (faceSign = 1) grows strictly UP.
 * Bottom (faceSign = -1) grows strictly DOWN.
 */
function getWorldNormal(shardMesh, faceSign) {
  return new THREE.Vector3(0, faceSign, 0);
}

// ─── Main Entry Points (Adapters) ──────────────────────────────────

/**
 * Route all cables for a set of routes.
 *
 * @param {Array} routes - route data from routes.json
 * @param {Object} shardMeshes - key → THREE.Mesh map
 * @param {Object} socketMeshes - socketKey → THREE.Group map
 * @param {number} visScale - VIS_SCALE factor
 * @param {Object} options - { numCurvePoints, avoidanceBuffer }
 * @returns {Map<string, THREE.Vector3[]>} routeKey → array of curve points
 */
export function routeAllCables(routes, shardMeshes, socketMeshes, visScale, options = {}) {
  const {
    numCurvePoints = 48,
    avoidanceBuffer = 5.0   // in voxels
  } = options;

  const buffer = avoidanceBuffer * visScale;

  // 1. Convert obstacles to plain layout data
  const obstacles = [];
  for (const [key, mesh] of Object.entries(shardMeshes)) {
    const box = getShardGeometryAABB(mesh);
    // Expand box by buffer
    const min = { x: box.min.x - buffer, y: box.min.y - buffer, z: box.min.z - buffer };
    const max = { x: box.max.x + buffer, y: box.max.y + buffer, z: box.max.z + buffer };
    obstacles.push({
      key,
      box: { min, max }
    });
  }

  // 2. Convert endpoints to plain layout data
  const endpoints = [];
  routes.forEach(route => {
    const sockKeyFrom = `${route.from}.${route.from_socket}`;
    const sockKeyTo = `${route.to}.${route.to_socket}`;
    const socketGroupFrom = socketMeshes[sockKeyFrom];
    const socketGroupTo = socketMeshes[sockKeyTo];

    if (!socketGroupFrom || !socketGroupTo) return;

    const fromMesh = shardMeshes[route.from];
    const toMesh = shardMeshes[route.to];
    if (!fromMesh || !toMesh) return;

    const p0 = new THREE.Vector3();
    const p3 = new THREE.Vector3();
    socketGroupFrom.getWorldPosition(p0);
    socketGroupTo.getWorldPosition(p3);

    const fsFrom = socketGroupFrom.userData.faceSign;
    const fsTo = socketGroupTo.userData.faceSign;

    const exitNormalVec = getWorldNormal(fromMesh, fsFrom);
    const entryNormalVec = getWorldNormal(toMesh, fsTo);

    endpoints.push({
      routeKey: `${sockKeyFrom}→${sockKeyTo}`,
      p0: { x: p0.x, y: p0.y, z: p0.z },
      p3: { x: p3.x, y: p3.y, z: p3.z },
      exitNormal: { x: exitNormalVec.x, y: exitNormalVec.y, z: exitNormalVec.z },
      entryNormal: { x: entryNormalVec.x, y: entryNormalVec.y, z: entryNormalVec.z },
      fromShardKey: route.from,
      toShardKey: route.to
    });
  });

  // 3. Call pure relaxation algorithm
  const waypointMap = relaxCables(endpoints, obstacles, ROUTER_CONFIG, visScale);

  // 4. Map waypoint results back to THREE.CatmullRomCurve3 curves
  const curveMap = new Map();
  for (const [routeKey, pts] of waypointMap.entries()) {
    const threePoints = pts.map(p => new THREE.Vector3(p.x, p.y, p.z));
    const curve = new THREE.CatmullRomCurve3(threePoints);
    curveMap.set(routeKey, curve.getPoints(numCurvePoints));
  }

  return curveMap;
}

/**
 * Route a single cable between two specific pins.
 */
export function routeSingleCable(p0, p3, fromMesh, toMesh, fsFrom, fsTo, shardMeshes, fromShardKey, toShardKey, visScale, numPoints = 24) {
  const routes = [{
    from: fromShardKey,
    from_socket: 'single',
    to: toShardKey,
    to_socket: 'single'
  }];

  const mockSocketMeshes = {
    [`${fromShardKey}.single`]: {
      getWorldPosition: (outVec) => outVec.copy(p0),
      userData: { faceSign: fsFrom }
    },
    [`${toShardKey}.single`]: {
      getWorldPosition: (outVec) => outVec.copy(p3),
      userData: { faceSign: fsTo }
    }
  };

  const curveMap = routeAllCables(routes, shardMeshes, mockSocketMeshes, visScale, { numCurvePoints: numPoints });
  return curveMap.get(`${fromShardKey}.single→${toShardKey}.single`);
}
