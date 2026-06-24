/**
 * @fileoverview transform.js — Coordinates the TransformControls widget for moving shards and sockets.
 */

import * as THREE from 'three';
import { TransformControls } from 'three/addons/controls/TransformControls.js';
import { camera, renderer, scene, controls } from '../viewer.js';
import { shardMeshes, socketMeshes, VIS_SCALE, shardDataMap, updateContainerWires } from '../scene_builder.js';
import { store } from '../store/store.js';
import { emit, EVENTS } from '../store/event_bus.js';
import { checkShardCollision } from './collision_adapter.js';

export let transformControls = null;

/**
 * Initializes the TransformControls widget.
 */
export function initTransformControls() {
  transformControls = new TransformControls(camera, renderer.domElement);
  transformControls.size = 0.75;
  transformControls.setMode('translate');

  transformControls.addEventListener('objectChange', () => {
    const selShardKey = store.get('selectedShardKey');
    const selSocketKey = store.get('selectedSocketKey');

    const editorSettings = store.get('editorSettings') || {};
    const snapStep = editorSettings.snap_step || 1;

    if (selShardKey) {
      const mesh = shardMeshes[selShardKey];
      const sd = shardDataMap[mesh.uuid];
      if (mesh && sd) {
        const w = sd.size.w;
        const d = sd.size.d;
        const h = sd.size.h;

        // Reconstruct AABB min coordinates in Three.js space (Y is height, Z is depth)
        let posX = mesh.position.x / VIS_SCALE - w / 2;
        let posY = mesh.position.y / VIS_SCALE - h / 2;
        let posZ = mesh.position.z / VIS_SCALE - d / 2;

        // Snap to grid
        posX = Math.round(posX / snapStep) * snapStep;
        posY = Math.round(posY / snapStep) * snapStep;
        posZ = Math.round(posZ / snapStep) * snapStep;

        // Reconstruct center position for Three.js
        mesh.position.x = (posX + w / 2) * VIS_SCALE;
        mesh.position.y = (posY + h / 2) * VIS_SCALE;
        mesh.position.z = (posZ + d / 2) * VIS_SCALE;

        // Collision Check: revert position if it overlaps other shards on the same layer
        if (checkShardCollision(selShardKey, mesh.position)) {
          mesh.position.copy(mesh.userData.lastValidPosition);
        } else {
          mesh.userData.lastValidPosition.copy(mesh.position);
        }

        // Dynamic update coordinates in the sidebar
        const ix = document.getElementById('shard-px');
        const iy = document.getElementById('shard-py');
        const iz = document.getElementById('shard-pz');
        
        // Re-calculate visual AABB min from the final valid snapped mesh position
        const finalX = Math.round(mesh.position.x / VIS_SCALE - w / 2);
        const finalY = Math.round(mesh.position.y / VIS_SCALE - h / 2);
        const finalZ = Math.round(mesh.position.z / VIS_SCALE - d / 2);

        if (ix) ix.value = finalX;
        if (iy) iy.value = finalY;
        if (iz) iz.value = finalZ;

        emit(EVENTS.SHARD_DRAGGING, {
          key: selShardKey,
          position: { x: finalX, y: finalY, z: finalZ }
        });
      }
    } else if (selSocketKey) {
      const group = socketMeshes[selSocketKey];
      if (group) {
        const shardMesh = shardMeshes[group.userData.shardKey];
        if (shardMesh) {
          // Snap position relative to shard face to exactly snapStep grid
          let localX = Math.round(group.position.x / (snapStep * VIS_SCALE)) * snapStep;
          let localY = Math.round(group.position.y / (snapStep * VIS_SCALE)) * snapStep;
          let localZ = Math.round(group.position.z / (snapStep * VIS_SCALE)) * snapStep;

          // Clamp bounds
          const shardW = shardMesh.geometry.parameters.width / VIS_SCALE;
          const shardD = shardMesh.geometry.parameters.height / VIS_SCALE; // local Y is depth
          const shardH = shardMesh.geometry.parameters.depth / VIS_SCALE; // local Z is height/thickness
          
          const backing = group.userData.backingMesh;
          const backingW = backing.geometry.parameters.width / VIS_SCALE;
          const backingH = backing.geometry.parameters.height / VIS_SCALE;

          const limitX = Math.floor((shardW - backingW) / 2);
          const limitY = Math.floor((shardD - backingH) / 2);

          localX = Math.max(-limitX, Math.min(limitX, localX));
          localY = Math.max(-limitY, Math.min(limitY, localY));
          
          // Clamp Z coordinate to shard thickness bounds
          localZ = Math.max(-shardH / 2, Math.min(shardH / 2, localZ));

          group.position.set(localX * VIS_SCALE, localY * VIS_SCALE, localZ * VIS_SCALE);

          // Auto-linking to layers based on Z position
          const placementData = store.get('placementData');
          const shard = placementData ? placementData.shards.find(s => s.key === group.userData.shardKey) : null;
          
          let entry_z = group.userData.entry_z || (group.userData.faceSign === 1 ? "top" : "bottom");
          
          // Thresholds to snap to top/bottom faces
          if (localZ >= shardH / 2 - 0.05) {
            entry_z = "top";
          } else if (localZ <= -shardH / 2 + 0.05) {
            entry_z = "bottom";
          } else if (shard && shard.layers && shard.layers.length > 0) {
            let currentZ = -shardH / 2;
            for (let i = 0; i < shard.layers.length; i++) {
              const layer = shard.layers[i];
              const layerHeight = shardH * layer.height_pct;
              const nextZ = currentZ + layerHeight;
              if (localZ >= currentZ - 0.01 && localZ < nextZ + 0.01) {
                entry_z = layer.name;
                break;
              }
              currentZ = nextZ;
            }
          }

          group.userData.originalOffset = { x: localX, y: localY, z: localZ };
          group.userData.entry_z = entry_z;

          // Update sidebar inputs
          const ox = document.getElementById('sock-ox');
          const oy = document.getElementById('sock-oy');
          if (ox && oy) {
            ox.value = localX;
            oy.value = localY;
          }
          const entryZSpan = document.getElementById('sock-entry-z-display');
          if (entryZSpan) {
            entryZSpan.textContent = entry_z;
          }
        }
      }
    }
    updateContainerWires();
    emit(EVENTS.VALIDATION_REQ);
  });

  // Temporarily disable OrbitControls while dragging
  transformControls.addEventListener('dragging-changed', (event) => {
    controls.enabled = !event.value;

    if (!event.value) {
      const selShardKey = store.get('selectedShardKey');
      const selSocketKey = store.get('selectedSocketKey');
      const placementData = store.get('placementData');
      if (!placementData) return;

      if (selShardKey) {
        const mesh = shardMeshes[selShardKey];
        const shard = placementData.shards.find(s => s.key === selShardKey);
        if (mesh && shard) {
          const sd = shardDataMap[mesh.uuid];

          const w = sd.size.w;
          const d = sd.size.d;
          const h = sd.size.h;

          const oldPosition = JSON.parse(JSON.stringify(shard.position));
          const newPosition = {
            x: Math.round(mesh.position.x / VIS_SCALE - w / 2),
            y: Math.round(mesh.position.y / VIS_SCALE - h / 2),
            z: Math.round(mesh.position.z / VIS_SCALE - d / 2)
          };

          if (oldPosition.x !== newPosition.x || oldPosition.y !== newPosition.y || oldPosition.z !== newPosition.z) {
            const undoState = JSON.parse(JSON.stringify(shard));
            
            // Save coordinates to placementData in store
            shard.position = newPosition;
            store.set('placementData', placementData);
            
            emit(EVENTS.SHARD_TRANSFORMED, {
              key: selShardKey,
              position: newPosition,
              size: shard.size
            });
            
            const redoState = JSON.parse(JSON.stringify(shard));

            import('../store/history_manager.js').then(({ historyManager }) => {
              historyManager.pushAction('move', 'shard', selShardKey, `Перемещение шарда ${selShardKey}`, undoState, redoState);
            });
          }
        }
      } else if (selSocketKey) {
        const group = socketMeshes[selSocketKey];
        if (group) {
          const { shardKey, socketName } = group.userData;
          const shard = placementData.shards.find(s => s.key === shardKey);
          if (shard && shard.sockets) {
            const socket = shard.sockets.find(s => s.name === socketName);
            if (socket) {
              const oldOffset = JSON.parse(JSON.stringify(socket.offset || { x: 0, y: 0, z: group.userData.faceSign * (shard.size.h / 2) }));
              const newOffset = {
                x: Number(group.userData.originalOffset.x.toFixed(2)),
                y: Number(group.userData.originalOffset.y.toFixed(2)),
                z: Number((group.userData.originalOffset.z !== undefined ? group.userData.originalOffset.z : 0).toFixed(2))
              };
              const oldEntryZ = socket.entry_z;
              const newEntryZ = group.userData.entry_z || socket.entry_z;

              if (oldOffset.x !== newOffset.x || oldOffset.y !== newOffset.y || oldOffset.z !== newOffset.z || oldEntryZ !== newEntryZ) {
                const undoState = JSON.parse(JSON.stringify(socket));
                
                socket.offset = newOffset;
                if (newEntryZ) {
                  socket.entry_z = newEntryZ;
                }
                store.set('placementData', placementData);
            updateContainerWires();
                
                const redoState = JSON.parse(JSON.stringify(socket));

                import('../store/history_manager.js').then(({ historyManager }) => {
                  historyManager.pushAction('move', 'socket', selSocketKey, `Перемещение сокета ${socketName}`, undoState, redoState);
                });
              }
            }
          }
        }
      }
    }
  });

  scene.add(transformControls);
}
