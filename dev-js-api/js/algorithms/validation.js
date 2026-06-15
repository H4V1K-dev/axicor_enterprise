/**
 * @fileoverview Pure topology validation checks for Axicor Visualizer.
 * Zero dependencies on Three.js or DOM.
 */

/**
 * @typedef {import("../contracts/types.js").Vec2} Vec2
 * @property {number} x
 * @property {number} y
 */

/**
 * @typedef {import("../contracts/types.js").ValidationIssue} ValidationIssue
 */

/**
 * @typedef {Object} PlainSocket
 * @property {string} socketKey - Unique SocketKey (ShardKey.SocketName)
 * @property {string} shardKey - Parent shard key
 * @property {string} socketName - Name of socket
 * @property {number} width - Pitch width
 * @property {number} height - Pitch height
 * @property {number} pitch - Spacing pitch
 * @property {Vec2} offset - Local face offsets
 * @property {number} faceSign - 1 for Top, -1 for Bottom
 */

/**
 * @typedef {Object} PlainShard
 * @property {string} key - Shard unique key
 * @property {{w: number, d: number, h: number}} size - Voxel dimensions
 * @property {number} orbit - Layer group number
 */

/**
 * Runs topology static validation on clean layout inputs.
 *
 * @param {Array} routes - Connection routes array
 * @param {Map<string, PlainShard>} shards - Map of ShardKey -> PlainShard details
 * @param {Map<string, PlainSocket>} sockets - Map of SocketKey -> PlainSocket details
 * @returns {ValidationIssue[]} Array of validation alerts and issues
 */
