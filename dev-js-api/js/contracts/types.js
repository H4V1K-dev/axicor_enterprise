/**
 * @fileoverview JSDoc types for Axicor SDK frontend.
 * These types mirror the Rust config crate structures.
 */

/**
 * @typedef {Object} Vec3
 * @property {number} x
 * @property {number} y
 * @property {number} z
 */

/**
 * @typedef {Object} Vec2
 * @property {number} x
 * @property {number} y
 */

/**
 * @typedef {Object} OrbitData
 * @property {number} index
 * @property {number} radius
 * @property {number} w
 * @property {number} d
 * @property {number} area
 * @property {number} dept_count
 */

/**
 * @typedef {Object} DepartmentData
 * @property {string} name
 * @property {number} orbit
 * @property {number} shard_count
 */

/**
 * @typedef {Object} SocketData
 * @property {string} name
 * @property {number} width
 * @property {number} height
 * @property {number} pitch
 * @property {Vec2} offset
 * @property {number} rotation
 * @property {number|null} faceSign - 1 for Top, -1 for Bottom, null for auto
 */

/**
 * @typedef {Object} PortData
 * @property {string} name
 * @property {number} width
 * @property {number} height
 */

/**
 * @typedef {Object} LayerData
 * @property {string} name
 * @property {number} height_pct
 * @property {number} density
 */

/**
 * @typedef {[string, string]} PopulationData - [layerName, neuronType]
 */

/**
 * @typedef {Object} QuaternionData
 * @property {number} x
 * @property {number} y
 * @property {number} z
 * @property {number} w
 */

/**
 * @typedef {Object} ShardData
 * @property {string} key - Unique key: Department.Shard
 * @property {string} dept - Department name
 * @property {string} shard - Shard name
 * @property {number} orbit
 * @property {number} radius
 * @property {Vec3} position
 * @property {{w: number, d: number, h: number}} size
 * @property {{u: number, v: number}} flat_position
 * @property {QuaternionData} quaternion
 * @property {SocketData[]} sockets
 * @property {PortData[]} input_ports
 * @property {PortData[]} output_ports
 * @property {LayerData[]} layers
 * @property {PopulationData[]} populations
 */

/**
 * @typedef {Object} RouteData
 * @property {string} from - Source shard name or Dept.Shard
 * @property {string} from_socket - Source socket name
 * @property {string} to - Destination shard name or Dept.Shard
 * @property {string} to_socket - Destination socket name
 * @property {Vec3[]} points - Route path waypoints
 * @property {number} [weight]
 * @property {string} [type]
 */

/**
 * @typedef {Object} ValidationIssue
 * @property {string} id
 * @property {'error'|'warning'} type
 * @property {string} message
 * @property {boolean} fixable
 * @property {string} [fixLabel]
 * @property {Object} [fixData]
 */

/**
 * @typedef {Object} SceneState
 * @property {OrbitData[]} orbits
 * @property {DepartmentData[]} departments
 * @property {ShardData[]} shards
 * @property {RouteData[]} connections
 */

// Export an empty object to satisfy module resolution
export {};
