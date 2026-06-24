import * as THREE from 'three';
import { selectShard, selectSocket, deselectAll } from '../selection.js';
import { updateFocusVisuals } from '../focus.js';
import { store } from '../../store/store.js';
import { resolveRaycastHit } from '../collision_manager.js';
import { transformControls } from '../transform.js';
import { shardMeshes, socketMeshes } from '../../scene_builder.js';

const hoverWireMaterial = new THREE.LineBasicMaterial({
  color: 0x10b981, // Emerald Green
  transparent: true,
  opacity: 0.95
});

function findClickedDomain(raycaster) {
  const placementData = store.get('placementData');
  if (!placementData) return null;

  const ray = raycaster.ray;
  if (Math.abs(ray.direction.y) < 0.0001) return null;

  const visScale = store.get('visScale') || 1.0;
  const levels = placementData.levels || [];
  const depts = placementData.departments || [];

  let bestDomain = null;
  let minT = Infinity;

  levels.forEach(lvl => {
    // Height plane of the level in Three.js (Y-axis)
    // We project onto the mid-plane of the level height: (lvl.z_start + lvl.height / 2) * visScale
    const y_level = (lvl.z_start + lvl.height / 2) * visScale;
    const t = (y_level - ray.origin.y) / ray.direction.y;

    if (t > 0 && t < minT) {
      const px = ray.origin.x + t * ray.direction.x;
      const pz = ray.origin.z + t * ray.direction.z;

      const x_vox = px / visScale;
      const z_vox = pz / visScale;

      // 1. Check if we fall inside any Department on this level
      const dept = depts.find(d => {
        if (d.orbit !== lvl.id) return false;
        return x_vox >= d.position.x && x_vox <= (d.position.x + d.size.w) &&
               z_vox >= d.position.z && z_vox <= (d.position.z + d.size.d);
      });

      if (dept) {
        bestDomain = { type: 'dept', orbit: dept.orbit, name: dept.name };
        minT = t;
        return;
      }

      // 2. If not in a department, check if we fall inside the Level's overall AABB bounds
      const levelShards = placementData.shards.filter(s => s.orbit === lvl.id);
      if (levelShards.length > 0) {
        const xMin = Math.min(...levelShards.map(s => s.position.x));
        const xMax = Math.max(...levelShards.map(s => s.position.x + s.size.w));
        const zMin = Math.min(...levelShards.map(s => s.position.z));
        const zMax = Math.max(...levelShards.map(s => s.position.z + s.size.d));

        if (x_vox >= xMin && x_vox <= xMax && z_vox >= zMin && z_vox <= zMax) {
          bestDomain = { type: 'level', levelId: lvl.id };
          minT = t;
        }
      }
    }
  });

  return bestDomain;
}

export class InspectMode {
  constructor() {
    this.lastClickTime = 0;
    this.lastClickX = 0;
    this.lastClickY = 0;
    this.hoveredKey = null;
    this.hoveredType = null;
  }

  enter() {
    document.body.style.cursor = 'default';
    updateFocusVisuals();
  }

  exit() {
    this.resetHover();
  }

  highlightObject(key, type) {
    this.resetHover();

    this.hoveredKey = key;
    this.hoveredType = type;

    if (type === 'shard') {
      const mesh = shardMeshes[key];
      if (mesh) {
        const mainWire = mesh.userData.mainWire;
        if (mainWire) {
          mainWire.material = hoverWireMaterial;
        }
      }
    } else if (type === 'socket') {
      const group = socketMeshes[key];
      if (group) {
        const backing = group.userData.backingMesh;
        const instMesh = group.children.find(c => c.isInstancedMesh);
        if (backing) {
          backing.material.opacity = 0.8;
          backing.material.needsUpdate = true;
        }
        if (instMesh) {
          instMesh.material.opacity = 0.95;
          instMesh.material.needsUpdate = true;
        }
      }
    }
  }

  resetHover() {
    if (!this.hoveredKey) return;

    if (this.hoveredType === 'shard') {
      const mesh = shardMeshes[this.hoveredKey];
      if (mesh) {
        updateFocusVisuals();
      }
    } else if (this.hoveredType === 'socket') {
      updateFocusVisuals();
    }

    this.hoveredKey = null;
    this.hoveredType = null;
  }

