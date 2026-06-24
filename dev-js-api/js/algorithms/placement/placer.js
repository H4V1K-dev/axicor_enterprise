/**
 * @fileoverview placer.js — Coordinating function for AxiCAD placement.
 * Imports specialized algorithms from levels, departments, and shards modules.
 */

import { layoutLevelsAndShards } from './levels.js';
import { packShardsLocally } from './shards.js';
import { packDepartmentsOnLevel, computeDepartmentBounds } from './departments.js';

/**
 * Pure 3D layout generator for Nested Shard Composition.
 * Places shards globally in layered stacks (levels) and groups them in departments.
 * 
 * @param {Array<{key: string, dept: string, shard: string, orbit: number, size: {w: number, d: number, h: number}}>} shards
 * @param {Object} overrides - Position/size overrides dictionary
 * @returns {Object} placement JSON containing levels, departments, and shards
 */
export function computePlacement(shards, overrides = {}) {
  const overridesShards = overrides.shards || {};
  const overridesLevels = overrides.levels || [];

  // 1. Resolve level ordering from overrides.levels array
  let levelsList = [];
  if (Array.isArray(overridesLevels)) {
    levelsList = JSON.parse(JSON.stringify(overridesLevels));
  }

  // Find all unique level IDs used by shards
  const activeLevelIds = new Set();
  shards.forEach(s => {
    const shardOverride = overridesShards[s.key] || {};
    const orbit = shardOverride.orbit !== undefined ? shardOverride.orbit : s.orbit;
    activeLevelIds.add(orbit);
  });

  // Ensure all active levels are registered in levelsList
  activeLevelIds.forEach(lvlId => {
    if (!levelsList.some(l => l.id === lvlId)) {
      let defaultName = `Level ${lvlId}`;
      const firstShardOnLvl = shards.find(s => {
        const shardOverride = overridesShards[s.key] || {};
        const orbit = shardOverride.orbit !== undefined ? shardOverride.orbit : s.orbit;
        return orbit === lvlId;
      });
      if (firstShardOnLvl) {
        defaultName = firstShardOnLvl.dept;
      }
      levelsList.push({
        id: lvlId,
        name: defaultName,
        color: levelsList.length === 0 ? "#34d399" : levelsList.length === 1 ? "#38bdf8" : "#f472b6"
      });
    }
  });

  // 2. Perform default packing for shards and departments if no overrides exist
  const gapShards = 0;
  const gapDepts = 1;

  // Group input shards by orbit
  const shardsByLevel = {};
  shards.forEach(s => {
    const shardOverride = overridesShards[s.key] || {};
    const orbit = shardOverride.orbit !== undefined ? shardOverride.orbit : s.orbit;
    if (!shardsByLevel[orbit]) shardsByLevel[orbit] = [];
    shardsByLevel[orbit].push(s);
  });

  const defaultPositions = {};

  levelsList.forEach(lvl => {
    const lvlShards = shardsByLevel[lvl.id] || [];
    const deptBuckets = {};
    lvlShards.forEach(s => {
      if (!deptBuckets[s.dept]) deptBuckets[s.dept] = [];
      deptBuckets[s.dept].push(s);
    });

    // Step A: Pack shards within each department locally
    const { deptPackings, deptRects } = packShardsLocally(deptBuckets, overridesShards, gapShards);

    // Step B: Pack departments within this level
    const deptPositions = packDepartmentsOnLevel(deptRects, gapDepts);

    // Step C: Assign default packed coordinates relative to level origin
    Object.entries(deptBuckets).forEach(([deptName, deptShards]) => {
      const deptPos = deptPositions[deptName] || { u: 0, v: 0 };
      const { positions: shardPos } = deptPackings[deptName] || { positions: {} };

      deptShards.forEach(s => {
        const sPos = shardPos[s.shard] || { u: 0, v: 0 };
        defaultPositions[s.key] = {
          x: deptPos.u + sPos.u,
          y: deptPos.v + sPos.v,
          z: 0 // Will be offset by levels stack z_start during layoutLevelsAndShards
        };
      });
    });
  });

  // 3. Construct final shard list with absolute positions (X, Y)
  const shardsOut = [];
  shards.forEach(s => {
    const key = s.key;
    const shardOverride = overridesShards[key] || {};
    const orbit = shardOverride.orbit !== undefined ? shardOverride.orbit : s.orbit;

    let px = 0, py = 0, pz = 0;
    const defPos = defaultPositions[key] || { x: 0, y: 0, z: 0 };

    if (shardOverride.position) {
      px = Math.round(shardOverride.position.x);
      py = Math.round(shardOverride.position.y);
      pz = Math.round(shardOverride.position.z);
    } else {
      px = defPos.x;
      py = defPos.y;
      pz = defPos.z;
    }

    const w = shardOverride.size?.w !== undefined ? shardOverride.size.w : s.size.w;
    const d = shardOverride.size?.d !== undefined ? shardOverride.size.d : s.size.d;
    const h = shardOverride.size?.h !== undefined ? shardOverride.size.h : s.size.h;

    shardsOut.push({
      key: key,
      dept: s.dept,
      shard: s.shard,
      orbit: orbit,
      position: { x: px, y: py, z: pz },
      size: { w: w, d: d, h: h },
      layers: s.layers || []
    });
  });

  // 4. Perform Z-stacking of levels and shards
  const layoutResult = layoutLevelsAndShards(levelsList, shardsOut);

  // 5. Calculate dynamic department bounding boxes
  const departmentsOut = computeDepartmentBounds(layoutResult.shards);

  return {
    levels: layoutResult.levels,
    departments: departmentsOut,
    shards: layoutResult.shards,
    connections: [],
    seed: overrides.seed || 42,
    simulation: overrides.simulation || {},
    world: overrides.world || {}
  };
}
