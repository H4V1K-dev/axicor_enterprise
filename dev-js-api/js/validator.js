import { store } from './store/store.js';

/**
 * Runs validation checks on visualizer layout data.
 * Checks for:
 * 1. Shard 3D collisions inside the same department.
 * 2. Department 2D collisions on the same level.
 * 3. Shards extending beyond vertical bounds of their level.
 * 
 * @returns {Array<{type: string, severity: string, message: string}>} Array of validation errors
 */
export function runValidation(routes, shardMeshes, socketMeshes, visScale) {
  const errors = [];
  const data = store.get('placementData');
  if (!data || !data.shards) return errors;

  const levels = data.levels || [];
  const depts = data.departments || [];
  const shards = data.shards || [];

  const levelsMap = {};
  levels.forEach(lvl => {
    levelsMap[lvl.id] = lvl;
  });

  // 1. Shard collisions inside the same department (3D AABB check)
  const shardsByDept = {};
  shards.forEach(s => {
    if (!shardsByDept[s.dept]) shardsByDept[s.dept] = [];
    shardsByDept[s.dept].push(s);
  });

  Object.entries(shardsByDept).forEach(([deptName, deptShards]) => {
    for (let i = 0; i < deptShards.length; i++) {
      for (let j = i + 1; j < deptShards.length; j++) {
        const s1 = deptShards[i];
        const s2 = deptShards[j];

        // 3D AABB overlap check
        const overlapX = s1.position.x < (s2.position.x + s2.size.w) && (s1.position.x + s1.size.w) > s2.position.x;
        const overlapY = s1.position.y < (s2.position.y + s2.size.d) && (s1.position.y + s1.size.d) > s2.position.y;
        const overlapZ = s1.position.z < (s2.position.z + s2.size.h) && (s1.position.z + s1.size.h) > s2.position.z;

        if (overlapX && overlapY && overlapZ) {
          errors.push({
            type: 'shard_collision',
            severity: 'error',
            message: `Пересечение шардов ${s1.shard} и ${s2.shard} в департаменте ${deptName}`
          });
        }
      }
    }
  });

  // 2. Department collisions on the same level (2D AABB check on XY floor)
  const deptsByLevel = {};
  depts.forEach(d => {
    if (!deptsByLevel[d.orbit]) deptsByLevel[d.orbit] = [];
    deptsByLevel[d.orbit].push(d);
  });

  Object.entries(deptsByLevel).forEach(([lvlId, lvlDepts]) => {
    const lvl = levelsMap[lvlId];
    const lvlName = lvl ? lvl.name : `Level ${lvlId}`;

    for (let i = 0; i < lvlDepts.length; i++) {
      for (let j = i + 1; j < lvlDepts.length; j++) {
        const d1 = lvlDepts[i];
        const d2 = lvlDepts[j];

        // 2D AABB overlap check on XY plane
        const overlapX = d1.position.x < (d2.position.x + d2.size.w) && (d1.position.x + d1.size.w) > d2.position.x;
        const overlapY = d1.position.y < (d2.position.y + d2.size.d) && (d1.position.y + d1.size.d) > d2.position.y;

        if (overlapX && overlapY) {
          errors.push({
            type: 'dept_collision',
            severity: 'error',
            message: `Пересечение департаментов ${d1.name} и ${d2.name} на уровне ${lvlName}`
          });
        }
      }
    }
  });

  // 3. Level vertical boundary violations
  shards.forEach(s => {
    const lvl = levelsMap[s.orbit];
    if (lvl) {
      const zMin = lvl.z_start;
      const zMax = lvl.z_start + lvl.height;
      const sMinZ = s.position.z;
      const sMaxZ = s.position.z + s.size.h;

      if (sMinZ < zMin || sMaxZ > zMax) {
        errors.push({
          type: 'level_bounds_error',
          severity: 'warning',
          message: `Шард ${s.shard} выходит за вертикальные пределы уровня ${lvl.name} (Z: [${zMin}, ${zMax}])`
        });
      }
    }
  });

  return errors;
}
