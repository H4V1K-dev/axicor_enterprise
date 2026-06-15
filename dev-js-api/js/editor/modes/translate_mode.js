import { shardMeshes, socketMeshes } from '../../scene_builder.js';
import { selectShard, selectSocket, deselectAll, selectRoute } from '../selection.js';
import { transformControls } from '../transform.js';
import { updateFocusVisuals } from '../focus.js';
import { store } from '../../store/store.js';
import { showToast } from '../../ui/toast.js';
import { on, off, EVENTS } from '../../store/event_bus.js';
import { modeManager } from '../../editor.js';
import { resolveRaycastHit } from '../collision_manager.js';
import {
  onPointerDown as onDividerPointerDown,
  onPointerMove as onDividerPointerMove,
  onPointerUp as onDividerPointerUp,
  isDragging as isDraggingDivider
} from '../divider_drag.js';

export class TranslateMode {
  constructor() {
    this.hoveredType = null;
    this.hoveredKey = null;
    this.onSelectionChanged = this.onSelectionChanged.bind(this);
  }

  enter() {
    showToast("Режим перемещения (Gizmo) активен", "success");
    
    // Auto attach gizmo if an object is already selected
    const selShardKey = store.get('selectedShardKey');
    const selSocketKey = store.get('selectedSocketKey');
    if (selShardKey) {
      selectShard(selShardKey);
    } else if (selSocketKey) {
      selectSocket(selSocketKey);
    }

    this.applyModeVisuals();
    on(EVENTS.SELECTION_CHANGED, this.onSelectionChanged);
  }

  exit() {
    off(EVENTS.SELECTION_CHANGED, this.onSelectionChanged);
    this.resetHover();

    if (transformControls) {
      const selShardKey = store.get('selectedShardKey');
      const selSocketKey = store.get('selectedSocketKey');
      if (!selShardKey && !selSocketKey) {
        transformControls.detach();
      }
    }

    updateFocusVisuals();
  }

  onSelectionChanged() {
    this.applyModeVisuals();
  }

  applyModeVisuals() {
    const selShardKey = store.get('selectedShardKey');
    const selSocketKey = store.get('selectedSocketKey');

    if (selShardKey || selSocketKey) {
      updateFocusVisuals();
      return;
    }

    this.dimAll();
  }

  dimAll() {
    // Dim shards
    for (const mesh of Object.values(shardMeshes)) {
      mesh.material.opacity = 0.15;
      mesh.material.transparent = true;
      mesh.material.needsUpdate = true;

      const mainWire = mesh.children.find(c => c.name === "main_wireframe");
      if (mainWire) {
        mainWire.material.opacity = 0.15;
        mainWire.material.needsUpdate = true;
      }
    }

    // Dim sockets
    for (const group of Object.values(socketMeshes)) {
      const backing = group.userData.backingMesh;
      const instMesh = group.children.find(c => c.isInstancedMesh);
      if (backing) {
        backing.material.opacity = 0.1;
        backing.material.needsUpdate = true;
      }
      if (instMesh) {
        instMesh.material.opacity = 0.1;
        instMesh.material.needsUpdate = true;
      }
    }
  }

