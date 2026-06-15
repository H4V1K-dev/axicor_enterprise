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
      // Fallback to select mode
      setTimeout(() => {
        modeManager.setMode('select');
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
      modeManager.setMode('select');
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
    const selShardKey = store.get('selectedShardKey');
    if (selShardKey) {
      updateFocusVisuals();
      return;
    }
    this.dimAll();
  }

  dimAll() {
    // Dim all shards except the active one (which should be highlighted)
    const selShardKey = store.get('selectedShardKey');
    for (const [key, mesh] of Object.entries(shardMeshes)) {
      if (key === selShardKey) {
        mesh.material.opacity = 0.8;
        mesh.material.transparent = true;
        mesh.material.needsUpdate = true;
        continue;
      }
      mesh.material.opacity = 0.15;
      mesh.material.transparent = true;
      mesh.material.needsUpdate = true;

      const mainWire = mesh.children.find(c => c.name === "main_wireframe");
      if (mainWire) {
        mainWire.material.opacity = 0.15;
        mainWire.material.needsUpdate = true;
      }
    }

    // Dim all sockets
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

  spawnHandles() {
    const selShardKey = store.get('selectedShardKey');
    if (!selShardKey) return;
    const shardMesh = shardMeshes[selShardKey];
    if (!shardMesh) return;

    const w = shardMesh.geometry.parameters.width;
    const d = shardMesh.geometry.parameters.height;
    const h = shardMesh.geometry.parameters.depth;

    const handleGeo = new THREE.BoxGeometry(1.2 * VIS_SCALE, 1.2 * VIS_SCALE, 1.2 * VIS_SCALE);
    
    // Vibrant cyan/teal glowing material
    const handleMat = new THREE.MeshStandardMaterial({
      color: 0x00ffcc,
      emissive: 0x003322,
      roughness: 0.1,
      metalness: 0.8,
      transparent: true,
      opacity: 0.85,
      depthTest: false,
      depthWrite: false
    });

    const normals = [
      { name: 'PX', normal: new THREE.Vector3(1, 0, 0), axis: 'x', pos: new THREE.Vector3(w / 2, 0, 0) },
      { name: 'NX', normal: new THREE.Vector3(-1, 0, 0), axis: 'x', pos: new THREE.Vector3(-w / 2, 0, 0) },
      { name: 'PY', normal: new THREE.Vector3(0, 1, 0), axis: 'y', pos: new THREE.Vector3(0, d / 2, 0) },
      { name: 'NY', normal: new THREE.Vector3(0, -1, 0), axis: 'y', pos: new THREE.Vector3(0, -d / 2, 0) },
      { name: 'PZ', normal: new THREE.Vector3(0, 0, 1), axis: 'z', pos: new THREE.Vector3(0, 0, h / 2) },
      { name: 'NZ', normal: new THREE.Vector3(0, 0, -1), axis: 'z', pos: new THREE.Vector3(0, 0, -h / 2) }
    ];

    normals.forEach(info => {
      const mesh = new THREE.Mesh(handleGeo, handleMat.clone());
      mesh.position.copy(info.pos);
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
    for (const mesh of Object.values(shardMeshes)) {
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
    const shardMesh = shardMeshes[selShardKey];
    if (!shardMesh) return Infinity;
    const sd = shardDataMap[shardMesh.uuid];
    if (!sd) return Infinity;

    const parentNormal = localNormal.clone().applyQuaternion(shardMesh.quaternion).normalize();
    const pnx = Math.round(parentNormal.x);
    const pny = Math.round(parentNormal.y);
    const pnz = Math.round(parentNormal.z);

    // If resizing vertically (Parent Y), horizontal plane collision limit doesn't apply
    if (pny !== 0) {
      return Infinity;
    }

    const initialW = sd.size.w;
    const initialD = sd.size.d;

    const posX = shardMesh.position.x / VIS_SCALE;
    const posZ = shardMesh.position.z / VIS_SCALE;

    const minX = posX - initialW / 2;
    const maxX = posX + initialW / 2;
    const minZ = posZ - initialD / 2;
    const maxZ = posZ + initialD / 2;

    let minDist = Infinity;
    const currentOrbit = sd.orbit;

    for (const [key, mesh] of Object.entries(shardMeshes)) {
      if (key === selShardKey) continue;
      const otherData = shardDataMap[mesh.uuid];
      if (!otherData || otherData.orbit !== currentOrbit) continue;

      const otherW = otherData.size.w;
      const otherD = otherData.size.d;

      const otherX = mesh.position.x / VIS_SCALE;
      const otherZ = mesh.position.z / VIS_SCALE;

      const otherMinX = otherX - otherW / 2;
      const otherMaxX = otherX + otherW / 2;
      const otherMinZ = otherZ - otherD / 2;
      const otherMaxZ = otherZ + otherD / 2;

      // Check spatial overlaps and calculate distances based on parent normal directions
      if (pnx > 0) { // Right (Parent X positive)
        const overlapZ = minZ < otherMaxZ && maxZ > otherMinZ;
        if (overlapZ && otherMinX >= maxX) {
          const dist = otherMinX - maxX;
          if (dist < minDist) minDist = dist;
        }
      } else if (pnx < 0) { // Left (Parent X negative)
        const overlapZ = minZ < otherMaxZ && maxZ > otherMinZ;
        if (overlapZ && otherMaxX <= minX) {
          const dist = minX - otherMaxX;
          if (dist < minDist) minDist = dist;
        }
      } else if (pnz > 0) { // Forward/depth (Parent Z positive)
        const overlapX = minX < otherMaxX && maxX > otherMinX;
        if (overlapX && otherMinZ >= maxZ) {
          const dist = otherMinZ - maxZ;
          if (dist < minDist) minDist = dist;
        }
      } else if (pnz < 0) { // Backward/depth (Parent Z negative)
        const overlapX = minX < otherMaxX && maxX > otherMinX;
        if (overlapX && otherMaxZ <= minZ) {
          const dist = minZ - otherMaxZ;
          if (dist < minDist) minDist = dist;
        }
      }
    }

    if (minDist !== Infinity) {
      // Round down to multiples of 10
      const steps = Math.floor(minDist / 10);
      const safeDist = steps * 10;
      if (pnx !== 0) {
        return initialW + safeDist;
      } else if (pnz !== 0) {
        return initialD + safeDist;
      }
    }

    return Infinity;
  }

  onUpdate(dt) {
    const selShardKey = store.get('selectedShardKey');
    if (!selShardKey) return;
    const shardMesh = shardMeshes[selShardKey];
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
      const shardMesh = shardMeshes[selShardKey];
      if (!shardMesh) return false;
      const sd = shardDataMap[shardMesh.uuid];
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
            const socketGroup = socketMeshes[socketKey];
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
      const handlesList = [];
      for (const group of Object.values(socketMeshes)) {
        group.traverse(child => {
          if (child.name && child.name.startsWith("handle_") && child.visible) {
            handlesList.push(child);
          }
        });
      }

      const hoverHits = raycaster.intersectObjects(handlesList);
      if (hoverHits.length > 0) {
        const name = hoverHits[0].object.name;
        if (name === 'handle_L' || name === 'handle_R') {
          document.body.style.cursor = 'col-resize';
        } else if (name === 'handle_T' || name === 'handle_B') {
          document.body.style.cursor = 'row-resize';
        } else if (name === 'handle_TR' || name === 'handle_BL') {
          document.body.style.cursor = 'nesw-resize';
        } else if (name === 'handle_TL' || name === 'handle_BR') {
          document.body.style.cursor = 'nwse-resize';
        }
      } else {
        document.body.style.cursor = 'default';
      }
      return;
    }

    if (selShardKey) {
      const shardMesh = shardMeshes[selShardKey];
      if (!shardMesh) return;
      const sd = shardDataMap[shardMesh.uuid];
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
        if (hits.length > 0) {
          const hoverH = hits[0].object;
          const axis = hoverH.userData.axis;
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

      // Snap delta to step increment from settings
      const editorSettings = store.get('editorSettings') || {};
      const RESIZE_STEP = editorSettings.resize_step || 10;
      const steppedDelta = Math.round(deltaVoxels / RESIZE_STEP) * RESIZE_STEP;

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
        if (axis === 'y') tempD += testDelta;
        if (axis === 'z') tempH += testDelta;

        // Clamp sizes
        let clampedW = tempW;
        let clampedD = tempD;
        let clampedH = tempH;

        if (axis === 'x') {
          if (clampedW < MIN_SHARD_SIZE) clampedW = MIN_SHARD_SIZE;
          clampedW = Math.min(MAX_SHARD_SIZE_XY, clampedW);
          if (testDelta > 0) {
            clampedW = Math.min(this.dynamicLimit, clampedW);
          }
        }
        if (axis === 'y') {
          if (clampedD < MIN_SHARD_SIZE) clampedD = MIN_SHARD_SIZE;
          clampedD = Math.min(MAX_SHARD_SIZE_XY, clampedD);
          if (testDelta > 0) {
            clampedD = Math.min(this.dynamicLimit, clampedD);
          }
        }
        if (axis === 'z') {
          if (clampedH < MIN_SHARD_SIZE) clampedH = MIN_SHARD_SIZE;
          clampedH = Math.min(MAX_SHARD_SIZE_Z, clampedH);
        }

        // Effective delta based on clamping
        let effectiveDelta = 0;
        if (axis === 'x') effectiveDelta = clampedW - this.initialW;
        if (axis === 'y') effectiveDelta = clampedD - this.initialD;
        if (axis === 'z') effectiveDelta = clampedH - this.initialH;

        const localShift = localNormal.clone()
          .multiplyScalar(effectiveDelta * VIS_SCALE / 2)
          .applyQuaternion(shardMesh.quaternion);
        
        const tempPosition = this.initialPosition.clone().add(localShift);
        const newSize2D = { w: clampedW, d: clampedD };

        // Shrinking (testDelta < 0) is always allowed as it resolves overlaps
        if (testDelta > 0 && checkShardCollision(selShardKey, tempPosition, newSize2D)) {
          // Collision detected at this step, stop at previous valid step
          break;
        }

        // Vertical floor and ceiling constraints check when growing along Z axis (Parent Y height)
        if (axis === 'z' && testDelta > 0) {
          const tempPosLocalY = tempPosition.y / VIS_SCALE;
          const tempBottomY = tempPosLocalY - clampedH / 2;
          const tempTopY = tempPosLocalY + clampedH / 2;

          const orbits = store.get('placementData').orbits;
          const currentOrbitIdx = sd.orbit;
          const nextOrbit = orbits
            .filter(o => o.index > currentOrbitIdx)
            .sort((a, b) => a.index - b.index)[0];
          
          const currentOrbit = orbits.find(o => o.index === currentOrbitIdx);
          const currentRadius = currentOrbit ? currentOrbit.radius : 0;
          const nextRadius = nextOrbit ? nextOrbit.radius : Infinity;
          const ceilingY = nextRadius - currentRadius;

          if (tempBottomY < -0.01 || tempTopY > ceilingY + 0.01) {
            break; // Stop growing vertically if violating floor or ceiling bounds
          }
        }
        finalDelta = testDelta;
      }

      // Calculate final sizes based on maximum valid delta found
      let finalW = this.initialW;
      let finalD = this.initialD;
      let finalH = this.initialH;

      if (axis === 'x') finalW += finalDelta;
      if (axis === 'y') finalD += finalDelta;
      if (axis === 'z') finalH += finalDelta;

      // Final bounds enforcement and warnings
      if (axis === 'x') {
        if (finalW < MIN_SHARD_SIZE) {
          if (finalW <= 9) showToast("Размер шарда не может быть меньше 10 вокселей", "warning");
          finalW = MIN_SHARD_SIZE;
        }
        finalW = Math.min(MAX_SHARD_SIZE_XY, finalW);
        if (finalDelta > 0) {
          finalW = Math.min(this.dynamicLimit, finalW);
        }
      }
      if (axis === 'y') {
        if (finalD < MIN_SHARD_SIZE) {
          if (finalD <= 9) showToast("Размер шарда не может быть меньше 10 вокселей", "warning");
          finalD = MIN_SHARD_SIZE;
        }
        finalD = Math.min(MAX_SHARD_SIZE_XY, finalD);
        if (finalDelta > 0) {
          finalD = Math.min(this.dynamicLimit, finalD);
        }
      }
      if (axis === 'z') {
        if (finalH < MIN_SHARD_SIZE) {
          if (finalH <= 9) showToast("Размер шарда не может быть меньше 10 вокселей", "warning");
          finalH = MIN_SHARD_SIZE;
        }
        finalH = Math.min(MAX_SHARD_SIZE_Z, finalH);
      }

      let finalEffectiveDelta = 0;
      if (axis === 'x') finalEffectiveDelta = finalW - this.initialW;
      if (axis === 'y') finalEffectiveDelta = finalD - this.initialD;
      if (axis === 'z') finalEffectiveDelta = finalH - this.initialH;

      const localShift = localNormal.clone()
        .multiplyScalar(finalEffectiveDelta * VIS_SCALE / 2)
        .applyQuaternion(shardMesh.quaternion);
      
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
      const shardMesh = shardMeshes[selShardKey];
      if (!shardMesh) return;
      const sd = shardDataMap[shardMesh.uuid];
      if (!sd) return;

      // Retrieve new dimensions from the modified mesh
      const newW = Math.round(shardMesh.geometry.parameters.width / VIS_SCALE);
      const newD = Math.round(shardMesh.geometry.parameters.height / VIS_SCALE);
      const newH = Math.round(shardMesh.geometry.parameters.depth / VIS_SCALE);

      // Convert local coordinates back to placement space
      // JSON X = Three X
      // JSON Y = Three Y + radius (actually, y relative to plane + radius)
      // JSON Z = Three Z? Wait, let's review:
      // In scene_builder:
      // x = sd.position.x * VIS_SCALE
      // y = (sd.position.y - radius) * VIS_SCALE -> so sd.position.y = (y / VIS_SCALE) + radius
      // z = sd.position.z * VIS_SCALE -> so sd.position.z = z / VIS_SCALE
      const orb = store.get('placementData').orbits.find(o => o.index === sd.orbit);
      const radius = orb ? orb.radius : 0.0;

      sd.size.w = newW;
      sd.size.d = newD;
      sd.size.h = newH;

      sd.position.x = Math.round(shardMesh.position.x / VIS_SCALE);
      sd.position.y = Math.round((shardMesh.position.y / VIS_SCALE) + radius);
      sd.position.z = Math.round(shardMesh.position.z / VIS_SCALE);

      // Update and finalize socket configurations in placement data
      if (sd.sockets) {
        sd.sockets.forEach(sock => {
          const socketKey = `${sd.key}.${sock.name}`;
          const socketGroup = socketMeshes[socketKey];
          if (socketGroup) {
            sock.offset = {
              x: Math.round(socketGroup.position.x / VIS_SCALE),
              y: Math.round(socketGroup.position.y / VIS_SCALE),
              z: socketGroup.userData.originalOffset && socketGroup.userData.originalOffset.z !== undefined 
                ? Number(socketGroup.userData.originalOffset.z.toFixed(2)) 
                : Number((socketGroup.position.z / VIS_SCALE).toFixed(2))
            };
            
            // Rebuild the socket completely to update handles bounds
            rebuildSocket(
              sd.key,
              sock.name,
              sock.width,
              sock.height,
              sock.pitch || 1,
              sock.offset,
              socketGroup.userData.faceSign,
              sock.rotation || 0
            );
          }
        });
      }

      // Redraw connection routing line graphics
      const routes = store.get('routesData');
      if (routes) {
        drawRoutes(routes);
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

      // Signal layout updates and validate connections
      emit(EVENTS.LAYOUT_CHANGED, sd);
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
          const shardMesh = shardMeshes[selShardKey];
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
    shardMesh.geometry = new THREE.BoxGeometry(w * VIS_SCALE, d * VIS_SCALE, h * VIS_SCALE);

    const mainWire = shardMesh.children.find(c => c.name === "main_wireframe");
    if (mainWire) {
      mainWire.geometry.dispose();
      mainWire.geometry = new THREE.EdgesGeometry(shardMesh.geometry);
    }
  }

  updateAttachments(shardMesh, w, d, h, localNormal, effectiveDelta) {
    // 1. Move the resize handles to match new boundaries
    shardMesh.traverse(child => {
      if (child.userData && child.userData.isResizeHandle) {
        const info = child.userData;
        if (info.handleName === 'PX') child.position.set((w * VIS_SCALE) / 2, 0, 0);
        if (info.handleName === 'NX') child.position.set(-(w * VIS_SCALE) / 2, 0, 0);
        if (info.handleName === 'PY') child.position.set(0, (d * VIS_SCALE) / 2, 0);
        if (info.handleName === 'NY') child.position.set(0, -(d * VIS_SCALE) / 2, 0);
        if (info.handleName === 'PZ') child.position.set(0, 0, (h * VIS_SCALE) / 2);
        if (info.handleName === 'NZ') child.position.set(0, 0, -(h * VIS_SCALE) / 2);
      }
    });

    // 2. Adjust Text Label sprite position
    const label = shardMesh.children.find(c => c instanceof THREE.Sprite);
    if (label) {
      label.position.set(0, 0, (h * VIS_SCALE) / 2 + 1.5);
    }

    // 3. Scale and shift horizontal layers
    const layerMeshes = [];
    shardMesh.traverse(child => {
      if (child.userData && child.userData.layerIndex !== undefined) {
        layerMeshes.push(child);
      }
    });
    layerMeshes.sort((a, b) => a.userData.layerIndex - b.userData.layerIndex);

    let currentZ = -(h * VIS_SCALE) / 2;
    layerMeshes.forEach(layerMesh => {
      const layer_vis_h = (h * VIS_SCALE) * layerMesh.userData.height_pct;
      const zCenter = currentZ + layer_vis_h / 2;

      layerMesh.position.set(0, 0, zCenter);
      layerMesh.scale.set(1.0, 1.0, layer_vis_h);

      // Re-create the horizontal layer geometry to accommodate width/depth changes
      layerMesh.geometry.dispose();
      layerMesh.geometry = new THREE.BoxGeometry(w * VIS_SCALE, d * VIS_SCALE, 1.0);

      const wire = layerMesh.children.find(c => c.name === "wireframe");
      if (wire) {
        wire.geometry.dispose();
        wire.geometry = new THREE.EdgesGeometry(layerMesh.geometry);
      }

      currentZ += layer_vis_h;
    });

    // 4. Update visual horizontal layer dividers positions and widths
    const dividers = [];
    shardMesh.traverse(child => {
      if (child.userData && child.userData.isDivider) {
        dividers.push(child);
      }
    });
    dividers.sort((a, b) => a.userData.dividerIndex - b.userData.dividerIndex);

    let accumZ = -(h * VIS_SCALE) / 2;
    dividers.forEach((divMesh, idx) => {
      accumZ += (h * VIS_SCALE) * layerMeshes[idx].userData.height_pct;
      divMesh.position.set(0, 0, accumZ);

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

    // 5. Update socket attachments positions
    const sd = shardDataMap[shardMesh.uuid];
    if (sd && sd.sockets) {
      sd.sockets.forEach(sock => {
        const socketKey = `${sd.key}.${sock.name}`;
        const socketGroup = socketMeshes[socketKey];
        if (socketGroup) {
          const faceSign = socketGroup.userData.faceSign;
          
          // Compute new offsets to lock their absolute world coordinates during horizontal scaling
          let newOffset = { ...this.initialSocketOffsets[socketKey] };

          if (localNormal.x !== 0) {
            newOffset.x -= localNormal.x * effectiveDelta / 2;
          }
          if (localNormal.y !== 0) {
            newOffset.y -= localNormal.y * effectiveDelta / 2;
          }

          let oz = faceSign * ((h * VIS_SCALE) / 2 + 0.01);
          if (newOffset.z !== undefined) {
            const scaleZ = h / this.initialH;
            const newOffsetZ = newOffset.z * scaleZ;
            oz = newOffsetZ * VIS_SCALE;
            if (socketGroup.userData.originalOffset) {
              socketGroup.userData.originalOffset.z = newOffsetZ;
            }
          }

          // Move the group to the new position
          socketGroup.position.set(
            newOffset.x * VIS_SCALE,
            newOffset.y * VIS_SCALE,
            oz
          );
        }
      });
    }
  }
}
