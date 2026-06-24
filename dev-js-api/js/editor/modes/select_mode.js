import * as THREE from 'three';
import { shardMeshes, socketMeshes } from '../../scene_builder.js';
import { selectShard, selectSocket, deselectAll } from '../selection.js';
import { store } from '../../store/store.js';
import { on, off, EVENTS } from '../../store/event_bus.js';
import { updateFocusVisuals } from '../focus.js';
import { modeManager } from '../../editor.js';
import { transformControls } from '../transform.js';
import { resolveRaycastHit } from '../collision_manager.js';

const hoverBodyMaterial = new THREE.MeshStandardMaterial({
  color: 0x6366f1,
  transparent: true,
  opacity: 0.15,
  roughness: 0.6,
  metalness: 0.1
});

const hoverWireMaterial = new THREE.LineBasicMaterial({
  color: 0x6366f1,
  transparent: true,
  opacity: 0.3
});

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
    updateFocusVisuals();
  }

  highlightObject(key, type) {
    this.resetHover();

    this.hoveredKey = key;
    this.hoveredType = type;

    if (type === 'shard') {
      const mesh = shardMeshes[key];
      if (mesh) {
        const body = mesh.userData.body;
        const mainWire = mesh.userData.mainWire;
        if (body) {
          body.material = hoverBodyMaterial;
        }
        if (mainWire) {
          mainWire.material = hoverWireMaterial;
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
        // Restore opacity/visibility according to current focus and level state
        updateFocusVisuals();

        // Hide inner layers & dividers
        mesh.children.forEach(child => {
          if (child.userData && (child.userData.layerIndex !== undefined || child.userData.isDivider)) {
            child.visible = false;
          }
        });
      }
    } else if (this.hoveredType === 'socket') {
      updateFocusVisuals();
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
        if (bestHit.type === 'socket') {
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