export function validateTopology(routes, shards, sockets) {
  const issues = [];

  if (!routes || !shards || !sockets) {
    return issues;
  }

  // 1. Connection checks
  routes.forEach(route => {
    const sockKeyFrom = `${route.from}.${route.from_socket}`;
    const sockKeyTo = `${route.to}.${route.to_socket}`;
    
    const socketFrom = sockets.get(sockKeyFrom);
    const socketTo = sockets.get(sockKeyTo);

    if (!socketFrom || !socketTo) return;

    const countFrom = socketFrom.width * socketFrom.height;
    const countTo = socketTo.width * socketTo.height;

    // Check count mismatch
    if (countFrom !== countTo) {
      issues.push({
        id: `mismatch_count_${sockKeyFrom}_${sockKeyTo}`,
        type: 'error',
        message: `Не совпадает число пинов в соединении: ${route.from_socket} (${countFrom}) ⇄ ${route.to_socket} (${countTo})`,
        fixable: true,
        affectedSockets: [sockKeyFrom, sockKeyTo],
        fixLabel: `Приравнять к ${socketFrom.width}×${socketFrom.height}`,
        fixData: {
          actionType: 'resize_socket',
          socketKey: sockKeyTo,
          width: socketFrom.width,
          height: socketFrom.height,
          pitch: socketFrom.pitch
        }
      });
    } else if (socketFrom.width !== socketTo.width || socketFrom.height !== socketTo.height) {
      // Check aspect mismatch
      issues.push({
        id: `mismatch_aspect_${sockKeyFrom}_${sockKeyTo}`,
        type: 'warning',
        message: `Разные пропорции матриц: ${route.from_socket} (${socketFrom.width}×${socketFrom.height}) ⇄ ${route.to_socket} (${socketTo.width}×${socketTo.height})`,
        fixable: true,
        affectedSockets: [sockKeyFrom, sockKeyTo],
        fixLabel: `Сделать ${socketFrom.width}×${socketFrom.height}`,
        fixData: {
          actionType: 'resize_socket',
          socketKey: sockKeyTo,
          width: socketFrom.width,
          height: socketFrom.height,
          pitch: socketFrom.pitch
        }
      });
    }

    // Check same face warning
    if (socketFrom.faceSign === socketTo.faceSign) {
      issues.push({
        id: `same_face_${sockKeyFrom}_${sockKeyTo}`,
        type: 'warning',
        message: `Соединенные сокеты на одной стороне: ${route.from_socket} ⇄ ${route.to_socket} (оба ${socketFrom.faceSign === 1 ? 'Top' : 'Bottom'})`,
        fixable: true,
        affectedSockets: [sockKeyFrom, sockKeyTo],
        fixLabel: `Флипнуть ${route.to_socket}`,
        fixData: {
          actionType: 'flip_socket',
          socketKey: sockKeyTo,
          faceSign: -socketTo.faceSign
        }
      });
    }
  });

  // 2. Bounds checks
  for (const [socketKey, socket] of sockets.entries()) {
    const shard = shards.get(socket.shardKey);
    if (!shard) continue;

    const shardW = shard.size.w;
    const shardD = shard.size.d;

    const backingW = socket.width * socket.pitch;
    const backingH = socket.height * socket.pitch;

    const halfShardW = shardW / 2;
    const halfShardD = shardD / 2;

    const outLeft = (socket.offset.x - backingW / 2) < -halfShardW;
    const outRight = (socket.offset.x + backingW / 2) > halfShardW;
    const outBottom = (socket.offset.y - backingH / 2) < -halfShardD;
    const outTop = (socket.offset.y + backingH / 2) > halfShardD;

    if (outLeft || outRight || outBottom || outTop) {
      const limitX = Math.max(0, (shardW - backingW) / 2);
      const limitY = Math.max(0, (shardD - backingH) / 2);
      const newX = Math.max(-limitX, Math.min(limitX, socket.offset.x));
      const newY = Math.max(-limitY, Math.min(limitY, socket.offset.y));

      issues.push({
        id: `out_of_bounds_${socketKey}`,
        type: 'warning',
        message: `Сокет ${socket.socketName} на шарде ${socket.shardKey} выходит за его границы`,
        fixable: true,
        affectedSockets: [socketKey],
        fixLabel: `Прижать внутрь`,
        fixData: {
          actionType: 'move_socket',
          socketKey: socketKey,
          offset: { x: newX, y: newY }
        }
      });
    }
  }

  // 3. Socket Z-collisions & air-gap validation
  const socketArray = Array.from(sockets.values());
  for (let i = 0; i < socketArray.length; i++) {
    const sA = socketArray[i];
    const shardA = shards.get(sA.shardKey);
    if (!shardA) continue;

    for (let j = i + 1; j < socketArray.length; j++) {
      const sB = socketArray[j];
      if (sA.shardKey !== sB.shardKey) continue;

      const halfWA = (sA.width * sA.pitch) / 2;
      const halfHA = (sA.height * sA.pitch) / 2;
      const minXA = sA.offset.x - halfWA;
      const maxXA = sA.offset.x + halfWA;
      const minYA = sA.offset.y - halfHA;
      const maxYA = sA.offset.y + halfHA;

      const halfWB = (sB.width * sB.pitch) / 2;
      const halfHB = (sB.height * sB.pitch) / 2;
      const minXB = sB.offset.x - halfWB;
      const maxXB = sB.offset.x + halfWB;
      const minYB = sB.offset.y - halfHB;
      const maxYB = sB.offset.y + halfHB;

      // Check if X and Y projections overlap (with 0.01 tolerance)
      const overlapX = maxXA > minXB + 0.01 && maxXB > minXA + 0.01;
      const overlapY = maxYA > minYB + 0.01 && maxYB > minYA + 0.01;

      if (overlapX && overlapY) {
        // Compute air-gap requirements based on perimeter: air_gap = (perimeter / 4) / 10
        const perimeterA = 2 * (sA.width + sA.height) * sA.pitch;
        const perimeterB = 2 * (sB.width + sB.height) * sB.pitch;

        const gapA = (perimeterA / 4) / 10;
        const gapB = (perimeterB / 4) / 10;

        const zA = sA.offset.z !== undefined ? sA.offset.z : (sA.faceSign * (shardA.size.h / 2));
        const zB = sB.offset.z !== undefined ? sB.offset.z : (sB.faceSign * (shardA.size.h / 2));

        const zDist = Math.abs(zA - zB);
        const requiredDist = gapA + gapB;

        if (zDist < requiredDist - 0.01) {
          issues.push({
            id: `z_collision_${sA.socketKey}_${sB.socketKey}`,
            type: 'error',
            message: `Коллизия по высоте Z: сокеты ${sA.socketName} и ${sB.socketName} на шарде ${sA.shardKey} перекрываются и не имеют воздушного зазора ${requiredDist.toFixed(1)} vx (фактический: ${zDist.toFixed(1)} vx)`,
            fixable: false,
            affectedSockets: [sA.socketKey, sB.socketKey]
          });
        }
      }
    }
  }

  return issues;
}