  onPointerDown(event, raycaster) {
    if (event.button !== 0) return false;

    // If TransformControls is active and click hits its gizmo axes, let it handle the event
    if (transformControls && transformControls.visible && transformControls.axis !== null) {
      console.log("[InspectMode] Clicked on Gizmo axis, ignoring");
      return true;
    }

    const now = performance.now();
    const isDoubleClick = (now - this.lastClickTime < 300) &&
                          (Math.abs(event.clientX - this.lastClickX) < 10) &&
                          (Math.abs(event.clientY - this.lastClickY) < 10);
    this.lastClickTime = now;
    this.lastClickX = event.clientX;
    this.lastClickY = event.clientY;

    let hit = resolveRaycastHit(raycaster);

    // If no shard/socket was hit directly, try mathematical projection on level/dept planes
    if (!hit) {
      hit = findClickedDomain(raycaster);
    }

    console.log("[InspectMode] pointerdown | isDoubleClick:", isDoubleClick, " | hit:", hit ? { type: hit.type, key: hit.key || hit.name || hit.levelId } : null);

    // Double click: Drill-Down / Drill-Up
    if (isDoubleClick) {
      if (hit) {
        if (hit.type === 'shard') {
          const placementData = store.get('placementData');
          const shard = placementData?.shards.find(s => s.key === hit.key);
          if (shard) {
            console.log("[InspectMode] Double click -> Drill-Down to Shard:", shard.key);
            store.setMultiple({
              focusedLevelId: shard.orbit,
              selectedDeptName: shard.dept,
              focusedShardKey: shard.key,
              selectedShardKey: shard.key
            });
            selectShard(shard.key, false);
          }
          return true;
        } else if (hit.type === 'socket') {
          console.log("[InspectMode] Double click -> Select Socket:", hit.key);
          selectSocket(hit.key);
          return true;
        } else if (hit.type === 'dept') {
          console.log("[InspectMode] Double click -> Focus Dept:", hit.name);
          store.setMultiple({
            focusedLevelId: hit.orbit,
            selectedDeptName: hit.name,
            focusedShardKey: null,
            selectedShardKey: null
          });
          deselectAll();
          return true;
        } else if (hit.type === 'level') {
          console.log("[InspectMode] Double click -> Focus Level:", hit.levelId);
          store.setMultiple({
            focusedLevelId: hit.levelId,
            selectedDeptName: null,
            focusedShardKey: null,
            selectedShardKey: null
          });
          deselectAll();
          return true;
        }
      } else {
        // Double click in empty space: Drill-Up
        const focusedShardKey = store.get('focusedShardKey');
        const selectedDeptName = store.get('selectedDeptName');
        const focusedLevelId = store.get('focusedLevelId');

        console.log("[InspectMode] Double click in empty space -> Drill-Up | current state:", { focusedShardKey, selectedDeptName, focusedLevelId });

        if (focusedShardKey) {
          store.set('focusedShardKey', null);
        } else if (selectedDeptName) {
          store.set('selectedDeptName', null);
        } else if (focusedLevelId !== null) {
          store.set('focusedLevelId', null);
        }
        return true;
      }
    } else {
      // Single click: Selection and click in empty space deselect
      if (hit) {
        if (hit.type === 'shard') {
          console.log("[InspectMode] Single click -> Select Shard:", hit.key);
          selectShard(hit.key, event.ctrlKey || event.metaKey);
          return true;
        } else if (hit.type === 'socket') {
          console.log("[InspectMode] Single click -> Select Socket:", hit.key);
          selectSocket(hit.key);
          return true;
        }
      } else {
        // Click in empty space (including double-click frames if not double clicking)
        if (!event.ctrlKey && !event.metaKey) {
          console.log("[InspectMode] Single click in empty space -> deselectAll");
          deselectAll();
        }
      }
    }

    return false;
  }

  onPointerMove(event, raycaster) {
    // Prevent hover raycasting when hovering over the TransformControls axes
    if (transformControls && transformControls.visible) {
      const gizmoHits = raycaster.intersectObjects(transformControls.children, true);
      if (gizmoHits.length > 0) {
        this.resetHover();
        document.body.style.cursor = 'default';
        return;
      }
    }

    const selShardKey = store.get('selectedShardKey');
    const selSocketKey = store.get('selectedSocketKey');

    // 1. Hover highlights if nothing is selected
    if (!selShardKey && !selSocketKey) {
      const bestHit = resolveRaycastHit(raycaster);

      if (bestHit) {
        if (bestHit.type === 'socket') {
          const key = bestHit.key;
          if (this.hoveredKey !== key || this.hoveredType !== 'socket') {
            this.highlightObject(key, 'socket');
          }
          document.body.style.cursor = 'pointer';
          return;
        } else if (bestHit.type === 'shard') {
          const key = bestHit.key;
          if (this.hoveredKey !== key || this.hoveredType !== 'shard') {
            this.highlightObject(key, 'shard');
          }
          document.body.style.cursor = 'pointer';
          return;
        }
      }

      // Check level/department hover
      const domainHit = findClickedDomain(raycaster);
      if (domainHit) {
        this.resetHover();
        document.body.style.cursor = 'pointer';
        return;
      }

      this.resetHover();
      document.body.style.cursor = 'default';
      return;
    }

    document.body.style.cursor = 'default';
  }

  onPointerUp(event, raycaster) {}

  onKeyDown(event) {
    if (event.key === 'Escape') {
      const selShardKey = store.get('selectedShardKey');
      const selSocketKey = store.get('selectedSocketKey');
      const focusedShardKey = store.get('focusedShardKey');
      const selectedDeptName = store.get('selectedDeptName');
      const focusedLevelId = store.get('focusedLevelId');

      if (selShardKey || selSocketKey) {
        deselectAll();
      } else if (focusedShardKey) {
        store.set('focusedShardKey', null);
      } else if (selectedDeptName) {
        store.set('selectedDeptName', null);
      } else if (focusedLevelId !== null) {
        store.set('focusedLevelId', null);
      }
      return true; // Handled Escape locally, prevent popping modes
    }
    return false;
  }
}
