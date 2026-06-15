/**
 * @fileoverview Main entry point for TOML engine package.
 * Exports functions for TOML serialization and scene linting.
 */

import { stringify } from 'smol-toml';
import { serializeModel } from './model_ser.js';
import { serializeDepartment } from './department_ser.js';
import { serializeShard } from './shard_ser.js';
import { lintScene as runLintRules } from './lint_rules.js';

/**
 * Serializes the current visualizer scene state into a map of relative file paths and their TOML content.
 * @param {Object} placementData - The parsed data from placement.json
 * @returns {Map<string, string>} A map from file paths (e.g., "model.toml") to their TOML string content.
 */
export function sceneToToml(placementData) {
  const fileMap = new Map();

  if (!placementData) {
    return fileMap;
  }

  // 1. Serialize and stringify model.toml
  const modelObj = serializeModel(placementData);
  fileMap.set("model.toml", stringify(modelObj));

  // Keep a map of serialized departments and shards for the linter
  const departmentObjects = {};
  const shardObjects = {};

  // Group shards by department
  const shardsByDept = {};
  (placementData.shards || []).forEach(shard => {
    if (!shardsByDept[shard.dept]) {
      shardsByDept[shard.dept] = [];
    }
    shardsByDept[shard.dept].push(shard);
  });

  // 2. Serialize and stringify each department
  (placementData.departments || []).forEach(dept => {
    const deptShards = shardsByDept[dept.name] || [];
    const deptObj = serializeDepartment(dept.name, deptShards, placementData.connections || []);
    
    departmentObjects[dept.name] = deptObj;
    fileMap.set(`${dept.name}/${dept.name}.toml`, stringify(deptObj));

    // 3. Serialize and stringify each shard in the department
    deptShards.forEach(shard => {
      const shardObj = serializeShard(shard);
      shardObjects[shard.key] = shardObj;
      fileMap.set(`${dept.name}/${shard.shard}/${shard.shard}.toml`, stringify(shardObj));
    });
  });

  return {
    fileMap,
    objectsForLint: {
      model: modelObj,
      departments: departmentObjects,
      shards: shardObjects
    }
  };
}

/**
 * Lints the current visualizer scene state.
 * @param {Object} placementData - The parsed data from placement.json
 * @returns {Array<Object>} List of lint issues.
 */
export function lintScene(placementData) {
  if (!placementData) return [];
  const { objectsForLint } = sceneToToml(placementData);
  return runLintRules(objectsForLint);
}

/**
 * Stubbed function for future TOML-to-Scene state import.
 * @param {Map<string, string>} files 
 * @returns {Object}
 */
export function tomlToScene(files) {
  console.warn("tomlToScene is not implemented in this version");
  return null;
}
