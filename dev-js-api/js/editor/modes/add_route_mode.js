import * as THREE from 'three';
import { scene, camera } from '../../viewer.js';
import { socketMeshes, VIS_SCALE, buildSceneData } from '../../scene_builder.js';
import { deselectAll, selectRoute } from '../selection.js';
import { store } from '../../store/store.js';
import { showToast } from '../../ui/toast.js';
import { modeManager } from '../../editor.js';
import { emit, EVENTS } from '../../store/event_bus.js';

export class AddRouteMode {
  constructor() {
    this.startSocketKey = null;
    this.tempLine = null;
    this.pulseOutlines = [];
    this.pulseTime = 0;
  }

  enter() {
    deselectAll();
    this.startSocketKey = null;
    document.body.style.cursor = 'default';
    showToast("Режим создания связи. Выберите исходный сокет.", "info");

    this.pulseOutlines = [];
    this.pulseTime = 0;

    const placementData = store.get('placementData');
    if (placementData && placementData.connections) {
      const connectedKeys = new Set();
      placementData.connections.forEach(c => {
        connectedKeys.add(`${c.from}.${c.from_socket}`);
        connectedKeys.add(`${c.to}.${c.to_socket}`);
      });

      for (const [socketKey, socketGroup] of Object.entries(socketMeshes)) {
        if (!connectedKeys.has(socketKey)) {
          const width = socketGroup.userData.width;
          const height = socketGroup.userData.height;
          const pitch = socketGroup.userData.pitch;
          const faceSign = socketGroup.userData.faceSign;

          const backingW = width * pitch * VIS_SCALE;
          const backingH = height * pitch * VIS_SCALE;
          
          const halfW = backingW / 2;
          const halfH = backingH / 2;
          
          const zOffset = faceSign * (0.075 * VIS_SCALE + 0.012);
          
          const framePoints = [
            new THREE.Vector3(-halfW, -halfH, zOffset),
            new THREE.Vector3(halfW, -halfH, zOffset),
            new THREE.Vector3(halfW, halfH, zOffset),
            new THREE.Vector3(-halfW, halfH, zOffset)
          ];
          
          const borderGeo = new THREE.BufferGeometry().setFromPoints(framePoints);
          const borderMat = new THREE.LineBasicMaterial({
            color: 0x10b981,
            linewidth: 3,
            transparent: true,
            opacity: 0.9,
            depthTest: true
          });
          const pulseOutline = new THREE.LineLoop(borderGeo, borderMat);
          pulseOutline.name = "unconnected_pulse_outline";
          
          socketGroup.add(pulseOutline);
          this.pulseOutlines.push(pulseOutline);
        }
      }
    }
  }

  exit() {
    this.cleanupPulseOutlines();
    this.cleanupTempLine();
    document.body.style.cursor = 'default';
  }

  cleanupTempLine() {
    if (this.tempLine) {
      scene.remove(this.tempLine);
      this.tempLine.geometry.dispose();
      this.tempLine.material.dispose();
      this.tempLine = null;
    }
  }

  cleanupPulseOutlines() {
    if (this.pulseOutlines) {
      this.pulseOutlines.forEach(outline => {
        if (outline.parent) {
          outline.parent.remove(outline);
        }
        if (outline.geometry) outline.geometry.dispose();
        if (outline.material) outline.material.dispose();
      });
      this.pulseOutlines = [];
    }
  }

  onUpdate(dt) {
    if (this.pulseOutlines && this.pulseOutlines.length > 0) {
      this.pulseTime += dt * 4;
      const opacity = 0.35 + 0.45 * Math.sin(this.pulseTime);
      const scale = 1.0 + 0.03 * Math.abs(Math.sin(this.pulseTime));
      
      this.pulseOutlines.forEach(outline => {
        if (outline.material) {
          outline.material.opacity = opacity;
        }
        outline.scale.set(scale, scale, 1);
      });
    }
  }

  onPointerMove(event, raycaster) {
    const socketsList = Object.values(socketMeshes);
    const socketHits = raycaster.intersectObjects(socketsList, true);

    if (this.startSocketKey) {
      // Trace line from starting socket center to mouse position
      const startGroup = socketMeshes[this.startSocketKey];
      if (startGroup) {
        const startPos = new THREE.Vector3();
        startGroup.getWorldPosition(startPos);

        // Find intersection point on the plane parallel to the viewport or at start position height
        const plane = new THREE.Plane(new THREE.Vector3(0, 1, 0), -startPos.y);
        const targetPos = new THREE.Vector3();
        raycaster.ray.intersectPlane(plane, targetPos);

        // If no intersection, use raycaster point at distance 100
        if (!targetPos) {
          raycaster.ray.at(100, targetPos);
        }

        // Rebuild temporary line geometry
        if (this.tempLine) {
          const points = [startPos, targetPos];
          this.tempLine.geometry.setFromPoints(points);
        }
      }

      // If hovering another compatible socket, show pointer, otherwise cell
      if (socketHits.length > 0) {
        let obj = socketHits[0].object;
        while (obj && !obj.userData.socketKey) {
          obj = obj.parent;
        }
        if (obj && obj.userData.socketKey !== this.startSocketKey) {
          document.body.style.cursor = 'pointer';
          return;
        }
      }
      document.body.style.cursor = 'cell';
    } else {
      // If hovering any socket, show pointer
      if (socketHits.length > 0) {
        document.body.style.cursor = 'pointer';
      } else {
        document.body.style.cursor = 'default';
      }
    }
  }

