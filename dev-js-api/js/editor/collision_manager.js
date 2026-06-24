import * as THREE from 'three';
import { shardMeshes, socketMeshes, VIS_SCALE } from '../scene_builder.js';
const routesGroup = null;
const routeEditorGroup = null;
import { store } from '../store/store.js';

/**
 * Unified resolver for raycast hits on interactive objects.
 * Resolves priority order:
 * 1. Route Control Points (nodes)
 * 2. Sockets (resolving X-ray transparency selection)
 * 3. Shards
 * 4. Routes (cables)
 * 
 * @param {THREE.Raycaster} raycaster
 * @returns {Object|null} Selected interactive target or null
 */
export function resolveRaycastHit(raycaster) {
  const socketsList = Object.values(socketMeshes);
  const shardsList = Object.values(shardMeshes);
  
  // Construct list of intersectable objects
  const intersectTargets = [...socketsList, ...shardsList];
  if (routeEditorGroup && routeEditorGroup.children.length > 0) {
    intersectTargets.push(...routeEditorGroup.children);
  }
  if (routesGroup && routesGroup.children.length > 0) {
    intersectTargets.push(...routesGroup.children);
  }

  const hits = raycaster.intersectObjects(intersectTargets, true);
  if (hits.length === 0) return null;

  // Filter out helper wireframes, labels and dividers
  const filteredHits = hits.filter(hit => {
    const obj = hit.object;
    // Skip divider planes
    if (obj.userData && obj.userData.isDivider) return false;
    // Skip main wireframe outlines
    if (obj.name === 'main_wireframe' || obj.name === 'wireframe' || obj.name === 'ghost_wireframe') return false;
    // Skip sprites (labels)
    if (obj instanceof THREE.Sprite) return false;
    return true;
  });

  if (filteredHits.length === 0) return null;

  // Evaluate the collision stack (up to 5 closest elements)
  const maxHitsToCheck = Math.min(filteredHits.length, 5);
  const candidates = [];

  for (let i = 0; i < maxHitsToCheck; i++) {
    const hit = filteredHits[i];
    const obj = hit.object;

    // 1. Route Control Points (highest priority, always returned immediately if hit first)
    let current = obj;
    let controlPointObj = null;
    while (current) {
      if (current.userData && current.userData.isControlPoint) {
        controlPointObj = current;
        break;
      }
      current = current.parent;
    }
    if (controlPointObj) {
      return { type: 'control_point', object: controlPointObj, hit };
    }

    // 2. Sockets
    current = obj;
    let socketGroupObj = null;
    while (current) {
      if (current.userData && current.userData.socketKey) {
        socketGroupObj = current;
        break;
      }
      current = current.parent;
    }

    if (socketGroupObj) {
      const faceSign = socketGroupObj.userData.faceSign || 1;
      const shardKey = socketGroupObj.userData.shardKey;
      const shardMeshObj = shardMeshes[shardKey];

      if (shardMeshObj) {
        // Calculate world normal of the socket face based on its faceSign (direction along local Z)
        const localNormal = new THREE.Vector3(0, 0, faceSign);
        const shardQuaternion = shardMeshObj.getWorldQuaternion(new THREE.Quaternion());
        const worldNormal = localNormal.applyQuaternion(shardQuaternion);

        // Dot product with raycaster direction. If dot > 0, the face is pointing away from the camera.
        const dot = worldNormal.dot(raycaster.ray.direction);
        if (dot > 0.0) {
          continue; // Backface culling: ignore socket facing away from the camera
        }
      }

      candidates.push({
        type: 'socket',
        key: socketGroupObj.userData.socketKey,
        object: socketGroupObj,
        distance: hit.distance,
        shardKey,
        hit
      });
      continue;
    }

    // 3. Shards
    current = obj;
    let shardMeshObj = null;
    while (current) {
      if (shardsList.includes(current)) {
        shardMeshObj = current;
        break;
      }
      current = current.parent;
    }

    if (shardMeshObj) {
      const key = Object.keys(shardMeshes).find(k => shardMeshes[k] === shardMeshObj);
      if (key) {
        // Determine opacity based on selection focus. A shard is physically opaque unless it is selected (or has a selected socket) and we are viewing its inner layers.
        const selShardKey = store.get('selectedShardKey');
        const selSocketKey = store.get('selectedSocketKey');
        const selSocketGroup = selSocketKey ? socketMeshes[selSocketKey] : null;
        const isFocused = (
          selShardKey === key || 
          (selSocketGroup && selSocketGroup.userData.shardKey === key)
        );
        const isOpaque = !isFocused;

        candidates.push({
          type: 'shard',
          key,
          object: shardMeshObj,
          distance: hit.distance,
          isOpaque,
          hit
        });
      }
      continue;
    }

    // 4. Routes (cables)
    current = obj;
    let routeObj = null;
    while (current) {
      if (current.userData && current.userData.isRoute) {
        routeObj = current;
        break;
      }
      current = current.parent;
    }
    if (routeObj) {
      candidates.push({
        type: 'route',
        key: routeObj.userData.routeKey,
        object: routeObj,
        distance: hit.distance,
        hit
      });
    }
  }

  // Epsilon threshold to prevent raycast fighting (z-fighting) between a socket and its parent shard
  const epsilon = 0.08 * VIS_SCALE;

  // Step 1: Find the closest visible socket
  let bestSocket = null;
  for (const cand of candidates) {
    if (cand.type === 'socket') {
      let occluded = false;
      for (const prev of candidates) {
        if (prev.distance >= cand.distance) break;
        if (prev.type === 'shard' && prev.isOpaque) {
          if (prev.key !== cand.shardKey) {
            occluded = true;
            break;
          } else {
            // Same parent shard: occlude only if socket is significantly further behind
            // (e.g. socket is on the back or side face, hidden by the shard geometry)
            if (cand.distance - prev.distance > epsilon) {
              occluded = true;
              break;
            }
          }
        }
      }
      if (!occluded) {
        bestSocket = cand;
        break;
      }
    }
  }

  if (bestSocket) {
    return { type: 'socket', key: bestSocket.key, object: bestSocket.object, hit: bestSocket.hit };
  }

  // Step 2: Find the closest visible shard
  for (const cand of candidates) {
    if (cand.type === 'shard') {
      let occluded = false;
      for (const prev of candidates) {
        if (prev.distance >= cand.distance) break;
        if (prev.type === 'shard' && prev.isOpaque && prev.key !== cand.key) {
          occluded = true;
          break;
        }
      }
      if (!occluded) {
        return { type: 'shard', key: cand.key, object: cand.object, hit: cand.hit };
      }
    }
  }

  // Step 3: Find the closest route
  for (const cand of candidates) {
    if (cand.type === 'route') {
      return { type: 'route', key: cand.key, object: cand.object, hit: cand.hit };
    }
  }

  return null;
}
