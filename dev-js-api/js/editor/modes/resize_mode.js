import * as THREE from 'three';
import { camera, controls } from '../../viewer.js';
import { shardMeshes, socketMeshes, VIS_SCALE, shardDataMap, rebuildSocket, drawRoutes } from '../../scene_builder.js';
import { selectShard, deselectAll } from '../selection.js';
import { store } from '../../store/store.js';
import { showToast } from '../../ui/toast.js';
import { on, off, emit, EVENTS } from '../../store/event_bus.js';
import { modeManager, checkShardCollision, transformControls } from '../../editor.js';
import { updateFocusVisuals } from '../focus.js';
import {
  onPointerDown as onHandlePointerDown,
  onPointerMove as onHandlePointerMove,
  onPointerUp as onHandlePointerUp,
  isDragging as isDraggingHandle
} from '../handle_drag.js';

let altPressed = false;
window.addEventListener('keydown', (e) => {
  if (e.key === 'Alt') {
    altPressed = true;
  }
});
window.addEventListener('keyup', (e) => {
  if (e.key === 'Alt') {
    altPressed = false;
  }
});
window.addEventListener('blur', () => {
  altPressed = false;
});

export class ResizeMode {
  constructor() {
    this.hoveredHandle = null;
    this.isDragging = false;
    this.activeHandle = null;
    
    // Drag session variables
    this.dragPlane = new THREE.Plane();
    this.dragStartPoint = new THREE.Vector3();
    this.initialW = 0;
    this.initialD = 0;
    this.initialH = 0;
    this.initialPosition = new THREE.Vector3();
    this.initialSocketOffsets = {}; // socketKey -> originalOffset

    this.onSelectionChanged = this.onSelectionChanged.bind(this);
  }

  enter() {
    const selShardKey = store.get('selectedShardKey');
    const selSocketKey = store.get('selectedSocketKey');
    if (!selShardKey && !selSocketKey) {
      showToast("Выберите шард или сокет для изменения размера", "info");
      // Fallback to inspect mode
      setTimeout(() => {
        modeManager.setMode('inspect');
      }, 0);
      return;
    }

    showToast("Режим изменения размеров активен", "success");
    
    if (transformControls) {
      transformControls.detach();
    }

    this.applyModeVisuals();
    if (selShardKey) {
      this.spawnHandles();
    }

    on(EVENTS.SELECTION_CHANGED, this.onSelectionChanged);
  }

  exit() {
    off(EVENTS.SELECTION_CHANGED, this.onSelectionChanged);
    this.removeHandles();
    this.isDragging = false;
    this.activeHandle = null;
    controls.enabled = true;
    document.body.style.cursor = 'default';
  }

  onSelectionChanged() {
    const selShardKey = store.get('selectedShardKey');
    const selSocketKey = store.get('selectedSocketKey');
    if (!selShardKey && !selSocketKey) {
      modeManager.setMode('inspect');
      return;
    }

    if (transformControls) {
      transformControls.detach();
    }

    this.removeHandles();
    this.applyModeVisuals();
    if (selShardKey) {
      this.spawnHandles();
    }
  }

  applyModeVisuals() {
    updateFocusVisuals();
  }

