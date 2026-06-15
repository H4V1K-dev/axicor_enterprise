/**
 * @fileoverview add_socket_mode.js — Mode for placing new socket grids onto existing shards.
 */

import * as THREE from 'three';
import { camera, scene, controls } from '../../viewer.js';
import { shardMeshes, VIS_SCALE, shardDataMap, buildSceneData } from '../../scene_builder.js';
import { deselectAll, selectSocket } from '../selection.js';
import { store } from '../../store/store.js';
import { showToast } from '../../ui/toast.js';
import { modeManager, transformControls } from '../../editor.js';
import { initAdvancedObjPropPanel, destroyAdvancedObjPropPanel, socketPropConfig, updateAdvancedPanelHeight } from '../../ui/advanced_obj_prop.js';

export class AddSocketMode {
  constructor() {
    this.ghostGroup = null;
    this.pulsePhase = 0.0;
    this.isValidPosition = false;
    this.allowedShardKey = null;
    this.targetShardKey = null;
    this.currentFaceSign = 1;
    this.currentVoxelOffset = { x: 0, y: 0 };
    this.transitionTimeouts = [];
    this.preventContextMenu = null;

    this.rebuildGhostGeometry = this.rebuildGhostGeometry.bind(this);
  }

  enter() {
    const selectedShardKey = store.get('selectedShardKey');
    if (!selectedShardKey) {
      showToast("Сначала выберите шард для добавления сокета.", "warning");
      setTimeout(() => {
        modeManager.setMode('select');
      }, 0);
      return;
    }

    this.allowedShardKey = selectedShardKey;
    if (transformControls) {
      transformControls.detach();
    }
    showToast("Режим добавления сокета. Наведите на шард для предпросмотра.", "info");

    // Block right-click context menu
    this.preventContextMenu = (e) => e.preventDefault();
    window.addEventListener('contextmenu', this.preventContextMenu);

    // Initialize config panel for socket configuration
    initAdvancedObjPropPanel('socket');
    socketPropConfig.onChange = this.rebuildGhostGeometry;

    // Create primary ghost socket graphics
    this.rebuildGhostGeometry();

    // Morph the sidebar into cross
    this.transitionTimeouts = [];
    const toolsSidebar = document.getElementById('tools-sidebar');
    const toolsToggle = document.getElementById('tools-toggle-btn');
    const advancedPanel = document.getElementById('advanced-obj-prop');

    if (toolsSidebar && toolsToggle && advancedPanel) {
      this.originalMenuIcon = toolsToggle.innerHTML;
      this.wasSidebarClosed = toolsSidebar.classList.contains('closed');

      toolsSidebar.classList.add('closed');
      toolsSidebar.classList.add('folding');

      const t1 = setTimeout(() => {
        toolsSidebar.classList.add('morphed');
        toolsToggle.innerHTML = `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-x"><line x1="18" x2="6" y1="6" y2="18"/><line x1="6" x2="18" y1="6" y2="18"/></svg>`;

        const t2 = setTimeout(() => {
          advancedPanel.classList.add('open');
          updateAdvancedPanelHeight();

          const t3 = setTimeout(() => {
            updateAdvancedPanelHeight();
            toolsSidebar.classList.add('snapped');
          }, 350);
          this.transitionTimeouts.push(t3);
        }, 400);
        this.transitionTimeouts.push(t2);
      }, 300);
      this.transitionTimeouts.push(t1);
    }
  }

