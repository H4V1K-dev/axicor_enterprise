/**
 * @fileoverview departments.js — Pure algorithms for department placement and AABB computations.
 */

import { packRectangles } from './shards.js';

/**
 * Packs departments relative to each other on a level.
 * 
 * @param {Array} deptRects - Department rectangles to pack
 * @param {number} gapDepts - Gap spacing between departments
 * @returns {Object<string, {u: number, v: number}>} Relative positions of departments
 */
export function packDepartmentsOnLevel(deptRects, gapDepts) {
  const { positions } = packRectangles(deptRects, gapDepts);
  return positions;
}

/**
 * Dynamically computes AABB boundaries for departments based on member shards' coordinates.
 * 
 * @param {Array} shardsOut - Placed shards list with absolute coordinates
 * @returns {Array} List of resolved departments with position and size
 */
export function computeDepartmentBounds(shardsOut) {
  const departmentsOut = [];
  const resolvedDepts = {};

  shardsOut.forEach(s => {
    const lvlId = s.orbit;
    if (!resolvedDepts[s.dept]) {
      resolvedDepts[s.dept] = {
        name: s.dept,
        orbit: lvlId,
        x_min: s.position.x,
        x_max: s.position.x + s.size.w,
        y_min: s.position.y,
        y_max: s.position.y + s.size.d
      };
    } else {
      const dObj = resolvedDepts[s.dept];
      dObj.x_min = Math.min(dObj.x_min, s.position.x);
      dObj.x_max = Math.max(dObj.x_max, s.position.x + s.size.w);
      dObj.y_min = Math.min(dObj.y_min, s.position.y);
      dObj.y_max = Math.max(dObj.y_max, s.position.y + s.size.d);
    }
  });

  Object.values(resolvedDepts).forEach(d => {
    departmentsOut.push({
      name: d.name,
      orbit: d.orbit,
      position: { x: d.x_min, y: d.y_min },
      size: { w: d.x_max - d.x_min, d: d.y_max - d.y_min }
    });
  });

  return departmentsOut;
}
