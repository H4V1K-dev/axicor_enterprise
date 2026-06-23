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
  const movedMinX = movedBox.x;
  const movedMaxX = movedBox.x + movedBox.w;
  const movedMinY = movedBox.y;
  const movedMaxY = movedBox.y + movedBox.h;
  const movedMinZ = movedBox.z;
  const movedMaxZ = movedBox.z + movedBox.d;

  for (const other of otherBoxes) {
    const otherMinX = other.x;
    const otherMaxX = other.x + other.w;
    const otherMinY = other.y;
    const otherMaxY = other.y + other.h;
    const otherMinZ = other.z;
    const otherMaxZ = other.z + other.d;

    // Check overlap on X, Y (height) and Z (depth) axes
    const overlapX = movedMinX < otherMaxX && movedMaxX > otherMinX;
    const overlapY = movedMinY < otherMaxY && movedMaxY > otherMinY;
    const overlapZ = movedMinZ < otherMaxZ && movedMaxZ > otherMinZ;

    if (overlapX && overlapY && overlapZ) {
      return true; // Collision detected
    }
  }
  return false;
}
