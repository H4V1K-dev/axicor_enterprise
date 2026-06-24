/**
 * Converts placement data from Rust Z-up (depth=Y, height=Z) to Three.js Y-up (height=Y, depth=Z) coordinates.
 * @param {any} placementData 
 * @returns {any}
 */
export function toThreeCoords(placementData) {
  if (!placementData) return null;
  const cloned = JSON.parse(JSON.stringify(placementData));

  if (cloned.shards) {
    cloned.shards.forEach(shard => {
      if (shard.position && shard.size) {
        const rx = shard.position.x;
        const ry = shard.position.y; // depth
        const rz = shard.position.z; // height

        shard.position.x = rx;
        shard.position.y = rz; // Three.js Y is height
        shard.position.z = ry; // Three.js Z is depth

        const rw = shard.size.w;
        const rd = shard.size.d; // depth
        const rh = shard.size.h; // height

        shard.size.w = rw;
        shard.size.h = rh; // Three.js H is height
        shard.size.d = rd; // Three.js D is depth
      }
    });
  }

  if (cloned.departments) {
    cloned.departments.forEach(dept => {
      if (dept.position) {
        // Rust Y (depth) is translated to Three.js Z (depth)
        dept.position.z = dept.position.y;
        delete dept.position.y;
      }
    });
  }

  return cloned;
}

/**
 * Converts placement data from Three.js Y-up (height=Y, depth=Z) to Rust Z-up (depth=Y, height=Z) coordinates.
 * @param {any} placementData 
 * @returns {any}
 */
export function toRustCoords(placementData) {
  if (!placementData) return null;
  const cloned = JSON.parse(JSON.stringify(placementData));

  if (cloned.shards) {
    cloned.shards.forEach(shard => {
      if (shard.position && shard.size) {
        const tx = shard.position.x;
        const ty = shard.position.y; // height
        const tz = shard.position.z; // depth

        shard.position.x = tx;
        shard.position.y = tz; // Rust Y is depth (Three Z)
        shard.position.z = ty; // Rust Z is height (Three Y)

        const tw = shard.size.w;
        const th = shard.size.h; // height
        const td = shard.size.d; // depth

        shard.size.w = tw;
        shard.size.d = td; // Rust D is depth (Three D)
        shard.size.h = th; // Rust H is height (Three H)
      }
    });
  }

  if (cloned.departments) {
    cloned.departments.forEach(dept => {
      if (dept.position) {
        // Three.js Z (depth) is translated to Rust Y (depth)
        dept.position.y = dept.position.z;
        delete dept.position.z;
      }
    });
  }

  return cloned;
}

/**
 * Converts a history object state to Three.js coords.
 */
function mapStateToThree(state) {
  if (!state) return null;
  if (state.position && state.size) {
    const rx = state.position.x;
    const ry = state.position.y;
    const rz = state.position.z;
    state.position.x = rx;
    state.position.y = rz;
    state.position.z = ry;

    const rw = state.size.w;
    const rd = state.size.d;
    const rh = state.size.h;
    state.size.w = rw;
    state.size.h = rh;
    state.size.d = rd;
  }
  if (state.shard) {
    state.shard = mapStateToThree(state.shard);
  }
  return state;
}

/**
 * Converts a history object state to Rust coords.
 */
function mapStateToRust(state) {
  if (!state) return null;
  if (state.position && state.size) {
    const tx = state.position.x;
    const ty = state.position.y;
    const tz = state.position.z;
    state.position.x = tx;
    state.position.y = tz;
    state.position.z = ty;

    const tw = state.size.w;
    const th = state.size.h;
    const td = state.size.d;
    state.size.w = tw;
    state.size.d = td;
    state.size.h = th;
  }
  if (state.shard) {
    state.shard = mapStateToRust(state.shard);
  }
  return state;
}

/**
 * Translates history cache stacks to Three.js coordinates.
 */
export function historyToThree(historyData) {
  if (!historyData) return null;
  const cloned = JSON.parse(JSON.stringify(historyData));

  const mapAction = (act) => {
    if (act.undoState) act.undoState = mapStateToThree(act.undoState);
    if (act.redoState) act.redoState = mapStateToThree(act.redoState);
    return act;
  };

  if (cloned.globalStack) {
    cloned.globalStack = cloned.globalStack.map(mapAction);
  }
  if (cloned.objectHistory) {
    for (const key of Object.keys(cloned.objectHistory)) {
      cloned.objectHistory[key] = cloned.objectHistory[key].map(mapAction);
    }
  }
  return cloned;
}

/**
 * Translates history cache stacks to Rust coordinates.
 */
export function historyToRust(historyData) {
  if (!historyData) return null;
  const cloned = JSON.parse(JSON.stringify(historyData));

  const mapAction = (act) => {
    if (act.undoState) act.undoState = mapStateToRust(act.undoState);
    if (act.redoState) act.redoState = mapStateToRust(act.redoState);
    return act;
  };

  if (cloned.globalStack) {
    cloned.globalStack = cloned.globalStack.map(mapAction);
  }
  if (cloned.objectHistory) {
    for (const key of Object.keys(cloned.objectHistory)) {
      cloned.objectHistory[key] = cloned.objectHistory[key].map(mapAction);
    }
  }
  return cloned;
}
