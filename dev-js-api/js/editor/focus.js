/**
 * @fileoverview focus.js — Focus system dimming inactive elements and highlighting active selections.
 */

import {
  shardMeshes,
  socketMeshes,
  shardDataMap,
  shardsByLevel,
  shardsByDept
} from '../scene_builder.js';
import { store } from '../store/store.js';
import { THEME } from '../rendering/theme.js';

function isSocketConnectedToSelection(socketKey) {
  const routes = store.get('routesData');
  if (!routes) return false;
  const selShardKey = store.get('selectedShardKey');
  const selSocketKey = store.get('selectedSocketKey');

  if (selSocketKey) {
    return routes.some(r => {
      const fromK = `${r.from}.${r.from_socket}`;
      const toK = `${r.to}.${r.to_socket}`;
      return (fromK === selSocketKey && toK === socketKey) || (toK === selSocketKey && fromK === socketKey);
    });
  } else if (selShardKey) {
    return routes.some(r => {
      const fromK = `${r.from}.${r.from_socket}`;
      const toK = `${r.to}.${r.to_socket}`;
      return (r.from === selShardKey && toK === socketKey) || (r.to === selShardKey && fromK === socketKey);
    });
  }
  return false;
}

/**
 * Applies opacity and highlight filters to shard and socket meshes based on current selection.
 */
