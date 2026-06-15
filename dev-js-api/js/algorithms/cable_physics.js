/**
 * @fileoverview Pure physical relaxation algorithm for axon/cable routing.
 * Decoupled from Three.js and the DOM.
 */

import {
  vec3_add,
  vec3_sub,
  vec3_scale,
  vec3_dist,
  vec3_lerp,
  vec3_clone,
  vec3_len
} from './vec3.js';

/**
 * @typedef {import("../contracts/types.js").Vec3} Vec3
 */

/**
 * @typedef {Object} CableEndpoint
 * @property {string} routeKey - Unique connection path identifier
 * @property {Vec3} p0 - Starting socket position
 * @property {Vec3} p3 - Ending socket position
 * @property {Vec3} exitNormal - Starting face normal direction
 * @property {Vec3} entryNormal - Ending face normal direction
 * @property {string} fromShardKey - Origin shard key
 * @property {string} toShardKey - Destination shard key
 */

/**
 * @typedef {Object} AABBObstacle
 * @property {string} key - Shard identifier
 * @property {{min: Vec3, max: Vec3}} box - Expanded AABB bounding coordinates
 */

/**
 * @typedef {Object} RouterConfig
 * @property {number} springK - Spring tension coefficient
 * @property {number} repulsionRad - Box repulsion radius in voxels
 * @property {number} repulsionStrength - Repulsion force coefficient
 * @property {number} attractionRad - Parallel tract bundling radius in voxels
 * @property {number} attractionStrength - Bundling force coefficient
 * @property {number} iterations - Physical loop steps count
 * @property {number} voxelSegmentLength - Voxel steps spacing for points
 */

function containsPoint(box, pt) {
  return pt.x >= box.min.x && pt.x <= box.max.x &&
         pt.y >= box.min.y && pt.y <= box.max.y &&
         pt.z >= box.min.z && pt.z <= box.max.z;
}

function clampPoint(box, pt) {
  return {
    x: Math.max(box.min.x, Math.min(box.max.x, pt.x)),
    y: Math.max(box.min.y, Math.min(box.max.y, pt.y)),
    z: Math.max(box.min.z, Math.min(box.max.z, pt.z))
  };
}

function pushPointOut(pt, box, minY) {
  const faces = [
    { axis: 'x', val: box.min.x, dist: pt.x - box.min.x, sign: -1 },
    { axis: 'x', val: box.max.x, dist: box.max.x - pt.x, sign: 1 },
    { axis: 'y', val: box.min.y, dist: pt.y - box.min.y, sign: -1 },
    { axis: 'y', val: box.max.y, dist: box.max.y - pt.y, sign: 1 },
    { axis: 'z', val: box.min.z, dist: pt.z - box.min.z, sign: -1 },
    { axis: 'z', val: box.max.z, dist: box.max.z - pt.z, sign: 1 },
  ];

  const scored = faces.map(f => ({
    ...f,
    score: f.dist * (f.axis === 'y' && f.sign === 1 ? 0.6 : 1.0)
  }));

  scored.sort((a, b) => a.score - b.score);
  const best = scored[0];

  pt[best.axis] = best.val;
  if (best.axis === 'y') {
    pt.y = Math.max(minY, pt.y);
  }
}

function computeRawWaypoints(p0, p3, exitNormal, entryNormal, config, visScale) {
  const exitDist = 12.0 * visScale;
  const entryDist = 12.0 * visScale;

  const exitPt = vec3_add(p0, vec3_scale(exitNormal, exitDist));
  const entryPt = vec3_add(p3, vec3_scale(entryNormal, entryDist));

  const dist = vec3_dist(exitPt, entryPt);
  const voxelSegmentLength = config.voxelSegmentLength * visScale;
  const numSegments = Math.max(3, Math.round(dist / voxelSegmentLength));
  const numMidpoints = numSegments - 1;

  const waypoints = [];
  for (let i = 1; i <= numMidpoints; i++) {
    const t = i / (numMidpoints + 1);
    waypoints.push(vec3_lerp(exitPt, entryPt, t));
  }

  return {
    anchor0: vec3_clone(p0),
    anchor1: vec3_clone(p3),
    waypoints: [exitPt, ...waypoints, entryPt]
  };
}