  exit() {
    if (this.transitionTimeouts) {
      this.transitionTimeouts.forEach(t => clearTimeout(t));
      this.transitionTimeouts = [];
    }

    if (this.preventContextMenu) {
      window.removeEventListener('contextmenu', this.preventContextMenu);
      this.preventContextMenu = null;
    }

    const toolsSidebar = document.getElementById('tools-sidebar');
    const toolsToggle = document.getElementById('tools-toggle-btn');
    const advancedPanel = document.getElementById('advanced-obj-prop');

    if (toolsSidebar) {
      toolsSidebar.classList.remove('snapped');
    }
    if (advancedPanel) {
      advancedPanel.classList.remove('open');
    }

    const tRestore = setTimeout(() => {
      if (toolsSidebar) {
        toolsSidebar.classList.remove('morphed');
        toolsSidebar.classList.remove('folding');
        if (!this.wasSidebarClosed) {
          toolsSidebar.classList.remove('closed');
        }
        toolsSidebar.style.setProperty('--advanced-panel-height', '0px');
      }
      if (toolsToggle && this.originalMenuIcon) {
        toolsToggle.innerHTML = this.originalMenuIcon;
      }
      destroyAdvancedObjPropPanel();
    }, 450);
    this.transitionTimeouts.push(tRestore);

    if (this.ghostGroup) {
      if (this.ghostGroup.parent) {
        this.ghostGroup.parent.remove(this.ghostGroup);
      } else {
        scene.remove(this.ghostGroup);
      }
      this.ghostGroup.traverse(child => {
        if (child.geometry) child.geometry.dispose();
        if (child.material) {
          if (Array.isArray(child.material)) {
            child.material.forEach(m => m.dispose());
          } else {
            child.material.dispose();
          }
        }
      });
      this.ghostGroup = null;
    }

    document.body.style.cursor = 'default';
  }

