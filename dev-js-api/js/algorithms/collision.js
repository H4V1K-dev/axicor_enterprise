/**
 * @fileoverview Pure 2D AABB collision detection algorithm.
 * Zero dependencies, works with plain objects.
 */

/**
 * @typedef {Object} Box2D
 * @property {number} x - Center X coordinate
 * @property {number} z - Center Z coordinate (or Y in 2D space)
 * @property {number} w - Width
 * @property {number} d - Depth
 * @property {number} orbit - The vertical layer/orbit level
 */

/**
 * Checks if a moved box overlaps with any other boxes on the same orbit.
 *
 * @param {Box2D} movedBox - Bounding details of the moved box
 * @param {Box2D[]} otherBoxes - Array of other boxes to check collision against
 * @returns {boolean} True if there is a collision/overlap
 */
export function checkShardCollision(movedBox, otherBoxes) {
  const halfW = movedBox.w / 2;
  const halfD = movedBox.d / 2;

  const movedMinX = movedBox.x - halfW;
  const movedMaxX = movedBox.x + halfW;
  const movedMinZ = movedBox.z - halfD;
  const movedMaxZ = movedBox.z + halfD;

  const checkY = (movedBox.y !== undefined && movedBox.h !== undefined);
  let movedMinY, movedMaxY;
  if (checkY) {
    const halfH = movedBox.h / 2;
    movedMinY = movedBox.y - halfH;
    movedMaxY = movedBox.y + halfH;
  }

  for (const other of otherBoxes) {
    // Only collide with boxes on the exact same orbit/layer
    if (other.orbit !== movedBox.orbit) continue;

    const otherHalfW = other.w / 2;
    const otherHalfD = other.d / 2;

    const otherMinX = other.x - otherHalfW;
    const otherMaxX = other.x + otherHalfW;
    const otherMinZ = other.z - otherHalfD;
    const otherMaxZ = other.z + otherHalfD;

    // Check overlap on X and Z axes
    const overlapX = movedMinX < otherMaxX && movedMaxX > otherMinX;
    const overlapZ = movedMinZ < otherMaxZ && movedMaxZ > otherMinZ;

    if (overlapX && overlapZ) {
      if (checkY && other.y !== undefined && other.h !== undefined) {
        const otherHalfH = other.h / 2;
        const otherMinY = other.y - otherHalfH;
        const otherMaxY = other.y + otherHalfH;
        const overlapY = movedMinY < otherMaxY && movedMaxY > otherMinY;
        if (!overlapY) {
          continue; // No overlap on Y, check next box
        }
      }
      return true; // Collision detected
    }
  }
  return false;
}