/**
 * Computes physically relaxed routes for multiple cables.
 * 
 * @param {CableEndpoint[]} endpoints
 * @param {AABBObstacle[]} obstacles
 * @param {RouterConfig} config
 * @param {number} visScale
 * @returns {Map<string, Vec3[]>} Map of routeKey to array of waypoint coordinates
 */
export function relaxCables(endpoints, obstacles, config, visScale) {
  const springK = config.springK;
  const repulsionRad = config.repulsionRad * visScale;
  const repulsionStrength = config.repulsionStrength;
  const attractionRad = config.attractionRad * visScale;
  const attractionStrength = config.attractionStrength;
  const minY = 3.0 * visScale;

  // 1. Initial Raw Waypoints
  const allCables = endpoints.map(ep => {
    const cable = computeRawWaypoints(ep.p0, ep.p3, ep.exitNormal, ep.entryNormal, config, visScale);
    cable._routeKey = ep.routeKey;
    cable._fromShardKey = ep.fromShardKey;
    cable._toShardKey = ep.toShardKey;
    return cable;
  });

  // 2. Physical Relaxation Loop
  for (let iter = 0; iter < config.iterations; iter++) {
    const forces = allCables.map(cable => {
      const N = cable.waypoints.length;
      const f = [];
      for (let i = 0; i < N; i++) f.push({ x: 0, y: 0, z: 0 });
      return f;
    });

    allCables.forEach((cable, cIdx) => {
      const N = cable.waypoints.length;
      const wps = cable.waypoints;

      const relevantBoxes = obstacles
        .filter(o => o.key !== cable._fromShardKey && o.key !== cable._toShardKey)
        .map(o => o.box);

      for (let i = 1; i < N - 1; i++) {
        const pt = wps[i];

        // 1. Spring Force (Laplacian smoothing)
        const sumNeighbors = vec3_add(wps[i-1], wps[i+1]);
        const selfScaled = vec3_scale(pt, -2);
        const fSpring = vec3_scale(vec3_add(sumNeighbors, selfScaled), springK);
        forces[cIdx][i] = vec3_add(forces[cIdx][i], fSpring);

        // 2. Shard Repulsion Force
        relevantBoxes.forEach(box => {
          if (containsPoint(box, pt)) {
            pushPointOut(pt, box, minY);
          } else {
            const closestPt = clampPoint(box, pt);
            const d = vec3_dist(pt, closestPt);
            if (d < repulsionRad) {
              const dir = vec3_sub(pt, closestPt);
              const distToClamp = vec3_len(dir);
              if (distToClamp > 0.001) {
                const normDir = vec3_scale(dir, 1 / distToClamp);
                const mag = repulsionStrength * (1.0 - d / repulsionRad) * visScale;
                forces[cIdx][i] = vec3_add(forces[cIdx][i], vec3_scale(normDir, mag));
              }
            }
          }
        });

        // 3. Tract Attraction Force (Organically bundle parallel cables)
        if (allCables.length > 1) {
          const t = i / (N - 1);
          allCables.forEach((otherCable, oIdx) => {
            if (oIdx === cIdx) return;
            const otherWps = otherCable.waypoints;
            const otherN = otherWps.length;
            const otherI = Math.round(t * (otherN - 1));
            const clampedI = Math.max(1, Math.min(otherI, otherN - 2));
            const otherPt = otherWps[clampedI];

            const d = vec3_dist(pt, otherPt);
            if (d > 0.01 && d < attractionRad) {
              const dir = vec3_sub(otherPt, pt);
              forces[cIdx][i] = vec3_add(forces[cIdx][i], vec3_scale(dir, attractionStrength));
            }
          });
        }
      }
    });

    // Apply forces
    allCables.forEach((cable, cIdx) => {
      const N = cable.waypoints.length;
      for (let i = 1; i < N - 1; i++) {
        cable.waypoints[i] = vec3_add(cable.waypoints[i], forces[cIdx][i]);
        cable.waypoints[i].y = Math.max(minY, cable.waypoints[i].y);
      }
    });
  }

  // 3. Collect relaxed waypoint paths
  const result = new Map();
  allCables.forEach(cable => {
    const fullPoints = [cable.anchor0, ...cable.waypoints, cable.anchor1];
    result.set(cable._routeKey, fullPoints);
  });

  return result;
}
