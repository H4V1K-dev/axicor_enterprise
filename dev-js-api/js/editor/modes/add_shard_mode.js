/**
 * @fileoverview add_shard_mode.js — Mode for adding new shards to the visualizer scene.
 */

import * as THREE from 'three';
import { camera, scene, controls } from '../../viewer.js';
import { shardMeshes, VIS_SCALE, shardDataMap, buildSceneData } from '../../scene_builder.js';
import { deselectAll, selectShard } from '../selection.js';
import { store } from '../../store/store.js';
import { showToast } from '../../ui/toast.js';
import { emit, EVENTS } from '../../store/event_bus.js';
import { modeManager, checkShardCollision } from '../../editor.js';
import { initAdvancedObjPropPanel, destroyAdvancedObjPropPanel, advancedObjPropConfig, updateAdvancedPanelHeight } from '../../ui/advanced_obj_prop.js';

export class AddShardMode {
  constructor() {
    this.ghostMesh = null;
    this.ghostWire = null;
    this.pulsePhase = 0.0;
    this.isValidPosition = false;
    this.currentOrbitIndex = null;
    this.transitionTimeouts = [];
    this.preventContextMenu = null;

    this.rebuildGhostGeometry = this.rebuildGhostGeometry.bind(this);
  }

  enter() {
    deselectAll();
    showToast("Режим добавления шарда активен. Кликните на уровень для размещения.", "info");

    // Block browser's right-click context menu
    this.preventContextMenu = (e) => e.preventDefault();
    window.addEventListener('contextmenu', this.preventContextMenu);

    // Initialize config panel and properties
    initAdvancedObjPropPanel();
    advancedObjPropConfig.onChange = this.rebuildGhostGeometry;

    // Create ghost mesh materials (pulsing transparent overlay)
    const ghostMat = new THREE.MeshStandardMaterial({
      color: 0x10b981,
      transparent: true,
      opacity: 0.5,
      roughness: 0.2,
      metalness: 0.1,
      depthWrite: false
    });

    const wireMat = new THREE.LineBasicMaterial({
      color: 0x10b981,
      transparent: true,
      opacity: 0.8
    });

    const geo = new THREE.BoxGeometry(
      advancedObjPropConfig.w * VIS_SCALE,
      advancedObjPropConfig.h * VIS_SCALE, // height
      advancedObjPropConfig.d * VIS_SCALE  // depth
    );

    this.ghostMesh = new THREE.Mesh(geo, ghostMat);
    this.ghostMesh.visible = false;

    const edges = new THREE.EdgesGeometry(geo);
    this.ghostWire = new THREE.LineSegments(edges, wireMat);
    this.ghostWire.name = "ghost_wireframe";
    this.ghostMesh.add(this.ghostWire);

    scene.add(this.ghostMesh);

    // Execute the morphing sequence for the tools sidebar
    this.transitionTimeouts = [];

    const toolsSidebar = document.getElementById('tools-sidebar');
    const toolsToggle = document.getElementById('tools-toggle-btn');
    const advancedPanel = document.getElementById('advanced-obj-prop');

    if (toolsSidebar && toolsToggle && advancedPanel) {
      // Save original toolbar properties
      this.originalMenuIcon = toolsToggle.innerHTML;
      this.wasSidebarClosed = toolsSidebar.classList.contains('closed');

      // 1. Fold tools sidebar: add 'closed' and 'folding' classes.
      // This will trigger the sequential button fade-out without shrinking the container yet.
      toolsSidebar.classList.add('closed');
      toolsSidebar.classList.add('folding');

      // 2. Wait for the button folding animation to start (300ms), then morph container to a square
      const t1 = setTimeout(() => {
        toolsSidebar.classList.add('morphed');
        // Swap icon to cross
        toolsToggle.innerHTML = `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-x"><line x1="18" x2="6" y1="6" y2="18"/><line x1="6" x2="18" y1="6" y2="18"/></svg>`;

        // 3. Wait for the morphed container shrink transition (400ms) to complete, then open the config panel
        const t2 = setTimeout(() => {
          advancedPanel.classList.add('open');

          // Pre-set target height immediately so transition bottom target is calculated correctly
          updateAdvancedPanelHeight();

          // 4. Once panel is open (takes 300ms), wait an additional 50ms (total 350ms) and snap morphed sidebar
          const t3 = setTimeout(() => {
            updateAdvancedPanelHeight(); // re-measure just in case
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
    // Clear pending timeouts
    if (this.transitionTimeouts) {
      this.transitionTimeouts.forEach(t => clearTimeout(t));
      this.transitionTimeouts = [];
    }

    // Unblock right-click context menu
    if (this.preventContextMenu) {
      window.removeEventListener('contextmenu', this.preventContextMenu);
      this.preventContextMenu = null;
    }

    const toolsSidebar = document.getElementById('tools-sidebar');
    const toolsToggle = document.getElementById('tools-toggle-btn');
    const advancedPanel = document.getElementById('advanced-obj-prop');

    // 1. Instantly lift the morphed sidebar back to the center of left edge and close advancedPanel
    if (toolsSidebar) {
      toolsSidebar.classList.remove('snapped');
    }
    if (advancedPanel) {
      advancedPanel.classList.remove('open');
    }

    // 2. Wait 0.45s (window fully closed in 0.3s + 0.15s delay), then unfold sidebar back to original state
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

    if (this.ghostMesh) {
      if (this.ghostMesh.parent) {
        this.ghostMesh.parent.remove(this.ghostMesh);
      } else {
        scene.remove(this.ghostMesh);
      }
      if (this.ghostMesh.geometry) this.ghostMesh.geometry.dispose();
      if (this.ghostMesh.material) this.ghostMesh.material.dispose();

      if (this.ghostWire) {
        if (this.ghostWire.geometry) this.ghostWire.geometry.dispose();
        if (this.ghostWire.material) this.ghostWire.material.dispose();
      }
      this.ghostMesh = null;
      this.ghostWire = null;
    }

    document.body.style.cursor = 'default';
  }

  rebuildGhostGeometry() {
    if (!this.ghostMesh) return;

    // Dispose old geometries
    this.ghostMesh.geometry.dispose();
    this.ghostWire.geometry.dispose();

    // Rebuild BoxGeometry
    this.ghostMesh.geometry = new THREE.BoxGeometry(
      advancedObjPropConfig.w * VIS_SCALE,
      advancedObjPropConfig.h * VIS_SCALE, // height
      advancedObjPropConfig.d * VIS_SCALE  // depth
    );

    // Rebuild EdgesGeometry
    this.ghostWire.geometry = new THREE.EdgesGeometry(this.ghostMesh.geometry);

    // Position adjustments (bottom of ghost lies flush on floor)
    this.ghostMesh.position.y = (advancedObjPropConfig.h * VIS_SCALE) / 2;
  }

  onUpdate(dt) {
    if (!this.ghostMesh) return;

    // Opacity pulsing animation
    this.pulsePhase += dt * 3.0;
    const opacityVal = 0.35 + Math.sin(this.pulsePhase) * 0.15;
    this.ghostMesh.material.opacity = opacityVal;
    this.ghostWire.material.opacity = opacityVal + 0.3;
  }

  onPointerMove(event, raycaster) {
    if (!this.ghostMesh) return;

    // Raycast against a virtual horizontal ground plane (Y=0)
    const groundPlane = new THREE.Plane(new THREE.Vector3(0, 1, 0), 0);
    const hitPoint = new THREE.Vector3();
    const hitSuccess = raycaster.ray.intersectPlane(groundPlane, hitPoint);

    if (!hitSuccess) {
      this.ghostMesh.visible = false;
      document.body.style.cursor = 'not-allowed';
      this.isValidPosition = false;
      return;
    }

    this.ghostMesh.visible = true;
    document.body.style.cursor = 'cell';
    this.currentOrbitIndex = 0;

    // Attach ghost directly to scene
    if (this.ghostMesh.parent !== scene) {
      scene.add(this.ghostMesh);
    }

    const localPoint = new THREE.Vector3().copy(hitPoint);

    // Grid details
    const gW = advancedObjPropConfig.w;
    const gD = advancedObjPropConfig.d;
    const gH = advancedObjPropConfig.h;
    
    // Calculate local voxels target as AABB min of the ghostMesh
    const localVoxX = localPoint.x / VIS_SCALE - gW / 2;
    const localVoxZ = localPoint.z / VIS_SCALE - gD / 2;

    // Snap to neighbor math
    const snapThreshold = 20.0; // snap trigger distance in voxels
    let bestSnapX = null;
    let bestSnapZ = null;
    let minSnapDistX = snapThreshold;
    let minSnapDistZ = snapThreshold;

    for (const [key, mesh] of Object.entries(shardMeshes)) {
      const sd = shardDataMap[mesh.uuid];
      if (!sd) continue;

      // Other mesh bounds in AABB min coordinates
      const otherMinX = sd.position.x;
      const otherMaxX = sd.position.x + sd.size.w;
      const otherMinZ = sd.position.y; // Rust Y (depth)
      const otherMaxZ = sd.position.y + sd.size.d;

      // check boundaries alignment distances
      const dist1 = Math.abs(localVoxX - otherMinX);
      if (dist1 < minSnapDistX) {
        minSnapDistX = dist1;
        bestSnapX = otherMinX;
      }
      const dist2 = Math.abs(localVoxX - otherMaxX);
      if (dist2 < minSnapDistX) {
        minSnapDistX = dist2;
        bestSnapX = otherMaxX;
      }
      const dist3 = Math.abs(localVoxZ - otherMinZ);
      if (dist3 < minSnapDistZ) {
        minSnapDistZ = dist3;
        bestSnapZ = otherMinZ;
      }
      const dist4 = Math.abs(localVoxZ - otherMaxZ);
      if (dist4 < minSnapDistZ) {
        minSnapDistZ = dist4;
        bestSnapZ = otherMaxZ;
      }
    }

    const editorSettings = store.get('editorSettings') || {};
    const snapStep = editorSettings.snap_step || 1;

    let finalVoxX, finalVoxZ;
    if (bestSnapX !== null) {
      finalVoxX = bestSnapX;
    } else {
      finalVoxX = Math.round(localVoxX / snapStep) * snapStep;
    }

    if (bestSnapZ !== null) {
      finalVoxZ = bestSnapZ;
    } else {
      finalVoxZ = Math.round(localVoxZ / snapStep) * snapStep;
    }

    // Set position local to level floor (Three.js center coordinates)
    this.ghostMesh.position.set(
      (finalVoxX + gW / 2) * VIS_SCALE,
      (gH * VIS_SCALE) / 2, // flush with floor plane
      (finalVoxZ + gD / 2) * VIS_SCALE
    );

    // Collision checking: retrieve absolute position to feed the collision_adapter
    const absoluteWorldPos = new THREE.Vector3();
    this.ghostMesh.getWorldPosition(absoluteWorldPos);

    const isColliding = checkShardCollision(
      null,
      absoluteWorldPos,
      { w: gW, d: gD, h: gH, orbit: orbitIndex }
    );

    if (isColliding) {
      this.ghostMesh.material.color.setHex(0xf59e0b); // yellow
      this.ghostWire.material.color.setHex(0xf59e0b);
      this.isValidPosition = false;
    } else {
      this.ghostMesh.material.color.setHex(0x10b981); // green
      this.ghostWire.material.color.setHex(0x10b981);
      this.isValidPosition = true;
    }
  }

  onPointerDown(event, raycaster) {
    // Right Click Cancel (ПКМ)
    if (event.button === 2) {
      modeManager.popMode();
      return true;
    }

    if (event.button !== 0) return false;
    if (!this.ghostMesh || !this.ghostMesh.visible) return false;

    if (!this.isValidPosition) {
      showToast("Невозможно создать шард в этой позиции (коллизия с другими шардами)", "warning");
      return true;
    }

    // Capture positioning variables
    const placementData = store.get('placementData');
    if (!placementData) return false;

    const orbitIndex = 0;

    const deptName = advancedObjPropConfig.dept;
    const orbitLabel = 'l0';
    const deptClean = deptName.toLowerCase().replace(/[^a-z0-9]/g, '_');

    let index = 0;
    const checkExists = (k) => placementData.shards.some(s => s.key === k);
    let finalShardName = `${orbitLabel}_${deptClean}_${index}`;
    let finalKey = `${deptName}.${finalShardName}`;

    while (checkExists(finalKey)) {
      index++;
      finalShardName = `${orbitLabel}_${deptClean}_${index}`;
      finalKey = `${deptName}.${finalShardName}`;
    }

    // Build layers data
    const popsCount = advancedObjPropConfig.populations;
    const layers = Array.from({ length: popsCount }, (_, i) => ({
      name: `layer_${i}`,
      height_pct: 1.0 / popsCount,
      density: 1.0
    }));

    // Create the shard data object (Absolute coordinates for placement.json)
    const newShard = {
      key: finalKey,
      dept: deptName,
      shard: finalShardName,
      orbit: orbitIndex,
      position: {
        x: Math.round(this.ghostMesh.position.x / VIS_SCALE - advancedObjPropConfig.w / 2),
        y: Math.round(this.ghostMesh.position.z / VIS_SCALE - advancedObjPropConfig.d / 2), // Rust Y (depth)
        z: Math.round(this.ghostMesh.position.y / VIS_SCALE - advancedObjPropConfig.h / 2) // Rust Z (height)
      },
      size: {
        w: advancedObjPropConfig.w,
        d: advancedObjPropConfig.d,
        h: advancedObjPropConfig.h
      },
      layers: layers,
      sockets: []
    };

    // Store modifications in local cache
    placementData.shards.push(newShard);
    store.set('placementData', placementData);
    store.set('hasUnsavedChanges', true);

    // Push creation to history
    import('../../store/history_manager.js').then(({ historyManager }) => {
      historyManager.pushAction('create', 'shard', finalKey, `Создание шарда ${finalShardName}`, null, newShard);
    });

    // Rebuild visual scene data, preserving camera position
    buildSceneData(placementData, true);

    // Auto-select the newly added shard
    selectShard(finalKey);

    // Auto-transition to translate mode with transform gizmo active
    modeManager.setMode('translate');

    showToast(`Шард ${finalShardName} успешно создан!`, "success");
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