export function updateFocusVisuals() {
  const selShardKey = store.get('selectedShardKey');
  const selSocketKey = store.get('selectedSocketKey');
  const selRouteKey = store.get('selectedRouteKey');
  const focusedLevelId = store.get('focusedLevelId');
  const hiddenLevelIds = store.get('hiddenLevelIds') || new Set();
  const selectedDeptName = store.get('selectedDeptName');
  const activeMode = store.get('activeMode');
  const data = store.get('placementData');

  let routeConn = null;
  if (selRouteKey) {
    const placementData = store.get('placementData');
    if (placementData && placementData.connections) {
      routeConn = placementData.connections.find(c =>
        `${c.from}.${c.from_socket}→${c.to}.${c.to_socket}` === selRouteKey ||
        `${c.to}.${c.to_socket}→${c.from}.${c.from_socket}` === selRouteKey
      );
    }
  }

  const isAnySelected = !!(selShardKey || selSocketKey || selRouteKey);
  const routes = store.get('routesData');

  // 1. Shards Focus using Flat Memory Layout Caching
  if (focusedLevelId === null) {
    // Global view: all active shards on layer 0 (raycasting active)
    for (const [key, mesh] of Object.entries(shardMeshes)) {
      const sd = shardDataMap[mesh.uuid];
      if (!sd) continue;

      const isHidden = hiddenLevelIds.has(sd.orbit);
      if (isHidden) {
        mesh.visible = false;
        continue;
      }
      mesh.visible = true;

      mesh.layers.set(0);
      mesh.traverse(child => child.layers.set(0));

      const selSocketGroup = selSocketKey ? socketMeshes[selSocketKey] : null;
      const isFocused = (
        selShardKey === key ||
        (selSocketGroup && selSocketGroup.userData.shardKey === key) ||
        (routeConn && (routeConn.from === key || routeConn.to === key))
      );

      const mode = store.get('connectionMode') || 1;
      let isConnectedShard = false;
      if ((mode === 2 || mode === 3) && routes && isAnySelected && !selRouteKey) {
        isConnectedShard = routes.some(r => {
          if (selSocketKey) {
            const fromSock = `${r.from}.${r.from_socket}`;
            const toSock = `${r.to}.${r.to_socket}`;
            if (fromSock === selSocketKey || toSock === selSocketKey) {
              return r.from === key || r.to === key;
            }
          } else if (selShardKey) {
            if (r.from === selShardKey || r.to === selShardKey) {
              return r.from === key || r.to === key;
            }
          }
          return false;
        });
      }

      const mainWire = mesh.children.find(c => c.name === "main_wireframe");

      if (mesh.userData.label) {
        mesh.userData.label.visible = true;
        mesh.userData.label.material.opacity = isAnySelected
          ? ((isFocused || isConnectedShard) ? THEME.label.activeLevelOpacity : THEME.label.activeLevelOpacity * 0.18)
          : THEME.label.activeLevelOpacity;
        mesh.userData.label.material.needsUpdate = true;
      }

      if (isFocused) {
        mesh.material.visible = false;
        if (mainWire) mainWire.visible = false;

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
      } else {
        mesh.material.visible = true;
        mesh.material.transparent = true;
        if (isAnySelected) {
          mesh.material.opacity = isConnectedShard ? THEME.shard.selectedConnectedOpacity : THEME.shard.selectedDimmedOpacity;
          mesh.material.needsUpdate = true;
          if (mainWire) {
            mainWire.visible = true;
            mainWire.material.opacity = isConnectedShard ? 0.85 : 0.15;
            mainWire.material.needsUpdate = true;
          }
        } else {
          if (activeMode === 'translate' || activeMode === 'resize') {
            mesh.material.opacity = THEME.shard.modeDimmedOpacity;
            mesh.material.transparent = true;
          } else {
            mesh.material.opacity = THEME.shard.activeLevelOpacity;
            mesh.material.transparent = false;
          }
          mesh.material.needsUpdate = true;
          if (mainWire) {
            mainWire.visible = true;
            mainWire.material.opacity = 0.85;
            mainWire.material.transparent = true;
            mainWire.material.needsUpdate = true;
          }
        }

        mesh.children.forEach(child => {
          if (child.userData && (child.userData.layerIndex !== undefined || child.userData.isDivider)) {
            child.visible = false;
          }
        });
      }
    }
  } else {
    // Focused level view
    // 1. Hide/dim shards of inactive levels
    Object.keys(shardsByLevel).forEach(lvlIdStr => {
      const lvlId = parseInt(lvlIdStr);
      if (lvlId !== focusedLevelId) {
        const meshes = shardsByLevel[lvlId] || [];
        meshes.forEach(mesh => {
          mesh.visible = !hiddenLevelIds.has(lvlId);
          if (mesh.visible) {
            mesh.layers.set(1);
            mesh.traverse(child => child.layers.set(1));

            mesh.material.visible = true;
            mesh.material.transparent = true;
            mesh.material.opacity = THEME.shard.inactiveLevelOpacity;
            mesh.material.needsUpdate = true;

            const mainWire = mesh.children.find(c => c.name === "main_wireframe");
            if (mainWire) {
              mainWire.visible = true;
              mainWire.material.opacity = THEME.shard.inactiveLevelOpacity;
              mainWire.material.needsUpdate = true;
            }

            if (mesh.userData.label) {
              mesh.userData.label.visible = false;
            }

            mesh.children.forEach(child => {
              if (child.userData && (child.userData.layerIndex !== undefined || child.userData.isDivider)) {
                child.visible = false;
              }
            });
          }
        });
      }
    });

    // 2. Process shards on the active focused level
    const activeLevelMeshes = shardsByLevel[focusedLevelId] || [];
    const isLevelHidden = hiddenLevelIds.has(focusedLevelId);

    activeLevelMeshes.forEach(mesh => {
      if (isLevelHidden) {
        mesh.visible = false;
        return;
      }
      mesh.visible = true;

      const sd = shardDataMap[mesh.uuid];
      if (!sd) return;

      const isDeptActive = (selectedDeptName !== null && sd.dept === selectedDeptName);

      if (selectedDeptName === null) {
        // Level is selected but Dept is not: dim shards, turn off raycast (layer 1)
        mesh.layers.set(1);
        mesh.traverse(child => child.layers.set(1));

        mesh.material.visible = true;
        mesh.material.transparent = true;
        mesh.material.opacity = THEME.shard.inactiveDeptOpacity;
        mesh.material.needsUpdate = true;

        const mainWire = mesh.children.find(c => c.name === "main_wireframe");
        if (mainWire) {
          mainWire.visible = true;
          mainWire.material.opacity = THEME.shard.inactiveDeptOpacity;
          mainWire.material.needsUpdate = true;
        }

        if (mesh.userData.label) {
          mesh.userData.label.visible = false;
        }

        mesh.children.forEach(child => {
          if (child.userData && (child.userData.layerIndex !== undefined || child.userData.isDivider)) {
            child.visible = false;
          }
        });
      } else {
        // Department is active
        if (isDeptActive) {
          // Active dept shard: layer 0, focus styling applied
          mesh.layers.set(0);
          mesh.traverse(child => child.layers.set(0));

          const selSocketGroup = selSocketKey ? socketMeshes[selSocketKey] : null;
          const isFocused = (
            selShardKey === sd.key ||
            (selSocketGroup && selSocketGroup.userData.shardKey === sd.key) ||
            (routeConn && (routeConn.from === sd.key || routeConn.to === sd.key))
          );

          const mode = store.get('connectionMode') || 1;
          let isConnectedShard = false;
          if ((mode === 2 || mode === 3) && routes && isAnySelected && !selRouteKey) {
            isConnectedShard = routes.some(r => {
              if (selSocketKey) {
                const fromSock = `${r.from}.${r.from_socket}`;
                const toSock = `${r.to}.${r.to_socket}`;
                if (fromSock === selSocketKey || toSock === selSocketKey) {
                  return r.from === sd.key || r.to === sd.key;
                }
              } else if (selShardKey) {
                if (r.from === selShardKey || r.to === selShardKey) {
                  return r.from === sd.key || r.to === sd.key;
                }
              }
              return false;
            });
          }

          const mainWire = mesh.children.find(c => c.name === "main_wireframe");

          if (mesh.userData.label) {
            mesh.userData.label.visible = true;
            mesh.userData.label.material.opacity = isAnySelected
              ? ((isFocused || isConnectedShard) ? THEME.label.activeLevelOpacity : THEME.label.activeLevelOpacity * 0.18)
              : THEME.label.activeLevelOpacity;
            mesh.userData.label.material.needsUpdate = true;
          }

          if (isFocused) {
            mesh.material.visible = false;
            if (mainWire) mainWire.visible = false;

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
          } else {
            mesh.material.visible = true;
            mesh.material.transparent = true;
            if (isAnySelected) {
              mesh.material.opacity = isConnectedShard ? THEME.shard.selectedConnectedOpacity : THEME.shard.selectedDimmedOpacity;
              mesh.material.needsUpdate = true;
              if (mainWire) {
                mainWire.visible = true;
                mainWire.material.opacity = isConnectedShard ? 0.85 : 0.15;
                mainWire.material.needsUpdate = true;
              }
            } else {
              if (activeMode === 'translate' || activeMode === 'resize') {
                mesh.material.opacity = THEME.shard.modeDimmedOpacity;
                mesh.material.transparent = true;
              } else {
                mesh.material.opacity = THEME.shard.activeLevelOpacity;
                mesh.material.transparent = false;
              }
              mesh.material.needsUpdate = true;
              if (mainWire) {
                mainWire.visible = true;
                mainWire.material.opacity = 0.85;
                mainWire.material.transparent = true;
                mainWire.material.needsUpdate = true;
              }
            }

            mesh.children.forEach(child => {
              if (child.userData && (child.userData.layerIndex !== undefined || child.userData.isDivider)) {
                child.visible = false;
              }
            });
          }
        } else {
          // Shard of inactive dept on active level: layer 1, dim
          mesh.layers.set(1);
          mesh.traverse(child => child.layers.set(1));

          mesh.material.visible = true;
          mesh.material.transparent = true;
          mesh.material.opacity = THEME.shard.inactiveDeptOpacity;
          mesh.material.needsUpdate = true;

          const mainWire = mesh.children.find(c => c.name === "main_wireframe");
          if (mainWire) {
            mainWire.visible = true;
            mainWire.material.opacity = THEME.shard.inactiveDeptOpacity;
            mainWire.material.needsUpdate = true;
          }

          if (mesh.userData.label) {
            mesh.userData.label.visible = false;
          }

          mesh.children.forEach(child => {
            if (child.userData && (child.userData.layerIndex !== undefined || child.userData.isDivider)) {
              child.visible = false;
            }
          });
        }
      }
    });
  }

  // 2. Sockets Focus
  for (const [key, group] of Object.entries(socketMeshes)) {
    const shardKey = group.userData?.shardKey;
    const shard = shardKey && data ? data.shards.find(s => s.key === shardKey) : null;
    const isLevelFocused = shard ? (focusedLevelId === null || shard.orbit === focusedLevelId) : true;
    const isDeptFocused = shard ? (selectedDeptName === null || shard.dept === selectedDeptName) : true;
    const isHidden = shard ? hiddenLevelIds.has(shard.orbit) : false;

    if (isHidden) {
      group.visible = false;
      continue;
    }
    group.visible = true;

    // Sockets are active in raycast only when both level and dept are active
    const isActiveLayer = isLevelFocused && isDeptFocused;
    const targetLayer = isActiveLayer ? 0 : 1;
    group.layers.set(targetLayer);
    group.traverse(child => {
      child.layers.set(targetLayer);
    });

    const isFocused = (store.get('selectedSocketKey') === key);
    const backing = group.userData.backingMesh;
    const instMesh = group.children.find(c => c.isInstancedMesh);

    const mode = store.get('connectionMode') || 1;
    const isConnected = isSocketConnectedToSelection(key) ||
      (routeConn && (`${routeConn.from}.${routeConn.from_socket}` === key || `${routeConn.to}.${routeConn.to_socket}` === key));
    const shouldHighlightSocket = isFocused || ((mode === 2 || mode === 3 || selRouteKey) && isConnected);

    if (!isActiveLayer) {
      // Inactive level or department: dim completely
      if (backing) {
        backing.material.opacity = THEME.socket.inactiveLevelOpacity;
        backing.material.transparent = true;
        backing.material.needsUpdate = true;
      }
      if (instMesh) {
        instMesh.material.opacity = THEME.socket.inactiveLevelOpacity;
        instMesh.material.transparent = true;
        instMesh.material.needsUpdate = true;
      }
      continue;
    }

    // Sockets on active level & active dept
    if (isAnySelected) {
      if (backing) {
        backing.material.opacity = shouldHighlightSocket ? THEME.socket.highlightBackingOpacity : THEME.socket.dimmedBackingOpacity;
        const origColor = group.userData.originalBackingColor !== undefined ? group.userData.originalBackingColor : 0x050508;
        backing.material.color.setHex(shouldHighlightSocket ? 0x8b9cf7 : origColor);
        backing.material.visible = shouldHighlightSocket ? true : (group.userData.originalBackingVisible !== false);
        backing.material.needsUpdate = true;
      }
      if (instMesh) {
        instMesh.material.opacity = shouldHighlightSocket ? THEME.socket.activeLevelOpacity : THEME.socket.dimmedOpacity;
        instMesh.material.transparent = !shouldHighlightSocket;
        instMesh.material.needsUpdate = true;
      }
    } else {
      // Restore standard states
      if (backing) {
        backing.material.opacity = THEME.socket.defaultBackingOpacity;
        const origColor = group.userData.originalBackingColor !== undefined ? group.userData.originalBackingColor : 0x050508;
        backing.material.color.setHex(origColor);
        backing.material.visible = (group.userData.originalBackingVisible !== false);
        backing.material.needsUpdate = true;
      }
      if (instMesh) {
        instMesh.material.opacity = THEME.socket.activeLevelOpacity;
        instMesh.material.transparent = false;
        instMesh.material.needsUpdate = true;
      }
    }
  }
}

// Self-subscribe to store changes
store.on('focusedLevelId', () => {
  updateFocusVisuals();
});
store.on('hiddenLevelIds', () => {
  updateFocusVisuals();
});
store.on('selectedDeptName', () => {
  updateFocusVisuals();
});
store.on('activeMode', () => {
  updateFocusVisuals();
});
store.on('placementData', () => {
  updateFocusVisuals();
});
