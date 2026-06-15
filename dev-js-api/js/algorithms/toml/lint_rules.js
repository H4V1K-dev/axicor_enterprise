/**
 * @fileoverview Lint rules checker for TOML engine.
 * Validates the serialized TOML structures against SDK linter specifications.
 */

/**
 * Runs lint checks on the serialized TOML structures.
 * @param {Object} serializedData - Object containing:
 *   - model: Object representation of model.toml
 *   - departments: Map of deptName -> department.toml object
 *   - shards: Map of shardKey -> shard.toml object
 * @returns {Array<Object>} List of lint issues, each with:
 *   - file: string (e.g. "model.toml")
 *   - path: string (JSON path or section name)
 *   - message: string (issue description)
 *   - severity: "error" | "warning"
 */
export function lintScene(serializedData) {
  const issues = [];
  const { model, departments, shards } = serializedData;

  // ─── 1. model.toml validation ──────────────────────────────────────────
  if (model) {
    // 1.1 Duplicates in departments
    const deptNames = (model.departments || []).map(d => d.name);
    const uniqueDepts = new Set(deptNames);
    if (uniqueDepts.size !== deptNames.length) {
      issues.push({
        file: "model.toml",
        path: "departments",
        message: "Duplicate department names are declared in the model",
        severity: "error"
      });
    }

    // 1.2 Connections checking
    (model.connections || []).forEach((conn, idx) => {
      const fromParts = conn.from.split('.');
      const toParts = conn.to.split('.');

      if (fromParts.length !== 2) {
        issues.push({
          file: "model.toml",
          path: `connections[${idx}].from`,
          message: `Connection source "${conn.from}" must be in "Department.Shard" dot notation`,
          severity: "error"
        });
      }
      if (toParts.length !== 2) {
        issues.push({
          file: "model.toml",
          path: `connections[${idx}].to`,
          message: `Connection target "${conn.to}" must be in "Department.Shard" dot notation`,
          severity: "error"
        });
      }

      // Resolve Shards
      const sourceShard = shards[conn.from];
      const targetShard = shards[conn.to];

      if (!sourceShard) {
        issues.push({
          file: "model.toml",
          path: `connections[${idx}].from`,
          message: `Connection source shard "${conn.from}" could not be resolved`,
          severity: "error"
        });
      } else {
        // Output matrix check on source
        const outputExists = (sourceShard.outputs || []).some(out => out.name === conn.output_matrix);
        if (!outputExists) {
          issues.push({
            file: "model.toml",
            path: `connections[${idx}].output_matrix`,
            message: `Output matrix "${conn.output_matrix}" does not exist in source shard "${conn.from}"`,
            severity: "error"
          });
        }
      }

      if (!targetShard) {
        issues.push({
          file: "model.toml",
          path: `connections[${idx}].to`,
          message: `Connection target shard "${conn.to}" could not be resolved`,
          severity: "error"
        });
      }
    });

    // 1.3 Segment length (v_seg) is integer
    const sim = model.simulation || {};
    if (sim.segment_length_voxels !== undefined && !Number.isInteger(sim.segment_length_voxels)) {
      issues.push({
        file: "model.toml",
        path: "simulation.segment_length_voxels",
        message: "simulation.segment_length_voxels must be an integer (v_seg invariant)",
        severity: "error"
      });
    }
  }

  // ─── 2. department.toml validation ──────────────────────────────────────
  if (departments) {
    for (const [deptName, deptObj] of Object.entries(departments)) {
      const fileName = `${deptName}/${deptName}.toml`;

      // 2.1 Shard duplicates
      const shardNames = (deptObj.shards || []).map(s => s.name);
      const uniqueShards = new Set(shardNames);
      if (uniqueShards.size !== shardNames.length) {
        issues.push({
          file: fileName,
          path: "shards",
          message: "Duplicate shard names are declared in the department",
          severity: "error"
        });
      }

      // 2.2 Connections
      (deptObj.connections || []).forEach((conn, idx) => {
        const sourceKey = `${deptName}.${conn.from}`;
        const targetKey = `${deptName}.${conn.to}`;

        const sourceShard = shards[sourceKey];
        const targetShard = shards[targetKey];

        if (!shardNames.includes(conn.from)) {
          issues.push({
            file: fileName,
            path: `connections[${idx}].from`,
            message: `Connection source shard "${conn.from}" is not declared in department shards`,
            severity: "error"
          });
        }
        if (!shardNames.includes(conn.to)) {
          issues.push({
            file: fileName,
            path: `connections[${idx}].to`,
            message: `Connection target shard "${conn.to}" is not declared in department shards`,
            severity: "error"
          });
        }

        if (sourceShard) {
          const outputExists = (sourceShard.outputs || []).some(out => out.name === conn.output_matrix);
          if (!outputExists) {
            issues.push({
              file: fileName,
              path: `connections[${idx}].output_matrix`,
              message: `Output matrix "${conn.output_matrix}" does not exist in source shard "${sourceKey}"`,
              severity: "error"
            });
          }
        }
      });
    }
  }

  // ─── 3. shard.toml validation ───────────────────────────────────────────
  if (shards) {
    for (const [shardKey, shard] of Object.entries(shards)) {
      const parts = shardKey.split('.');
      const fileName = `${parts[0]}/${parts[1]}/${parts[1]}.toml`;

      // 3.1 Dimensions w, d <= 1023, h <= 255
      const dims = shard.dimensions || {};
      if (dims.w > 1023 || dims.w < 0) {
        issues.push({
          file: fileName,
          path: "dimensions.w",
          message: `Width ${dims.w} must be in range 0..1023 (10-bit PackedPosition limit)`,
          severity: "error"
        });
      }
      if (dims.d > 1023 || dims.d < 0) {
        issues.push({
          file: fileName,
          path: "dimensions.d",
          message: `Depth ${dims.d} must be in range 0..1023 (10-bit PackedPosition limit)`,
          severity: "error"
        });
      }
      if (dims.h > 255 || dims.h < 0) {
        issues.push({
          file: fileName,
          path: "dimensions.h",
          message: `Height ${dims.h} must be in range 0..255 (8-bit PackedPosition limit)`,
          severity: "error"
        });
      }

      // 3.2 Layer sum height_pct == 1.0 (allow small rounding tolerance, e.g. 1e-4)
      const layersList = shard.layers || [];
      if (layersList.length > 0) {
        const sumHeight = layersList.reduce((sum, l) => sum + (l.height_pct || 0), 0.0);
        if (Math.abs(sumHeight - 1.0) > 1e-4) {
          issues.push({
            file: fileName,
            path: "layers",
            message: `Sum of cortical layer height percentages is ${sumHeight.toFixed(4)}, must be exactly 1.0`,
            severity: "error"
          });
        }
      }

      // 3.3 Composition check and share sum == 1.0 per layer
      const typeNames = new Set((shard.neuron_types || []).map(nt => nt.name));

      layersList.forEach((layer, lIdx) => {
        const comp = layer.composition || [];
        if (comp.length > 0) {
          const sumShare = comp.reduce((sum, c) => sum + (c.share || 0), 0.0);
          if (Math.abs(sumShare - 1.0) > 1e-4) {
            issues.push({
              file: fileName,
              path: `layers[${lIdx}].composition`,
              message: `Sum of composition shares in layer "${layer.name}" is ${sumShare.toFixed(4)}, must be exactly 1.0`,
              severity: "error"
            });
          }

          comp.forEach((c, cIdx) => {
            if (!typeNames.has(c.type_name)) {
              issues.push({
                file: fileName,
                path: `layers[${lIdx}].composition[${cIdx}].type_name`,
                message: `Composition type "${c.type_name}" does not exist in declared neuron types`,
                severity: "error"
              });
            }
          });
        }
      });

      // 3.4 Maximum 16 neuron types
      const neuronTypes = shard.neuron_types || [];
      if (neuronTypes.length > 16) {
        issues.push({
          file: fileName,
          path: "neuron_types",
          message: `Declared ${neuronTypes.length} neuron types, which exceeds the maximum of 16 (4-bit LUT limit)`,
          severity: "error"
        });
      }

      neuronTypes.forEach((nt, idx) => {
        // 3.5 Sprouting weights sum == 1.0 (growth parameters)
        const gr = (nt.growth || {});
        const sumSprout = (gr.sprouting_weight_distance || 0) + 
                         (gr.sprouting_weight_power || 0) + 
                         (gr.sprouting_weight_explore || 0) + 
                         (gr.sprouting_weight_type || 0);
        if (sumSprout > 0 && Math.abs(sumSprout - 1.0) > 1e-4) {
          issues.push({
            file: fileName,
            path: `neuron_types[${idx}].growth`,
            message: `Sum of sprouting weights for "${nt.name}" is ${sumSprout.toFixed(4)}, must be exactly 1.0`,
            severity: "error"
          });
        }

        // 3.6 Signal propagation length >= refractory period
        const propLen = (nt.signal || {}).signal_propagation_length || 0;
        const refr = (nt.timings || {}).refractory_period || 0;
        if (propLen < refr) {
          issues.push({
            file: fileName,
            path: `neuron_types[${idx}].signal`,
            message: `signal_propagation_length (${propLen}) must be >= refractory_period (${refr}) for type "${nt.name}"`,
            severity: "error"
          });
        }

        // 3.7 Inertia curve has exactly 8 elements
        const gs = (nt.gsop || {});
        if (gs.inertia_curve && gs.inertia_curve.length !== 8) {
          issues.push({
            file: fileName,
            path: `neuron_types[${idx}].gsop.inertia_curve`,
            message: `Inertia curve for "${nt.name}" must have exactly 8 elements (found ${gs.inertia_curve.length})`,
            severity: "error"
          });
        }
      });

      // 3.8 Ghost capacity check
      const settings = shard.settings || {};
      // Find connections targeting this shard
      const isTargetOfConnections = (model?.connections || []).some(c => c.to === shardKey) || 
                                    (departments?.[parts[0]]?.connections || []).some(c => `${parts[0]}.${c.to}` === shardKey);
      if (isTargetOfConnections && (!settings.ghost_capacity || settings.ghost_capacity <= 0)) {
        issues.push({
          file: fileName,
          path: "settings.ghost_capacity",
          message: `Shard is a target of connection(s), ghost_capacity must be > 0`,
          severity: "error"
        });
      }

      // 3.9 Projection bounds for inputs and outputs pins
      (shard.inputs || []).forEach((inp, inpIdx) => {
        (inp.pins || []).forEach((pin, pinIdx) => {
          if ((pin.local_u || 0) + (pin.u_width || 0) > 1.0001) {
            issues.push({
              file: fileName,
              path: `inputs[${inpIdx}].pins[${pinIdx}].u_bounds`,
              message: `Projection bounds horizontal overflow for input pin "${pin.name}": local_u (${pin.local_u}) + u_width (${pin.u_width}) exceeds 1.0`,
              severity: "error"
            });
          }
          if ((pin.local_v || 0) + (pin.v_height || 0) > 1.0001) {
            issues.push({
              file: fileName,
              path: `inputs[${inpIdx}].pins[${pinIdx}].v_bounds`,
              message: `Projection bounds vertical overflow for input pin "${pin.name}": local_v (${pin.local_v}) + v_height (${pin.v_height}) exceeds 1.0`,
              severity: "error"
            });
          }
        });
      });
      (shard.outputs || []).forEach((out, outIdx) => {
        (out.pins || []).forEach((pin, pinIdx) => {
          if ((pin.local_u || 0) + (pin.u_width || 0) > 1.0001) {
            issues.push({
              file: fileName,
              path: `outputs[${outIdx}].pins[${pinIdx}].u_bounds`,
              message: `Projection bounds horizontal overflow for output pin "${pin.name}": local_u (${pin.local_u}) + u_width (${pin.u_width}) exceeds 1.0`,
              severity: "error"
            });
          }
          if ((pin.local_v || 0) + (pin.v_height || 0) > 1.0001) {
            issues.push({
              file: fileName,
              path: `outputs[${outIdx}].pins[${pinIdx}].v_bounds`,
              message: `Projection bounds vertical overflow for output pin "${pin.name}": local_v (${pin.local_v}) + v_height (${pin.v_height}) exceeds 1.0`,
              severity: "error"
            });
          }
        });
      });
    }
  }

  // ─── 4. Cross-file / System level validation ─────────────────────────────
  if (model && departments && shards) {
    // 4.1 sum(ghost_capacity) validation
    // For each shard, check that its ghost_capacity >= sum(width * height) of all incoming connections
    for (const [shardKey, shard] of Object.entries(shards)) {
      const parts = shardKey.split('.');
      const fileName = `${parts[0]}/${parts[1]}/${parts[1]}.toml`;
      const settings = shard.settings || {};

      let totalIncomingMatrixArea = 0;

      // Check model.toml connections targeting this shard
      (model.connections || []).forEach(c => {
        if (c.to === shardKey) {
          totalIncomingMatrixArea += (c.width || 0) * (c.height || 0);
        }
      });

      // Check department.toml connections targeting this shard
      const deptObj = departments[parts[0]];
      if (deptObj) {
        (deptObj.connections || []).forEach(c => {
          if (`${parts[0]}.${c.to}` === shardKey) {
            totalIncomingMatrixArea += (c.width || 0) * (c.height || 0);
          }
        });
      }

      if (totalIncomingMatrixArea > 0) {
        const capacity = settings.ghost_capacity || 0;
        if (capacity < totalIncomingMatrixArea) {
          issues.push({
            file: fileName,
            path: "settings.ghost_capacity",
            message: `ghost_capacity (${capacity}) is less than total incoming connections matrix cells (${totalIncomingMatrixArea})`,
            severity: "warning"
          });
        }
      }
    }
  }

  return issues;
}