  spawnHandles() {
    const selShardKey = store.get('selectedShardKey');
    if (!selShardKey) return;
    const shardMesh = shardMeshes.get(selShardKey);
    if (!shardMesh) return;

    const w = shardMesh.geometry.parameters.width;
    const h = shardMesh.geometry.parameters.height; // Three Y height
    const d = shardMesh.geometry.parameters.depth;  // Three Z depth

    const handleGeo = new THREE.PlaneGeometry(1.2 * VIS_SCALE, 1.2 * VIS_SCALE);
    
    // Vibrant cyan/teal glowing material
    const handleMat = new THREE.MeshBasicMaterial({
      color: 0x00ffcc,
      transparent: true,
      opacity: 0.8,
      side: THREE.DoubleSide,
      depthTest: false,
      depthWrite: false
    });

    const normals = [
      { name: 'PX', normal: new THREE.Vector3(1, 0, 0), axis: 'x', pos: new THREE.Vector3(w / 2, 0, 0) },
      { name: 'NX', normal: new THREE.Vector3(-1, 0, 0), axis: 'x', pos: new THREE.Vector3(-w / 2, 0, 0) },
      { name: 'PY', normal: new THREE.Vector3(0, 1, 0), axis: 'y', pos: new THREE.Vector3(0, h / 2, 0) },
      { name: 'NY', normal: new THREE.Vector3(0, -1, 0), axis: 'y', pos: new THREE.Vector3(0, -h / 2, 0) },
      { name: 'PZ', normal: new THREE.Vector3(0, 0, 1), axis: 'z', pos: new THREE.Vector3(0, 0, d / 2) },
      { name: 'NZ', normal: new THREE.Vector3(0, 0, -1), axis: 'z', pos: new THREE.Vector3(0, 0, -d / 2) }
    ];

    normals.forEach(info => {
      const mesh = new THREE.Mesh(handleGeo, handleMat.clone());
      mesh.position.copy(info.pos);
      if (info.axis === 'x') {
        mesh.rotateY(Math.PI / 2);
      } else if (info.axis === 'y') {
        mesh.rotateX(Math.PI / 2);
      }
      mesh.name = `resize_handle_${info.name}`;
      mesh.renderOrder = 9999;
      mesh.userData = {
        isResizeHandle: true,
        normal: info.normal,
        axis: info.axis,
        handleName: info.name,
        shardKey: selShardKey
      };
      shardMesh.add(mesh);
    });
  }

  removeHandles() {
    for (const mesh of shardMeshes.values()) {
      const toRemove = [];
      mesh.traverse(child => {
        if (child.userData && child.userData.isResizeHandle) {
          toRemove.push(child);
        }
      });
      toRemove.forEach(child => {
        mesh.remove(child);
        if (child.geometry) child.geometry.dispose();
        if (child.material) {
          if (Array.isArray(child.material)) {
            child.material.forEach(m => m.dispose());
          } else {
            child.material.dispose();
          }
        }
      });
    }
  }

  computeDynamicLimit(selShardKey, localNormal) {
    return Infinity;
  }

  onUpdate(dt) {
    const selShardKey = store.get('selectedShardKey');
    if (!selShardKey) return;
    const shardMesh = shardMeshes.get(selShardKey);
    if (!shardMesh) return;

    const camPos = camera.position;
    const minD = 40.0;
    const maxD = 250.0;
    const minScale = 1.0;
    const maxScale = 5.0;

    shardMesh.traverse(child => {
      if (child.userData && child.userData.isResizeHandle) {
        const worldPos = new THREE.Vector3();
        child.getWorldPosition(worldPos);
        const distWorld = camPos.distanceTo(worldPos);
        const distVoxels = distWorld / VIS_SCALE;

        let ratio = (distVoxels - minD) / (maxD - minD);
        ratio = Math.max(0, Math.min(1, ratio));
        const s = minScale + ratio * (maxScale - minScale);

        child.scale.set(s, s, s);
      }
    });
  }

  onPointerDown(event, raycaster) {
    if (event.button !== 0) return false;

    const selShardKey = store.get('selectedShardKey');
    const selSocketKey = store.get('selectedSocketKey');

    if (selSocketKey) {
      if (onHandlePointerDown(event)) {
        return true;
      }
    }

    if (selShardKey) {
      const shardMesh = shardMeshes.get(selShardKey);
      if (!shardMesh) return false;
      const sd = shardDataMap.get(shardMesh.uuid);
      if (!sd) return false;

      // Collect active handles
      const handles = [];
      shardMesh.traverse(child => {
        if (child.userData && child.userData.isResizeHandle) {
          handles.push(child);
        }
      });

      const hits = raycaster.intersectObjects(handles);
      if (hits.length > 0) {
        const clickedHandle = hits[0].object;
        this.isDragging = true;
        this.activeHandle = clickedHandle;
        controls.enabled = false; // Disable OrbitControls during resizing

        // Retrieve initial dimensions in voxels
        this.initialW = sd.size.w;
        this.initialD = sd.size.d;
        this.initialH = sd.size.h;
        this.initialPosition.copy(shardMesh.position);
        this.initialShardState = JSON.parse(JSON.stringify(sd));

        const localNormal = clickedHandle.userData.normal;
        this.dynamicLimit = this.computeDynamicLimit(selShardKey, localNormal);

        // Save initial socket offsets
        this.initialSocketOffsets = {};
        if (sd.sockets) {
          sd.sockets.forEach(sock => {
            const socketKey = `${sd.key}.${sock.name}`;
            const socketGroup = socketMeshes.get(socketKey);
            if (socketGroup) {
              const defaultZ = socketGroup.userData.faceSign * (this.initialH / 2);
              this.initialSocketOffsets[socketKey] = {
                x: socketGroup.userData.originalOffset.x,
                y: socketGroup.userData.originalOffset.y,
                z: socketGroup.userData.originalOffset.z !== undefined ? socketGroup.userData.originalOffset.z : defaultZ
              };
            }
          });
        }

        // Math setup: construct drag plane containing the axis line and facing the camera
        const U = localNormal.clone().applyQuaternion(shardMesh.quaternion).normalize();
        
        const V_cam = new THREE.Vector3();
        camera.getWorldDirection(V_cam);
        
        const V_perp = new THREE.Vector3().crossVectors(U, V_cam).normalize();
        const N_plane = new THREE.Vector3().crossVectors(U, V_perp).normalize();

        const worldPos = new THREE.Vector3();
        clickedHandle.getWorldPosition(worldPos);
        this.dragPlane.setFromNormalAndCoplanarPoint(N_plane, worldPos);

        // Find initial plane intersection
        raycaster.ray.intersectPlane(this.dragPlane, this.dragStartPoint);
        return true;
      }
    }

    return false;
  }

