import * as THREE from 'three';
import { THEME, RENDER_BINS } from './theme.js';
import { recomputeSpatialLayoutFromMeshes, levelAABBs, deptAABBs } from '../algorithms/placement/spatial_manager.js';

/**
 * Creates the wireframe mesh representing a level boundary.
 * @param {any} lvl 
 * @param {any} box 
 * @param {THREE.BufferGeometry} unitEdgeGeo 
 * @returns {THREE.LineSegments}
 */
export function createLevelWire(lvl, box, unitEdgeGeo) {
  const lvlColor = new THREE.Color(lvl.color || "#ffffff");
  const mat = new THREE.LineBasicMaterial({
    color: lvlColor,
    transparent: true,
    opacity: 0.18,
  });
  const wire = new THREE.LineSegments(unitEdgeGeo, mat);
  wire.position.set(box.x, box.y, box.z);
  wire.scale.set(box.w, box.h, box.d);
  wire.raycast = () => {};
  wire.renderOrder = RENDER_BINS.wireframes;
  wire.userData = { levelId: lvl.id };
  return wire;
}

/**
 * Creates the wireframe mesh representing a department boundary.
 * @param {any} dept 
 * @param {any} box 
 * @param {THREE.BufferGeometry} unitEdgeGeo 
 * @param {number} visScale 
 * @returns {THREE.LineSegments}
 */
export function createDeptWire(dept, box, unitEdgeGeo, visScale) {
  const deptGeo = unitEdgeGeo.clone();
  const mat = new THREE.LineDashedMaterial({
    color: 0x8b949e,
    dashSize: 0.8 * visScale,
    gapSize: 0.4 * visScale,
    transparent: true,
    opacity: 0.25
  });
  const wire = new THREE.LineSegments(deptGeo, mat);
  wire.position.set(box.x, box.y, box.z);
  wire.scale.set(box.w, box.h, box.d);
  wire.computeLineDistances();
  wire.raycast = () => {};
  wire.renderOrder = RENDER_BINS.wireframes;
  wire.userData = { orbit: dept.orbit, name: dept.name };
  return wire;
}

/**
 * Updates 3D meshes visibility based on hiddenLevelIds, focusedLevelId, etc.
 * @param {THREE.Group} levelsGroup 
 * @param {THREE.Group} deptsGroup 
 * @param {Set<number>} hiddenLevelIds 
 * @param {number|null} focusedLevelId 
 * @param {string|null} selectedDeptName 
 * @param {string|null} selectedShardKey 
 * @param {any} placementData 
 */
