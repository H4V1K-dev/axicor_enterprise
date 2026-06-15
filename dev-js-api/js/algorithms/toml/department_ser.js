/**
 * @fileoverview Department serializer for TOML engine.
 * Converts frontend Department and internal Shard list to department.toml structure.
 */

/**
 * Serializes a department's shards and connections.
 * @param {string} deptName - e.g. "SensoryInput"
 * @param {Array<Object>} deptShards - Shards belonging to this department
 * @param {Array<Object>} allConnections - All connections in the system
 * @returns {Object}
 */
export function serializeDepartment(deptName, deptShards, allConnections) {
  // 1. Shards list
  const shards = deptShards.map(s => {
    return {
      name: s.shard,
      config: `${s.shard}/${s.shard}.toml`
    };
  });

  // 2. Intra-department connections
  const connections = allConnections
    .filter(conn => {
      // Both from and to must belong to this department
      return conn.from.startsWith(deptName + '.') && conn.to.startsWith(deptName + '.');
    })
    .map(conn => {
      // Strip department prefix for local names
      const fromShard = conn.from.split('.')[1];
      const toShard = conn.to.split('.')[1];
      return {
        from: fromShard,
        to: toShard,
        output_matrix: conn.from_socket,
        width: conn.matrix_w,
        height: conn.matrix_h,
        entry_z: "Mid",
        target_type: "All",
        growth_steps: 800
      };
    });

  return {
    shards,
    connections
  };
}
