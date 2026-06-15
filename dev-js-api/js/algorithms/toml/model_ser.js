/**
 * @fileoverview Model serializer for TOML engine.
 * Converts global model data to model.toml structure.
 */

/**
 * Serializes model placement data to the model.toml structure.
 * @param {Object} placementData
 * @returns {Object}
 */
export function serializeModel(placementData) {
  // 1. World parameters
  const world = {
    width_um: (placementData.world && placementData.world.width_um !== undefined) ? parseFloat(placementData.world.width_um) : 25000.0,
    depth_um: (placementData.world && placementData.world.depth_um !== undefined) ? parseFloat(placementData.world.depth_um) : 25000.0,
    height_um: (placementData.world && placementData.world.height_um !== undefined) ? parseFloat(placementData.world.height_um) : 6375.0
  };

  // 2. Simulation parameters
  const simulation = {
    tick_duration_us: (placementData.simulation && placementData.simulation.tick_duration_us !== undefined) ? parseInt(placementData.simulation.tick_duration_us) : 100,
    total_ticks: (placementData.simulation && placementData.simulation.total_ticks !== undefined) ? parseInt(placementData.simulation.total_ticks) : 0,
    master_seed: (placementData.simulation && placementData.simulation.master_seed !== undefined) ? placementData.simulation.master_seed : "AXICOR",
    voxel_size_um: (placementData.simulation && placementData.simulation.voxel_size_um !== undefined) ? parseFloat(placementData.simulation.voxel_size_um) : 25.0,
    signal_speed_m_s: (placementData.simulation && placementData.simulation.signal_speed_m_s !== undefined) ? parseFloat(placementData.simulation.signal_speed_m_s) : 0.5,
    sync_batch_ticks: (placementData.simulation && placementData.simulation.sync_batch_ticks !== undefined) ? parseInt(placementData.simulation.sync_batch_ticks) : 100,
    segment_length_voxels: (placementData.simulation && placementData.simulation.segment_length_voxels !== undefined) ? parseInt(placementData.simulation.segment_length_voxels) : 2,
    axon_growth_max_steps: (placementData.simulation && placementData.simulation.axon_growth_max_steps !== undefined) ? parseInt(placementData.simulation.axon_growth_max_steps) : 250,
    max_dendrites: (placementData.simulation && placementData.simulation.max_dendrites !== undefined) ? parseInt(placementData.simulation.max_dendrites) : 128
  };

  // 3. Departments list
  const departments = (placementData.departments || []).map(dept => {
    return {
      name: dept.name,
      config: `${dept.name}/${dept.name}.toml`
    };
  });

  // 4. Inter-department connections
  const connections = (placementData.connections || [])
    .filter(conn => {
      // Find from and to shard departments
      const fromDept = conn.from.split('.')[0];
      const toDept = conn.to.split('.')[0];
      // Keep only connections between different departments
      return fromDept !== toDept;
    })
    .map(conn => {
      return {
        from: conn.from,
        to: conn.to,
        output_matrix: conn.from_socket,
        width: conn.matrix_w,
        height: conn.matrix_h,
        entry_z: "Top",
        target_type: "All",
        growth_steps: 1000
      };
    });

  return {
    world,
    simulation,
    departments,
    connections
  };
}
