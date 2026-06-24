/**
 * @fileoverview levels.js — Pure function to perform Z-axis stacking of levels and shards.
 * Ready to be ported to Rust/Python.
 */

/**
 * Pure function to perform Z-axis stacking of levels and shards.
 * Computes z_start and height for each level and updates absolute position.z of member shards.
 * Preserves local Z offset (height above level floor) for shards.
 * 
 * @param {Array<{id: number, name: string, color: string}>} levelsList - Ordered levels array
 * @param {Array} shards - Shards array
 * @param {Object} oldZStarts - Old z_starts lookup dictionary { levelId: number }
 * @returns {{levels: Array, shards: Array}} Stacked levels and shards
 */
export function layoutLevelsAndShards(levelsList, shards, oldZStarts = {}) {
  const nextLevels = JSON.parse(JSON.stringify(levelsList));
  const nextShards = JSON.parse(JSON.stringify(shards));
  
  let currentZ = 0;
  const gapBetweenLevels = 0;

  nextLevels.forEach((lvl) => {
    lvl.z_start = currentZ;

    // Find shards belonging to this level
    const lvlShards = nextShards.filter(s => s.orbit === lvl.id);

    // Auto-detect old floor if not provided in oldZStarts lookup
    let oldFloor = oldZStarts[lvl.id];
    if (oldFloor === undefined) {
      if (lvlShards.length > 0) {
        oldFloor = Math.min(...lvlShards.map(s => s.position.z));
      } else {
        oldFloor = 0;
      }
    }

    let maxLvlH = 10; // Default height if level is empty
    lvlShards.forEach(s => {
      // Calculate local Z height above level floor using the old floor position
      const localZ = Math.max(0, s.position.z - oldFloor);
      
      const shardTop = localZ + s.size.h;
      if (shardTop > maxLvlH) {
        maxLvlH = shardTop;
      }

      // Translate shard to the new absolute Z position
      s.position.z = lvl.z_start + localZ;
    });

    lvl.height = maxLvlH;
    currentZ = lvl.z_start + lvl.height + gapBetweenLevels;
  });

  return { levels: nextLevels, shards: nextShards };
}
