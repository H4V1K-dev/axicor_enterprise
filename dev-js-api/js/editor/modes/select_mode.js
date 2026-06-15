import { shardMeshes, socketMeshes } from '../../scene_builder.js';
import { selectShard, selectSocket, deselectAll, selectRoute } from '../selection.js';
import { store } from '../../store/store.js';
import { on, off, EVENTS } from '../../store/event_bus.js';
import { updateFocusVisuals } from '../focus.js';
import { modeManager } from '../../editor.js';
import { transformControls } from '../transform.js';
import { resolveRaycastHit } from '../collision_manager.js';

export class SelectMode {
  constructor() {
    this.hoveredType = null;
    this.hoveredKey = null;
    this.onSelectionChanged = this.onSelectionChanged.bind(this);
  }

  enter() {
    document.body.style.cursor = 'default';
    this.applyModeVisuals();
    on(EVENTS.SELECTION_CHANGED, this.onSelectionChanged);
  }

  exit() {
    off(EVENTS.SELECTION_CHANGED, this.onSelectionChanged);
    this.resetHover();
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
    // Reset all shards to opaque matte state (since nothing is selected)
    for (const mesh of Object.values(shardMeshes)) {
      mesh.material.opacity = 1.0;
      mesh.material.transparent = false;
      mesh.material.needsUpdate = true;

      const mainWire = mesh.children.find(c => c.name === "main_wireframe");
      if (mainWire) {
        mainWire.visible = true;
        mainWire.material.opacity = 0.85;
        mainWire.material.transparent = true;
        mainWire.material.needsUpdate = true;
      }
    }

    // Reset all sockets to standard fully opaque state
    for (const group of Object.values(socketMeshes)) {
      const backing = group.userData.backingMesh;
      const instMesh = group.children.find(c => c.isInstancedMesh);
      if (backing) {
        backing.material.opacity = 0.7;
        backing.material.color.setHex(group.userData.originalBackingColor || 0x050508);
        backing.material.visible = (group.userData.originalBackingVisible !== false);
        backing.material.needsUpdate = true;
      }
      if (instMesh) {
        instMesh.material.opacity = 1.0;
        instMesh.material.transparent = false;
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
        // Make parent monolith highly transparent for X-ray view
        mesh.material.opacity = 0.15;
        mesh.material.transparent = true;
        mesh.material.needsUpdate = true;

        const mainWire = mesh.children.find(c => c.name === "main_wireframe");
        if (mainWire) {
          mainWire.material.opacity = 0.3;
          mainWire.material.needsUpdate = true;
        }

        // Show inner layers & dividers
        mesh.children.forEach(child => {
          if (child.userData) {
            if (child.userData.layerIndex !== undefined) {
              child.visible = true;
              child.material.opacity = 0.5;
              child.material.needsUpdate = true;
              
              const wire = child.children.find(c => c.name === "wireframe");
              if (wire) {
                wire.material.opacity = 0.8;
                wire.material.needsUpdate = true;
              }
            } else if (child.userData.isDivider) {
              child.visible = true;
              child.material.opacity = 0.0;
              child.material.needsUpdate = true;
              
              const border = child.children.find(c => c.name === "border");
              if (border) {
                border.material.opacity = 0.3;
                border.material.needsUpdate = true;
              }
            }
          }
        });
      }
    } else if (type === 'socket') {
      const group = socketMeshes[key];
      if (group) {
        const backing = group.userData.backingMesh;
        const instMesh = group.children.find(c => c.isInstancedMesh);
        if (backing) {
          backing.material.opacity = 0.9;
          backing.material.color.setHex(0x8b9cf7);
          backing.material.visible = true;
          backing.material.needsUpdate = true;
        }
        if (instMesh) {
          instMesh.material.opacity = 1.0;
          instMesh.material.transparent = false;
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
        // Restore opacity according to current focus state
        const selShardKey = store.get('selectedShardKey');
        const selSocketKey = store.get('selectedSocketKey');
        const selRouteKey = store.get('selectedRouteKey');
        const isAnySelected = !!(selShardKey || selSocketKey || selRouteKey);

        if (isAnySelected) {
          // If something is selected, reset to focus states (dimmed)
          updateFocusVisuals();
        } else {
          // If no selection, reset to default opaque state
          mesh.material.opacity = 1.0;
          mesh.material.transparent = false;
          mesh.material.needsUpdate = true;
          const mainWire = mesh.children.find(c => c.name === "main_wireframe");
          if (mainWire) {
            mainWire.visible = true;
            mainWire.material.opacity = 0.85;
            mainWire.material.transparent = true;
            mainWire.material.needsUpdate = true;
          }
        }

        // Hide inner layers & dividers
        mesh.children.forEach(child => {
          if (child.userData && (child.userData.layerIndex !== undefined || child.userData.isDivider)) {
            child.visible = false;
          }
        });
      }
    } else if (this.hoveredType === 'socket') {
      const key = this.hoveredKey;
      const group = socketMeshes[key];
      if (group) {
        // Restore focus/dim state for this socket
        updateFocusVisuals();
      }
    }

    this.hoveredKey = null;
    this.hoveredType = null;
  }

  onPointerDown(event, raycaster) {
    if (event.button !== 0) return false;

    // 1. Check if click falls on the TransformControls gizmo axes
    if (transformControls && transformControls.visible && transformControls.axis !== null) {
      modeManager.setMode('translate');
      return true;
    }

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
      deselectAll();
      return true;
    }

    // Single click handles selection
    if (event.detail <= 1) {
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
    const selShardKey = store.get('selectedShardKey');
    const selSocketKey = store.get('selectedSocketKey');

    const bestHit = resolveRaycastHit(raycaster);

    if (bestHit) {
      if (bestHit.type === 'socket') {
        const key = bestHit.key;
        if (selSocketKey === key) {
          this.resetHover();
          document.body.style.cursor = 'default';
          return;
        }
        if (this.hoveredKey !== key || this.hoveredType !== 'socket') {
          this.highlightObject(key, 'socket');
        }
        document.body.style.cursor = 'pointer';
        return;
      } else if (bestHit.type === 'shard') {
        const key = bestHit.key;
        if (selShardKey === key) {
          this.resetHover();
          document.body.style.cursor = 'default';
          return;
        }
        if (this.hoveredKey !== key || this.hoveredType !== 'shard') {
          this.highlightObject(key, 'shard');
        }
        document.body.style.cursor = 'pointer';
        return;
      }
    }

    this.resetHover();
    document.body.style.cursor = 'default';
  }

  onPointerUp(event, raycaster) {}

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