  onPointerDown(event, raycaster) {
    if (event.button === 2) {
      // Right Click Cancel
      modeManager.popMode();
      return true;
    }

    if (event.button !== 0) return false;

    const socketsList = Object.values(socketMeshes);
    const socketHits = raycaster.intersectObjects(socketsList, true);

    if (socketHits.length > 0) {
      let obj = socketHits[0].object;
      while (obj && !obj.userData.socketKey) {
        obj = obj.parent;
      }

      if (obj) {
        const socketKey = obj.userData.socketKey;

        if (!this.startSocketKey) {
          // Select starting socket
          this.startSocketKey = socketKey;
          const startPos = new THREE.Vector3();
          obj.getWorldPosition(startPos);

          const geom = new THREE.BufferGeometry().setFromPoints([startPos, startPos]);
          const mat = new THREE.LineBasicMaterial({
            color: 0x10b981,
            linewidth: 2,
            depthTest: false
          });
          this.tempLine = new THREE.Line(geom, mat);
          this.tempLine.renderOrder = 999;
          scene.add(this.tempLine);

          // Filter out outlines that are on the same shard or on the selected start socket
          const startShardKey = obj.userData.shardKey;
          this.pulseOutlines = this.pulseOutlines.filter(outline => {
            const socketGroup = outline.parent;
            if (!socketGroup) return false;
            
            const isSameShard = socketGroup.userData.shardKey === startShardKey;
            const isStartSocket = socketGroup.userData.socketKey === socketKey;
            
            if (isSameShard || isStartSocket) {
              socketGroup.remove(outline);
              if (outline.geometry) outline.geometry.dispose();
              if (outline.material) outline.material.dispose();
              return false;
            }
            return true;
          });

          showToast("Исходный сокет выбран. Выберите целевой сокет.", "info");
          return true;
        } else {
          // Select target socket
          if (this.startSocketKey === socketKey) {
            showToast("Нельзя соединить сокет с самим собой!", "warning");
            return true;
          }

          const startGroup = socketMeshes[this.startSocketKey];
          const endGroup = obj;

          if (startGroup.userData.shardKey === endGroup.userData.shardKey) {
            showToast("Нельзя соединять сокеты одного и того же шарда!", "warning");
            return true;
          }

          const placementData = store.get('placementData');
          if (!placementData) return false;

          const fromShard = startGroup.userData.shardKey;
          const fromSocket = startGroup.userData.socketName;
          const toShard = endGroup.userData.shardKey;
          const toSocket = endGroup.userData.socketName;

          // Check if connection already exists
          const exists = placementData.connections.some(c => 
            (c.from === fromShard && c.from_socket === fromSocket && c.to === toShard && c.to_socket === toSocket) ||
            (c.from === toShard && c.from_socket === toSocket && c.to === fromShard && c.to_socket === fromSocket)
          );

          if (exists) {
            showToast("Связь между этими сокетами уже существует!", "warning");
            return true;
          }

          // Create initial control points (subdivided straight line in voxels)
          const startPos = new THREE.Vector3();
          startGroup.getWorldPosition(startPos);
          const startVox = [startPos.x / VIS_SCALE, startPos.y / VIS_SCALE, startPos.z / VIS_SCALE];

          const endPos = new THREE.Vector3();
          endGroup.getWorldPosition(endPos);
          const endVox = [endPos.x / VIS_SCALE, endPos.y / VIS_SCALE, endPos.z / VIS_SCALE];

          const startVec = new THREE.Vector3(...startVox);
          const endVec = new THREE.Vector3(...endVox);
          const distance = startVec.distanceTo(endVec);

          const editorSettings = store.get('editorSettings') || {};
          const subdivStep = editorSettings.cable_subdivision_step || 30;

          const numSegments = Math.max(2, Math.ceil(distance / subdivStep));
          const controlPoints = [];
          for (let i = 0; i <= numSegments; i++) {
            const t = i / numSegments;
            const pt = new THREE.Vector3().lerpVectors(startVec, endVec, t);
            controlPoints.push([
              Number(pt.x.toFixed(2)),
              Number(pt.y.toFixed(2)),
              Number(pt.z.toFixed(2)),
              1.0 // initial radius scale
            ]);
          }

          const newConnection = {
            from: fromShard,
            from_socket: fromSocket,
            to: toShard,
            to_socket: toSocket,
            manual: true,
            matrix_w: startGroup.userData.width,
            matrix_h: startGroup.userData.height,
            control_points: controlPoints
          };

          if (!placementData.connections) {
            placementData.connections = [];
          }
          placementData.connections.push(newConnection);

          store.set('placementData', placementData);
          store.set('hasUnsavedChanges', true);
          emit(EVENTS.LAYOUT_CHANGED);

          const connectionKey = `${fromShard}.${fromSocket}→${toShard}.${toSocket}`;

          // Push action to history
          import('../../store/history_manager.js').then(({ historyManager }) => {
            historyManager.pushAction(
              'create',
              'connection',
              connectionKey,
              `Создание связи между ${fromSocket} и ${toSocket}`,
              null,
              newConnection
            );
          });

          // Re-draw routes
          const routes = store.get('routesData') || [];
          // Also append route representation for renderer lookup
          routes.push({
            from: fromShard,
            from_socket: fromSocket,
            to: toShard,
            to_socket: toSocket,
            matrix_w: startGroup.userData.width,
            matrix_h: startGroup.userData.height,
            manual: true,
            control_points: controlPoints,
            points: controlPoints.map(cp => [cp[0], cp[1], cp[2]])
          });
          store.set('routesData', routes);
          buildSceneData(placementData, true);

          // Select the new route and enter inspect mode
          selectRoute(connectionKey);
          modeManager.setMode('inspect');

          showToast("Связь успешно создана!", "success");
          return true;
        }
      }
    }

    return false;
  }

  onPointerUp(event, raycaster) {}

  onKeyDown(event) {
    if (event.key === 'Escape') {
      modeManager.popMode();
      return true;
    }
    return false;
  }
}