  highlightObject(key, type) {
    this.resetHover();

    this.hoveredKey = key;
    this.hoveredType = type;

    if (type === 'shard') {
      const mesh = shardMeshes[key];
      if (mesh) {
        mesh.material.opacity = 0.5;
        mesh.material.needsUpdate = true;
        const mainWire = mesh.children.find(c => c.name === "main_wireframe");
        if (mainWire) {
          mainWire.material.opacity = 0.95;
          mainWire.material.needsUpdate = true;
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
        mesh.material.opacity = 0.15;
        mesh.material.needsUpdate = true;
        const mainWire = mesh.children.find(c => c.name === "main_wireframe");
        if (mainWire) {
          mainWire.material.opacity = 0.15;
          mainWire.material.needsUpdate = true;
        }
      }
    } else if (this.hoveredType === 'socket') {
      const group = socketMeshes[this.hoveredKey];
      if (group) {
        const backing = group.userData.backingMesh;
        const instMesh = group.children.find(c => c.isInstancedMesh);
        if (backing) {
          backing.material.opacity = 0.1;
          backing.material.needsUpdate = true;
        }
        if (instMesh) {
          instMesh.material.opacity = 0.1;
          instMesh.material.needsUpdate = true;
        }
      }
    }

    this.hoveredKey = null;
    this.hoveredType = null;
  }

  onPointerDown(event, raycaster) {
    if (event.button !== 0) return false;
    const selShardKey = store.get('selectedShardKey');
    const selSocketKey = store.get('selectedSocketKey');

    // Double click handles deselection or popping mode
    if (event.detail >= 2) {
      const bestHit = resolveRaycastHit(raycaster);
      if (bestHit) {
        if (bestHit.type === 'socket' && selSocketKey === bestHit.key) {
          deselectAll();
          modeManager.popMode();
          return true;
        } else if (bestHit.type === 'shard' && selShardKey === bestHit.key) {
          deselectAll();
          modeManager.popMode();
          return true;
        }
      }

      // Clicked in empty space: deselect but stay in mode
      deselectAll();
      return true;
    }

    // Only process edits/selections on single clicks
    if (event.detail <= 1) {
      // 1. Check if click falls on the TransformControls gizmo axes
      if (transformControls && transformControls.visible) {
        const gizmoHits = raycaster.intersectObjects(transformControls.children, true);
        if (gizmoHits.length > 0) {
          return true; // Let TransformControls handle the drag
        }
      }

      // 2. Try layer divider drag first
      if (onDividerPointerDown(event)) {
        return true;
      }

      // 3. Unified raycast hit resolution
      const bestHit = resolveRaycastHit(raycaster);

      if (bestHit) {
        if (bestHit.type === 'control_point') {
          transformControls.attach(bestHit.object);
          return true;
        } else if (bestHit.type === 'route') {
          selectRoute(bestHit.key);
          return true;
        } else if (bestHit.type === 'socket') {
          selectSocket(bestHit.key);
          return true;
        } else if (bestHit.type === 'shard') {
          selectShard(bestHit.key);
          return true;
        }
      }
    }

    return false;
  }

  onPointerMove(event, raycaster) {
    if (isDraggingDivider()) {
      onDividerPointerMove(event);
      return;
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

      this.resetHover();
      document.body.style.cursor = 'default';
      return;
    }

    // 2. Cursor styling if something IS selected (e.g. handles/dividers resizing hover)
    if (!transformControls.dragging) {
      const dividersList = [];
      for (const mesh of Object.values(shardMeshes)) {
        mesh.traverse(child => {
          if (child.userData && child.userData.isDivider && child.visible) {
            dividersList.push(child);
          }
        });
      }

      dividersList.forEach(div => {
        div.material.opacity = 0.0;
        const border = div.children.find(c => c.name === "border");
        if (border) border.material.opacity = 0.3;
      });

      const divHoverHits = raycaster.intersectObjects(dividersList);
      if (divHoverHits.length > 0) {
        const hoveredDiv = divHoverHits[0].object;
        hoveredDiv.material.opacity = 0.15;
        const border = hoveredDiv.children.find(c => c.name === "border");
        if (border) border.material.opacity = 1.0;
        document.body.style.cursor = 'row-resize';
        return;
      }

      document.body.style.cursor = 'auto';
    }
  }

  onPointerUp(event, raycaster) {
    onDividerPointerUp(event);
  }

  onKeyDown(event) {
    if (event.key === 'Escape') {
      const selShardKey = store.get('selectedShardKey');
      const selSocketKey = store.get('selectedSocketKey');
      if (selShardKey || selSocketKey) {
        deselectAll();
        return true; // Handled deselect, stay in mode
      }
      return false; // Not handled, ModeManager will pop mode stack
    }
    return false;
  }
}