  onPointerMove(event, raycaster) {
    if (isDraggingHandle()) {
      onHandlePointerMove(event);
      return;
    }

    const selShardKey = store.get('selectedShardKey');
    const selSocketKey = store.get('selectedSocketKey');

    if (selSocketKey) {
      // Socket drag is handled inside handle_drag.js
      return;
    }

    if (selShardKey) {
      const shardMesh = shardMeshes.get(selShardKey);
      if (!shardMesh) return;
      const sd = shardDataMap.get(shardMesh.uuid);
      if (!sd) return;

      if (!this.isDragging || !this.activeHandle) {
        // Hover highlight style cursor
        const handles = [];
        shardMesh.traverse(child => {
          if (child.userData && child.userData.isResizeHandle) {
            handles.push(child);
          }
        });

        const hits = raycaster.intersectObjects(handles);
        let hoveredObj = null;
        if (hits.length > 0) {
          hoveredObj = hits[0].object;
          const axis = hoveredObj.userData.axis;
          if (axis === 'x') {
            document.body.style.cursor = 'col-resize';
          } else if (axis === 'y') {
            document.body.style.cursor = 'row-resize';
          } else if (axis === 'z') {
            document.body.style.cursor = 'ns-resize';
          }
        } else {
          document.body.style.cursor = 'default';
        }

        // Apply hover colors (Emerald Green when hovered, Turquoise when idle)
        handles.forEach(h => {
          if (h === hoveredObj) {
            h.material.color.setHex(0x10b981);
          } else {
            h.material.color.setHex(0x00ffcc);
          }
        });
        return;
      }

      // Drag processing
      const intersectPoint = new THREE.Vector3();
      if (raycaster.ray.intersectPlane(this.dragPlane, intersectPoint)) {
        const localNormal = this.activeHandle.userData.normal;
        const axis = this.activeHandle.userData.axis;

        // Project world delta vector onto the motion axis
        const U = localNormal.clone().applyQuaternion(shardMesh.quaternion).normalize();
        const Delta = new THREE.Vector3().subVectors(intersectPoint, this.dragStartPoint);
        const deltaVoxels = Delta.dot(U) / VIS_SCALE;

        // Snap delta to step increment from store settings
        const gridSnap = store.get('gridSnapStep') ?? 1;
        const RESIZE_STEP = altPressed ? 1 : (gridSnap > 0 ? gridSnap : 1);
        const steppedDelta = Math.round(deltaVoxels / RESIZE_STEP) * RESIZE_STEP;

        // Maintain highlighted green color on active handle during drag, other handles turquoise
        const handles = [];
        shardMesh.traverse(child => {
          if (child.userData && child.userData.isResizeHandle) {
            handles.push(child);
          }
        });
        handles.forEach(h => {
          if (h === this.activeHandle) {
            h.material.color.setHex(0x10b981);
          } else {
            h.material.color.setHex(0x00ffcc);
          }
        });

        // Iterative step verification to prevent tunnel collision issues
        let finalDelta = 0;
        const stepsCount = Math.abs(steppedDelta) / RESIZE_STEP;
        const stepDir = Math.sign(steppedDelta);

        const MAX_SHARD_SIZE_XY = 1024;
        const MAX_SHARD_SIZE_Z = 256;
        const MIN_SHARD_SIZE = 10;

        for (let i = 1; i <= stepsCount; i++) {
          const testDelta = i * RESIZE_STEP * stepDir;
          
          let tempW = this.initialW;
          let tempD = this.initialD;
          let tempH = this.initialH;

          if (axis === 'x') tempW += testDelta;
          if (axis === 'y') tempH += testDelta; // Three Y height
          if (axis === 'z') tempD += testDelta; // Three Z depth

          // Clamp sizes
          let clampedW = tempW;
          let clampedD = tempD;
          let clampedH = tempH;

          if (axis === 'x') {
            if (clampedW < MIN_SHARD_SIZE) clampedW = MIN_SHARD_SIZE;
            clampedW = Math.min(MAX_SHARD_SIZE_XY, clampedW);
          }
          if (axis === 'z') {
            if (clampedD < MIN_SHARD_SIZE) clampedD = MIN_SHARD_SIZE;
            clampedD = Math.min(MAX_SHARD_SIZE_XY, clampedD);
          }
          if (axis === 'y') {
            if (clampedH < MIN_SHARD_SIZE) clampedH = MIN_SHARD_SIZE;
            clampedH = Math.min(MAX_SHARD_SIZE_Z, clampedH);
          }

          // Effective delta based on clamping
          let effectiveDelta = 0;
          if (axis === 'x') effectiveDelta = clampedW - this.initialW;
          if (axis === 'z') effectiveDelta = clampedD - this.initialD;
          if (axis === 'y') effectiveDelta = clampedH - this.initialH;

          const localShift = localNormal.clone()
            .multiplyScalar(effectiveDelta * VIS_SCALE / 2)
            .applyQuaternion(shardMesh.quaternion);
          
          const tempPosition = this.initialPosition.clone().add(localShift);
          const newSize2D = { w: clampedW, d: clampedD, h: clampedH };

          // Shrinking (testDelta < 0) is always allowed as it resolves overlaps
          if (testDelta > 0 && checkShardCollision(selShardKey, tempPosition, newSize2D)) {
            // Collision detected at this step, stop at previous valid step
            break;
          }

          // Vertical limits check removed as orbits/levels are disabled
          if (axis === 'y' && testDelta > 0) {
            // No vertical constraints, fully free movement/resize
          }
          finalDelta = testDelta;
        }

        // Calculate final sizes based on maximum valid delta found
        let finalW = this.initialW;
        let finalD = this.initialD;
        let finalH = this.initialH;

        if (axis === 'x') finalW += finalDelta;
        if (axis === 'z') finalD += finalDelta;
        if (axis === 'y') finalH += finalDelta;

        // Final bounds enforcement and warnings
        if (axis === 'x') {
          if (finalW < MIN_SHARD_SIZE) {
            if (finalW <= 9) showToast("Размер шарда не может быть меньше 10 вокселей", "warning");
            finalW = MIN_SHARD_SIZE;
          }
          finalW = Math.min(MAX_SHARD_SIZE_XY, finalW);
        }
        if (axis === 'z') {
          if (finalD < MIN_SHARD_SIZE) {
            if (finalD <= 9) showToast("Размер шарда не может быть меньше 10 вокселей", "warning");
            finalD = MIN_SHARD_SIZE;
          }
          finalD = Math.min(MAX_SHARD_SIZE_XY, finalD);
        }
        if (axis === 'y') {
          if (finalH < MIN_SHARD_SIZE) {
            if (finalH <= 9) showToast("Размер шарда не может быть меньше 10 вокселей", "warning");
            finalH = MIN_SHARD_SIZE;
          }
          finalH = Math.min(MAX_SHARD_SIZE_Z, finalH);
        }

        let finalEffectiveDelta = 0;
        if (axis === 'x') finalEffectiveDelta = finalW - this.initialW;
        if (axis === 'z') finalEffectiveDelta = finalD - this.initialD;
        if (axis === 'y') finalEffectiveDelta = finalH - this.initialH;

        const finalPosition = this.initialPosition.clone().add(localShift);

        // Apply to mesh
        shardMesh.position.copy(finalPosition);

        // Dynamically reconstruct geometry for real-time visual feedback
        this.updateMeshGeometry(shardMesh, finalW, finalD, finalH);

        // Handle child attachments (sockets, layers, handles)
        this.updateAttachments(shardMesh, finalW, finalD, finalH, localNormal, finalEffectiveDelta);
      }
    }
  }