  rebuildGhostGeometry() {
    const isVisible = this.ghostGroup ? this.ghostGroup.visible : false;
    const parent = this.ghostGroup ? this.ghostGroup.parent : null;

    if (this.ghostGroup) {
      if (parent) {
        parent.remove(this.ghostGroup);
      } else {
        scene.remove(this.ghostGroup);
      }
      this.ghostGroup.traverse(child => {
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

    this.ghostGroup = new THREE.Group();
    this.ghostGroup.visible = isVisible;

    const sock = socketPropConfig;
    const spacing = VIS_SCALE * sock.pitch;
    const backingW = sock.width * spacing;
    const backingH = sock.height * spacing;

    // 1. Backing plane
    const backingGeo = new THREE.PlaneGeometry(backingW, backingH);
    const backingMat = new THREE.MeshStandardMaterial({
      color: 0x10b981,
      transparent: true,
      opacity: 0.5,
      roughness: 0.3,
      metalness: 0.1,
      side: THREE.DoubleSide,
      depthWrite: false
    });
    const backingMesh = new THREE.Mesh(backingGeo, backingMat);
    backingMesh.name = "ghost_backing";
    this.ghostGroup.add(backingMesh);

    // 2. Instanced Pins
    const pinGeo = new THREE.BoxGeometry(0.12 * VIS_SCALE, 0.12 * VIS_SCALE, 0.15 * VIS_SCALE);
    const pinMat = new THREE.MeshStandardMaterial({
      color: 0x10b981,
      transparent: true,
      opacity: 0.8,
      roughness: 0.15,
      metalness: 0.8,
      depthWrite: false
    });

    const count = sock.width * sock.height;
    const instancedMesh = new THREE.InstancedMesh(pinGeo, pinMat, count);
    const dummy = new THREE.Object3D();
    let idx = 0;
    for (let row = 0; row < sock.height; row++) {
      for (let col = 0; col < sock.width; col++) {
        const localX = (col - (sock.width - 1) / 2) * spacing;
        const localY = (row - (sock.height - 1) / 2) * spacing;
        dummy.position.set(localX, localY, 0.08 * VIS_SCALE);
        dummy.updateMatrix();
        instancedMesh.setMatrixAt(idx++, dummy.matrix);
      }
    }
    instancedMesh.instanceMatrix.needsUpdate = true;
    this.ghostGroup.add(instancedMesh);

    // Set local rotation
    this.ghostGroup.rotation.z = THREE.MathUtils.degToRad(sock.rotation);

    if (parent) {
      parent.add(this.ghostGroup);
    } else {
      scene.add(this.ghostGroup);
    }
  }

  onUpdate(dt) {
    if (!this.ghostGroup) return;

    this.pulsePhase += dt * 3.0;
    const opacityVal = 0.35 + Math.sin(this.pulsePhase) * 0.15;

    this.ghostGroup.traverse(child => {
      if (child.material) {
        if (child.name === "ghost_backing") {
          child.material.opacity = opacityVal;
        } else {
          child.material.opacity = opacityVal + 0.3;
        }
      }
    });
  }

  onPointerMove(event, raycaster) {
    if (!this.ghostGroup) return;

    if (!this.allowedShardKey) {
      this.ghostGroup.visible = false;
      document.body.style.cursor = 'not-allowed';
      this.isValidPosition = false;
      this.targetShardKey = null;
      return;
    }

    const targetMesh = shardMeshes[this.allowedShardKey];
    if (!targetMesh) {
      this.ghostGroup.visible = false;
      document.body.style.cursor = 'not-allowed';
      this.isValidPosition = false;
      this.targetShardKey = null;
      return;
    }

    const hits = raycaster.intersectObjects([targetMesh], true);
    if (hits.length === 0) {
      this.ghostGroup.visible = false;
      document.body.style.cursor = 'not-allowed';
      this.isValidPosition = false;
      this.targetShardKey = null;
      return;
    }

    // Find the primary shard mesh hit
    let shardMesh = hits[0].object;
    while (shardMesh && shardMesh !== targetMesh) {
      shardMesh = shardMesh.parent;
    }

    if (!shardMesh) {
      this.ghostGroup.visible = false;
      document.body.style.cursor = 'not-allowed';
      this.isValidPosition = false;
      this.targetShardKey = null;
      return;
    }

    const shardKey = this.allowedShardKey;
    const sd = shardDataMap[shardMesh.uuid];

    if (!shardKey || !sd) {
      this.ghostGroup.visible = false;
      document.body.style.cursor = 'not-allowed';
      this.isValidPosition = false;
      this.targetShardKey = null;
      return;
    }

    this.ghostGroup.visible = true;
    document.body.style.cursor = 'cell';
    this.isValidPosition = true;
    this.targetShardKey = shardKey;

    // Attach ghost group to selected shard mesh if not already
    if (this.ghostGroup.parent !== shardMesh) {
      shardMesh.add(this.ghostGroup);
    }

    // Convert world hit point to local coordinate system of the shard
    const localHit = new THREE.Vector3().copy(hits[0].point);
    shardMesh.worldToLocal(localHit);

    // Shard dimensions
    const shardW = sd.size.w * VIS_SCALE;
    const shardD = sd.size.d * VIS_SCALE;
    const shardH = sd.size.h * VIS_SCALE;

    // Determine target face
    let faceSign = 1;
    if (socketPropConfig.face === 'top') {
      faceSign = 1;
    } else if (socketPropConfig.face === 'bottom') {
      faceSign = -1;
    } else {
      // Auto-detect based on height of click relative to center of shard
      faceSign = localHit.z > 0 ? 1 : -1;
    }
    this.currentFaceSign = faceSign;

    // Socket dimensions in voxel units
    const pitch = socketPropConfig.pitch;
    const sockW_voxels = socketPropConfig.width * pitch;
    const sockH_voxels = socketPropConfig.height * pitch;

    // Compute bounding rectangle dimensions on the face depending on rotation
    const rotationDeg = socketPropConfig.rotation;
    const isRotated = (rotationDeg === 90 || rotationDeg === 270);
    const socketFaceW_voxels = isRotated ? sockH_voxels : sockW_voxels;
    const socketFaceD_voxels = isRotated ? sockW_voxels : sockH_voxels;

    // Calculate clamp boundaries in voxel units (ensures center remains strictly integer voxel)
    const maxVoxelX = Math.max(0, Math.floor((sd.size.w - socketFaceW_voxels) / 2));
    const minVoxelX = -maxVoxelX;
    const maxVoxelY = Math.max(0, Math.floor((sd.size.d - socketFaceD_voxels) / 2));
    const minVoxelY = -maxVoxelY;

    const editorSettings = store.get('editorSettings') || {};
    const snapStep = editorSettings.snap_step || 1;

    // Convert local hit to voxel coordinates and round to nearest snapStep
    const rawVoxelX = Math.round(localHit.x / (snapStep * VIS_SCALE)) * snapStep;
    const rawVoxelY = Math.round(localHit.y / (snapStep * VIS_SCALE)) * snapStep;

    // Apply clamping to integer voxel coordinates
    const clampedVoxelX = Math.max(minVoxelX, Math.min(maxVoxelX, rawVoxelX));
    const clampedVoxelY = Math.max(minVoxelY, Math.min(maxVoxelY, rawVoxelY));

    // Convert clamped voxel coordinates back to Three.js coordinates
    const clampedX = clampedVoxelX * VIS_SCALE;
    const clampedY = clampedVoxelY * VIS_SCALE;

    // Place the ghost relative to the shard center.
    // Local Z axis of Three.js shard mesh: +Z is top face, -Z is bottom face
    this.ghostGroup.position.set(clampedX, clampedY, faceSign * (shardH / 2 + 0.01));

    // Store voxel coordinates for placement
    this.currentVoxelOffset = {
      x: clampedVoxelX,
      y: clampedVoxelY
    };
  }

  onPointerDown(event, raycaster) {
    if (event.button === 2) {
      // Right Click Cancel
      modeManager.popMode();
      return true;
    }

    if (event.button !== 0) return false;
    if (!this.ghostGroup || !this.ghostGroup.visible || !this.isValidPosition || !this.targetShardKey) {
      return false;
    }

    const placementData = store.get('placementData');
    if (!placementData) return false;

    const shard = placementData.shards.find(s => s.key === this.targetShardKey);
    if (!shard) return false;

    if (!shard.sockets) {
      shard.sockets = [];
    }

    // Auto-generate name
    const desiredName = socketPropConfig.name.trim();
    let finalName = desiredName || 'sock';
    let index = 0;
    const checkExists = (n) => shard.sockets.some(s => s.name === n);

    if (checkExists(finalName)) {
      finalName = `${desiredName}_${index}`;
      while (checkExists(finalName)) {
        index++;
        finalName = `${desiredName}_${index}`;
      }
    }

    const socketKey = `${shard.key}.${finalName}`;

    // Create socket object
    const newSocket = {
      name: finalName,
      width: socketPropConfig.width,
      height: socketPropConfig.height,
      pitch: socketPropConfig.pitch,
      rotation: socketPropConfig.rotation,
      faceSign: this.currentFaceSign,
      offset: {
        x: this.currentVoxelOffset.x,
        y: this.currentVoxelOffset.y
      }
    };

    shard.sockets.push(newSocket);
    store.set('placementData', placementData);
    store.set('hasUnsavedChanges', true);

    // Push action to history
    import('../../store/history_manager.js').then(({ historyManager }) => {
      historyManager.pushAction('create', 'socket', socketKey, `Создание сокета ${finalName} на шарде ${shard.shard}`, null, newSocket);
    });

    // Rebuild visual scene data
    buildSceneData(placementData, true);

    // Auto-select socket
    selectSocket(socketKey);

    // Transition to translate mode
    modeManager.setMode('translate');

    showToast(`Сокет ${finalName} успешно создан!`, "success");
    return true;
  }

  onPointerUp(event, raycaster) { }

  onKeyDown(event) {
    if (event.key === 'Escape') {
      modeManager.popMode();
      return true;
    }
    return false;
  }
}
