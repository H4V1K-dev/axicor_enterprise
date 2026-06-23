/**
 * @fileoverview placer.js — Pure placement algorithms for AxiCAD.
 * Decoupled from any runtime, DOM, or file system. Fully ready to port to Rust/Python.
 */

/**
 * 2D Shelf Packing Algorithm for rectangles.
 * Packs rectangles with a given spacing to fit in a roughly square bounding box.
 * 
 * @param {Array<{id: string, w: number, d: number}>} rectangles - Rectangles to pack
 * @param {number} gap - Spacing between rectangles
 * @returns {{widthUsed: number, depthUsed: number, positions: Object<string, {u: number, v: number}>}}
 */
export function packRectangles(rectangles, gap) {
  if (!rectangles || rectangles.length === 0) {
    return { widthUsed: 0, depthUsed: 0, positions: {} };
  }

  // Sort by depth descending (shelf algorithm heuristic)
  const sortedRects = [...rectangles].sort((a, b) => b.d - a.d);

  const totalArea = sortedRects.reduce((sum, r) => sum + (r.w + gap) * (r.d + gap), 0);
  const maxW = Math.max(...sortedRects.map(r => r.w + gap));
  const targetW = Math.max(Math.ceil(Math.sqrt(totalArea)), maxW);

  const shelves = [];
  const positions = {};

  for (const r of sortedRects) {
    const rid = r.id;
    const rw = r.w + gap;
    const rd = r.d + gap;

    let placed = false;
    for (const shelf of shelves) {
      if (shelf.xCursor + rw <= targetW) {
        positions[rid] = { u: shelf.xCursor, v: shelf.yStart };
        shelf.xCursor += rw;
        shelf.height = Math.max(shelf.height, rd);
        placed = true;
        break;
      }
    }

    if (!placed) {
      let yStart = 0;
      if (shelves.length > 0) {
        const prev = shelves[shelves.length - 1];
        yStart = prev.yStart + prev.height;
      }

      const newShelf = {
        yStart: yStart,
        height: rd,
        xCursor: rw
      };
      positions[rid] = { u: 0, v: yStart };
      shelves.push(newShelf);
    }
  }

  const widthUsed = shelves.length > 0 ? Math.max(...shelves.map(s => s.xCursor)) : 0;
  const depthUsed = shelves.reduce((sum, s) => sum + s.height, 0);

  return { widthUsed, depthUsed, positions };
}

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

  // 2. Calculate dynamic heights and z_start for levels in order
  const levelsMap = {};
  let currentZ = 0;
  const gapBetweenLevels = 20;

  levelsList.forEach(lvl => {
    lvl.z_start = currentZ;

    // Find all shards belonging to this level
    const lvlShards = shards.filter(s => {
      const shardOverride = overridesShards[s.key] || {};
      const orbit = shardOverride.orbit !== undefined ? shardOverride.orbit : s.orbit;
      return orbit === lvl.id;
    });

    // Calculate dynamic thickness (height)
    let maxLvlH = 40; // Default height if level is empty
    lvlShards.forEach(s => {
      const shardOverride = overridesShards[s.key] || {};
      const h = shardOverride.size?.h !== undefined ? shardOverride.size.h : s.size.h;

      if (shardOverride.position && shardOverride.position.z !== undefined) {
        const localZ = shardOverride.position.z - lvl.z_start;
        if (localZ + h > maxLvlH) {
          maxLvlH = localZ + h;
        }
      } else {
        if (h > maxLvlH) {
          maxLvlH = h;
        }
      }
    });

    lvl.height = maxLvlH;
    levelsMap[lvl.id] = lvl;
    currentZ = lvl.z_start + lvl.height + gapBetweenLevels;
  });

  // 3. Perform default packing for shards and departments if no overrides exist
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

    const deptRects = [];
    const deptPackings = {};

    // Step A: Pack shards within each department locally
    Object.entries(deptBuckets).forEach(([deptName, deptShards]) => {
      const rects = deptShards.map(s => {
        const shardOverride = overridesShards[s.key] || {};
        const w = shardOverride.size?.w !== undefined ? shardOverride.size.w : s.size.w;
        const d = shardOverride.size?.d !== undefined ? shardOverride.size.d : s.size.d;
        return { id: s.shard, w, d };
      });

      const { widthUsed, depthUsed, positions } = packRectangles(rects, gapShards);
      deptPackings[deptName] = { w: widthUsed, d: depthUsed, positions };
      deptRects.push({ id: deptName, w: widthUsed, d: depthUsed });
    });

    // Step B: Pack departments within this level
    const { positions: deptPositions } = packRectangles(deptRects, gapDepts);

    // Step C: Assign default packed coordinates relative to level origin
    Object.entries(deptBuckets).forEach(([deptName, deptShards]) => {
      const deptPos = deptPositions[deptName] || { u: 0, v: 0 };
      const { positions: shardPos } = deptPackings[deptName];

      deptShards.forEach(s => {
        const sPos = shardPos[s.shard] || { u: 0, v: 0 };
        defaultPositions[s.key] = {
          x: deptPos.u + sPos.u,
          y: deptPos.v + sPos.v,
          z: levelsMap[lvl.id].z_start
        };
      });
    });
  });

  // 4. Construct final shard list with absolute positions
  const shardsOut = [];
  shards.forEach(s => {
    const key = s.key;
    const shardOverride = overridesShards[key] || {};
    const orbit = shardOverride.orbit !== undefined ? shardOverride.orbit : s.orbit;
    const level = levelsMap[orbit];

    let px = 0, py = 0, pz = 0;
    const defPos = defaultPositions[key] || { x: 0, y: 0, z: level ? level.z_start : 0 };

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

  // 5. Dynamic department bounding boxes
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

  return {
    levels: levelsList,
    departments: departmentsOut,
    shards: shardsOut,
    connections: [],
    seed: overrides.seed || 42,
    simulation: overrides.simulation || {},
    world: overrides.world || {}
  };
}