  onPointerUp(event, raycaster) {
    if (isDraggingHandle()) {
      onHandlePointerUp(event);
      return;
    }

    if (this.isDragging) {
      this.isDragging = false;
      this.activeHandle = null;
      controls.enabled = true;
      document.body.style.cursor = 'default';

      const selShardKey = store.get('selectedShardKey');
      if (!selShardKey) return;
      const shardMesh = shardMeshes.get(selShardKey);
      if (!shardMesh) return;
      const sd = shardDataMap.get(shardMesh.uuid);
      if (!sd) return;

      // Retrieve new dimensions from the modified mesh
      const newW = Math.round(shardMesh.geometry.parameters.width / VIS_SCALE);
      const newH = Math.round(shardMesh.geometry.parameters.height / VIS_SCALE); // Three Y height
      const newD = Math.round(shardMesh.geometry.parameters.depth / VIS_SCALE);  // Three Z depth

      // Store in native Three.js coordinates
      sd.size.w = newW;
      sd.size.h = newH;
      sd.size.d = newD;

      sd.position.x = Math.round(shardMesh.position.x / VIS_SCALE - newW / 2);
      sd.position.y = Math.round(shardMesh.position.y / VIS_SCALE - newH / 2);
      sd.position.z = Math.round(shardMesh.position.z / VIS_SCALE - newD / 2);

      // Update and finalize socket configurations in placement data
      if (sd.sockets) {
        sd.sockets = [];
      }

      // Check if size or position changed and push to history
      if (this.initialShardState) {
        if (sd.size.w !== this.initialShardState.size.w ||
            sd.size.d !== this.initialShardState.size.d ||
            sd.size.h !== this.initialShardState.size.h ||
            sd.position.x !== this.initialShardState.position.x ||
            sd.position.y !== this.initialShardState.position.y ||
            sd.position.z !== this.initialShardState.position.z) {
          
          const undoState = this.initialShardState;
          const redoState = JSON.parse(JSON.stringify(sd));
          
          import('../../store/history_manager.js').then(({ historyManager }) => {
            historyManager.pushAction(
              'resize', 
              'shard', 
              selShardKey, 
              `Изменение размеров шарда ${selShardKey}`, 
              undoState, 
              redoState
            );
          });
        }
      }

      this.initialShardState = null;

      // Signal layout updates via delta events
      emit(EVENTS.SHARD_TRANSFORMED, {
        key: selShardKey,
        position: { x: sd.position.x, y: sd.position.y, z: sd.position.z },
        size: { w: sd.size.w, h: sd.size.h, d: sd.size.d }
      });
      emit(EVENTS.VALIDATION_REQ);

      // Re-spawn Handles at finalized boundary coordinates
      this.removeHandles();
      this.spawnHandles();
    }
  }

