/**
 * @fileoverview focus.js — Focus system dimming inactive elements and highlighting active selections.
 */

import { shardMeshes, socketMeshes } from '../scene_builder.js';
import { store } from '../store/store.js';

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

  // 1. Shards Focus
  for (const [key, mesh] of Object.entries(shardMeshes)) {
    const selSocketGroup = selSocketKey ? socketMeshes[selSocketKey] : null;
    const isFocused = (
      selShardKey === key || 
      (selSocketGroup && selSocketGroup.userData.shardKey === key) ||
      (routeConn && (routeConn.from === key || routeConn.to === key))
    );
    
    // Also, if Mode 2 or 3 is active, is the shard connected to the selected element?
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

    // Manage label opacity
    if (mesh.userData.label) {
      mesh.userData.label.material.opacity = isAnySelected ? ((isFocused || isConnectedShard) ? 0.75 : 0.12) : 0.65;
      mesh.userData.label.material.needsUpdate = true;
    }

    if (isFocused) {
      // Hide monolith container mesh completely and its wireframe when focused
      mesh.material.visible = false;
      if (mainWire) {
        mainWire.visible = false;
      }

      // Show child layers and dividers
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
            child.material.opacity = 0.0; // Keep plane invisible, hover handles this
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
      // Show monolith container mesh and its wireframe when not focused
      mesh.material.visible = true;
      mesh.material.transparent = true;
      if (isAnySelected) {
        // Dimmed monolith state when another shard/socket is selected
        mesh.material.opacity = isConnectedShard ? 0.5 : 0.08;
        mesh.material.needsUpdate = true;
        if (mainWire) {
          mainWire.visible = true;
          mainWire.material.opacity = isConnectedShard ? 0.85 : 0.15;
          mainWire.material.needsUpdate = true;
        }
      } else {
        // Standard monolith state when nothing is selected
        mesh.material.opacity = 1.0;
        mesh.material.transparent = false;
        mesh.material.needsUpdate = true;
        if (mainWire) {
          mainWire.visible = true;
          mainWire.material.opacity = 0.85;
          mainWire.material.transparent = true;
          mainWire.material.needsUpdate = true;
        }
      }

      // Hide child layers and dividers
      mesh.children.forEach(child => {
        if (child.userData && (child.userData.layerIndex !== undefined || child.userData.isDivider)) {
          child.visible = false;
        }
      });
    }
  }

  // 2. Sockets Focus
  for (const [key, group] of Object.entries(socketMeshes)) {
    const isFocused = (store.get('selectedSocketKey') === key);
    const backing = group.userData.backingMesh;
    const instMesh = group.children.find(c => c.isInstancedMesh);

    const mode = store.get('connectionMode') || 1;
    const isConnected = isSocketConnectedToSelection(key) || 
      (routeConn && (`${routeConn.from}.${routeConn.from_socket}` === key || `${routeConn.to}.${routeConn.to_socket}` === key));
    const shouldHighlightSocket = isFocused || ((mode === 2 || mode === 3 || selRouteKey) && isConnected);

    if (isAnySelected) {
      if (backing) {
        backing.material.opacity = shouldHighlightSocket ? 0.75 : 0.1;
        const origColor = group.userData.originalBackingColor !== undefined ? group.userData.originalBackingColor : 0x050508;
        backing.material.color.setHex(shouldHighlightSocket ? 0x8b9cf7 : origColor);
        // Volumetric backing is always visible when highlighted, otherwise matches its original visibility state
        backing.material.visible = shouldHighlightSocket ? true : (group.userData.originalBackingVisible !== false);
        backing.material.needsUpdate = true;
      }
      if (instMesh) {
        // Active socket pins stay fully opaque, other sockets are dimmed/semi-transparent
        instMesh.material.opacity = shouldHighlightSocket ? 1.0 : 0.12;
        instMesh.material.transparent = !shouldHighlightSocket;
        instMesh.material.needsUpdate = true;
      }
    } else {
      // Restore standard states
      if (backing) {
        backing.material.opacity = 0.7;
        const origColor = group.userData.originalBackingColor !== undefined ? group.userData.originalBackingColor : 0x050508;
        backing.material.color.setHex(origColor);
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
}