export function updateLevelsVisibility(
  levelsGroup,
  deptsGroup,
  hiddenLevelIds,
  focusedLevelId,
  selectedDeptName,
  selectedShardKey,
  placementData
) {
  if (!placementData) return;

  // Determine active level ID from selected shard/dept if level is not explicitly focused
  let activeLvlId = focusedLevelId;
  if (activeLvlId === null && selectedShardKey) {
    const shard = placementData.shards.find(s => s.key === selectedShardKey);
    if (shard) activeLvlId = shard.orbit;
  }
  if (activeLvlId === null && selectedDeptName) {
    const dept = placementData.departments.find(d => d.name === selectedDeptName);
    if (dept) activeLvlId = dept.orbit;
  }
  
  // Determine active department name from selected shard if not explicitly selected
  let activeDeptName = selectedDeptName;
  if (activeDeptName === null && selectedShardKey) {
    const shard = placementData.shards.find(s => s.key === selectedShardKey);
    if (shard) activeDeptName = shard.dept;
  }

  const isLevelFocused = activeLvlId !== null;
  const isDeptFocused = activeDeptName !== null;
  const isShardFocused = selectedShardKey !== null;

  // 1. Level wireframe visibility & opacity
  if (levelsGroup) {
    levelsGroup.children.forEach(lvlMesh => {
      const lvlId = lvlMesh.userData?.levelId;
      if (lvlId !== undefined) {
        const isHidden = hiddenLevelIds.has(lvlId);
        if (isHidden) {
          lvlMesh.visible = false;
          return;
        }

        lvlMesh.visible = true;

        if (!lvlMesh.userData.originalColor) {
          lvlMesh.userData.originalColor = lvlMesh.material.color.clone();
        }

        const isCurrentLevel = (Number(lvlId) === Number(activeLvlId));

        if (isCurrentLevel) {
          lvlMesh.material.color.copy(lvlMesh.userData.originalColor);
          lvlMesh.material.opacity = THEME.levelWireframe.activeOpacity; // 0.85
        } else {
          lvlMesh.material.color.setHex(0x555555); // серый
          if (isShardFocused) {
            lvlMesh.material.opacity = 0.05; // 5%
          } else if (isDeptFocused) {
            lvlMesh.material.opacity = 0.2; // 20%
          } else if (isLevelFocused) {
            lvlMesh.material.opacity = 0.5; // 50%
          } else {
            lvlMesh.material.color.copy(lvlMesh.userData.originalColor);
            lvlMesh.material.opacity = THEME.levelWireframe.defaultOpacity;
          }
        }
        lvlMesh.material.transparent = true;
        lvlMesh.material.needsUpdate = true;
      }
    });
  }

  // 2. Department boundary visibility & opacity
  if (deptsGroup) {
    deptsGroup.children.forEach(deptMesh => {
      const lvlId = deptMesh.userData?.orbit;
      const deptName = deptMesh.userData?.name;
      if (lvlId !== undefined) {
        const isHidden = hiddenLevelIds.has(lvlId);
        if (isHidden) {
          deptMesh.visible = false;
          return;
        }

        deptMesh.visible = true;

        if (!deptMesh.userData.originalColor) {
          deptMesh.userData.originalColor = deptMesh.material.color.clone();
        }

        const isCurrentLevel = (Number(lvlId) === Number(activeLvlId));
        const isCurrentDept = (deptName === activeDeptName);

        if (isCurrentDept) {
          deptMesh.material.color.copy(deptMesh.userData.originalColor);
          deptMesh.material.opacity = THEME.deptWireframe.selectedOpacity; // 0.9
        } else if (isCurrentLevel) {
          // Other depts on active level
          deptMesh.material.color.setHex(0x555555); // серый
          if (isShardFocused) {
            deptMesh.material.opacity = 0.2; // 20%
          } else if (isDeptFocused) {
            deptMesh.material.opacity = 0.5; // 50%
          } else {
            // Level focused but no active dept
            deptMesh.material.color.copy(deptMesh.userData.originalColor);
            deptMesh.material.opacity = THEME.deptWireframe.activeOpacity; // 0.7
          }
        } else {
          // Depts on inactive levels
          deptMesh.material.color.setHex(0x555555); // серый
          if (isShardFocused) {
            deptMesh.material.opacity = 0.05; // 5%
          } else if (isDeptFocused) {
            deptMesh.material.opacity = 0.2; // 20%
          } else if (isLevelFocused) {
            deptMesh.material.opacity = 0.5; // 50%
          } else {
            // No focus at all
            deptMesh.material.color.copy(deptMesh.userData.originalColor);
            deptMesh.material.opacity = THEME.deptWireframe.defaultOpacity;
          }
        }
        deptMesh.material.transparent = true;
        deptMesh.material.needsUpdate = true;
      }
    });
  }
}

/**
 * Re-computes and updates the visual wireframe boxes of Levels and Departments.
 * @param {THREE.Group} levelsGroup 
 * @param {THREE.Group} deptsGroup 
 * @param {Map} levelsMeshes 
 * @param {Map} deptsMeshes 
 * @param {any} placementData 
 * @param {Map} shardMeshes 
 * @param {Map} shardDataMap 
 * @param {number} visScale 
 */
export function updateContainerWires(
  levelsGroup,
  deptsGroup,
  levelsMeshes,
  deptsMeshes,
  placementData,
  shardMeshes,
  shardDataMap,
  visScale
) {
  if (!levelsGroup || !deptsGroup || !placementData) return;

  const levels = placementData.levels || [];
  const depts = placementData.departments || [];

  // Compute boundaries dynamically based on current mesh positions
  recomputeSpatialLayoutFromMeshes(shardMeshes, shardDataMap, levels, depts, visScale);

  // Adjust level wireframe scales
  levels.forEach(lvl => {
    const box = levelAABBs.get(lvl.id);
    const wire = levelsMeshes.get(lvl.id);
    if (!box || !wire) {
      if (wire) wire.visible = false;
      return;
    }

    wire.position.set(box.x, box.y, box.z);
    wire.scale.set(box.w, box.h, box.d);
    wire.visible = true;
  });

  // Adjust department wireframe scales and line distances
  depts.forEach(dept => {
    const key = `${dept.name}@${dept.orbit}`;
    const box = deptAABBs.get(key);
    const wire = deptsMeshes.get(dept.name);
    if (!box || !wire) {
      if (wire) wire.visible = false;
      return;
    }

    wire.position.set(box.x, box.y, box.z);
    wire.scale.set(box.w, box.h, box.d);
    wire.computeLineDistances();
    wire.visible = true;
  });
}