  onKeyDown(event) {
    if (event.key === 'Escape') {
      if (this.isDragging) {
        // Rollback current drag session changes
        const selShardKey = store.get('selectedShardKey');
        if (selShardKey) {
          const shardMesh = shardMeshes.get(selShardKey);
          if (shardMesh) {
            shardMesh.position.copy(this.initialPosition);
            this.updateMeshGeometry(shardMesh, this.initialW, this.initialD, this.initialH);
            this.updateAttachments(shardMesh, this.initialW, this.initialD, this.initialH, new THREE.Vector3(), 0);
          }
        }
        this.isDragging = false;
        this.activeHandle = null;
        controls.enabled = true;
        document.body.style.cursor = 'default';
        this.removeHandles();
        this.spawnHandles();
        return true;
      }
      deselectAll();
      return true;
    }
    return false;
  }

  updateMeshGeometry(shardMesh, w, d, h) {
    shardMesh.geometry.dispose();
    shardMesh.geometry = new THREE.BoxGeometry(w * VIS_SCALE, h * VIS_SCALE, d * VIS_SCALE);

    const mainWire = shardMesh.children.find(c => c.name === "main_wireframe");
    if (mainWire) {
      mainWire.geometry.dispose();
      mainWire.geometry = new THREE.EdgesGeometry(shardMesh.geometry);
    }
  }

