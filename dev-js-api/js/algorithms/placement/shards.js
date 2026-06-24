/**
 * @fileoverview shards.js — Pure algorithms for packing shards inside departments.
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
 * Packs shards locally within their respective departments.
 * 
 * @param {Object<string, Array>} deptBuckets - Shards grouped by department name
 * @param {Object} overridesShards - Overrides dictionary for shards
 * @param {number} gapShards - Gap spacing between shards
 * @returns {{deptPackings: Object, deptRects: Array}} Local packings and department bounds
 */
export function packShardsLocally(deptBuckets, overridesShards, gapShards) {
  const deptPackings = {};
  const deptRects = [];

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

  return { deptPackings, deptRects };
}