  updateAttachments(shardMesh, w, d, h, localNormal, effectiveDelta) {
    // 1. Move the resize handles to match new boundaries (Three Y = height, Three Z = depth)
    shardMesh.traverse(child => {
      if (child.userData && child.userData.isResizeHandle) {
        const info = child.userData;
        if (info.handleName === 'PX') child.position.set((w * VIS_SCALE) / 2, 0, 0);
        if (info.handleName === 'NX') child.position.set(-(w * VIS_SCALE) / 2, 0, 0);
        if (info.handleName === 'PY') child.position.set(0, (h * VIS_SCALE) / 2, 0);
        if (info.handleName === 'NY') child.position.set(0, -(h * VIS_SCALE) / 2, 0);
        if (info.handleName === 'PZ') child.position.set(0, 0, (d * VIS_SCALE) / 2);
        if (info.handleName === 'NZ') child.position.set(0, 0, -(d * VIS_SCALE) / 2);
      }
    });

    // 2. Adjust Text Label sprite position (on top of the height axis Y)
    const label = shardMesh.children.find(c => c instanceof THREE.Sprite);
    if (label) {
      label.position.set(0, (h * VIS_SCALE) / 2 + 1.5, 0);
    }

    // 3. Scale and shift horizontal layers (along the Y height axis)
    const layerMeshes = [];
    shardMesh.traverse(child => {
      if (child.userData && child.userData.layerIndex !== undefined) {
        layerMeshes.push(child);
      }
    });
    layerMeshes.sort((a, b) => a.userData.layerIndex - b.userData.layerIndex);

    let currentY = -(h * VIS_SCALE) / 2;
    layerMeshes.forEach(layerMesh => {
      const layer_vis_h = (h * VIS_SCALE) * layerMesh.userData.height_pct;
      const yCenter = currentY + layer_vis_h / 2;

      layerMesh.position.set(0, yCenter, 0);
      layerMesh.scale.set(1.0, layer_vis_h, 1.0);

      // Re-create the horizontal layer geometry
      layerMesh.geometry.dispose();
      layerMesh.geometry = new THREE.BoxGeometry(w * VIS_SCALE, 1.0, d * VIS_SCALE);

      const wire = layerMesh.children.find(c => c.name === "wireframe");
      if (wire) {
        wire.geometry.dispose();
        wire.geometry = new THREE.EdgesGeometry(layerMesh.geometry);
      }

      currentY += layer_vis_h;
    });

    // 4. Update visual horizontal layer dividers positions and widths (along Y axis)
    const dividers = [];
    shardMesh.traverse(child => {
      if (child.userData && child.userData.isDivider) {
        dividers.push(child);
      }
    });
    dividers.sort((a, b) => a.userData.dividerIndex - b.userData.dividerIndex);

    let accumY = -(h * VIS_SCALE) / 2;
    dividers.forEach((divMesh, idx) => {
      accumY += (h * VIS_SCALE) * layerMeshes[idx].userData.height_pct;
      divMesh.position.set(0, accumY, 0);

      // Recreate plane geometry to match width/depth changes
      divMesh.geometry.dispose();
      divMesh.geometry = new THREE.PlaneGeometry((w * VIS_SCALE) * 1.02, (d * VIS_SCALE) * 1.02);

      const border = divMesh.children.find(c => c.name === "border");
      if (border) {
        const divW = w * VIS_SCALE;
        const divD = d * VIS_SCALE;
        border.geometry.dispose();
        border.geometry = new THREE.BufferGeometry().setFromPoints([
          new THREE.Vector3(-divW / 2, -divD / 2, 0),
          new THREE.Vector3(divW / 2, -divD / 2, 0),
          new THREE.Vector3(divW / 2, divD / 2, 0),
          new THREE.Vector3(-divW / 2, divD / 2, 0),
          new THREE.Vector3(-divW / 2, -divD / 2, 0)
        ]);
      }
    });
  }
}
