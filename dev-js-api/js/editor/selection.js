/**
 * @fileoverview selection.js — Selection system for clicking shards and sockets, raycasting, and deselecting.
 */

import * as THREE from 'three';
import { camera, scene } from '../viewer.js';
import { shardMeshes, socketMeshes, shardDataMap, drawRoutes, VIS_SCALE } from '../scene_builder.js';
import { transformControls } from './transform.js';
import { updateFocusVisuals } from './focus.js';
import { store } from '../store/store.js';
import { on, emit, EVENTS } from '../store/event_bus.js';

let raycaster = new THREE.Raycaster();
let mouse = new THREE.Vector2();

// Soma functions will be imported from coordinate re-exporter or direct modules
import { spawnSomasForShard, clearSomas } from '../editor.js';

export function selectShard(key) {
  // Clean up previous selection artifacts directly
  updateSocketHandlesVisibility();
  if (transformControls) transformControls.detach();
  clearSomas();

  // Set the new selected shard key and clear socket selection
  store.set('selectedShardKey', key);
  store.set('selectedSocketKey', null);
  store.set('connectionMode', 1);

  const mesh = shardMeshes[key];
  if (mesh) {
    // Record current position as valid for collision checks
    mesh.userData.lastValidPosition = mesh.position.clone();

    // Attach TransformControls
    transformControls.attach(mesh);
    transformControls.space = 'world';
    transformControls.showX = true;
    transformControls.showY = false; // Locked to horizontal Y height
    transformControls.showZ = true;
    const editorSettings = store.get('editorSettings') || {};
    const snapStep = editorSettings.snap_step || 1;
    transformControls.translationSnap = snapStep * VIS_SCALE;

    // Apply Focus opacity states
    updateFocusVisuals();

    // Redraw routes to highlight active ones
    const routes = store.get('routesData');
    if (routes) drawRoutes(routes);

    // Spawn 3D somas for this shard
    spawnSomasForShard(key);

    // Emit selection changed event (UI panel listens to this)
    const shardData = shardDataMap[mesh.uuid];
    emit(EVENTS.SELECTION_CHANGED, { type: 'shard', data: shardData });
  }
}

export function selectSocket(key) {
  // Clean up previous selection artifacts directly
  updateSocketHandlesVisibility();
  if (transformControls) transformControls.detach();
  clearSomas();

  // Set the new selected socket key and clear shard selection
  store.set('selectedSocketKey', key);
  store.set('selectedShardKey', null);
  store.set('connectionMode', 2);

  const group = socketMeshes[key];
  if (group) {
    // Show resizer handles if in resize mode
    updateSocketHandlesVisibility();

    // Attach local-space TransformControls
    transformControls.attach(group);
    transformControls.space = 'local';
    transformControls.showX = true;
    transformControls.showY = true;
    transformControls.showZ = true; // Allow local X-Y-Z movements
    const editorSettings = store.get('editorSettings') || {};
    const snapStep = editorSettings.snap_step || 1;
    transformControls.translationSnap = snapStep * VIS_SCALE;

    // Apply Focus opacity states
    updateFocusVisuals();

    // Redraw routes to highlight active ones
    const routes = store.get('routesData');
    if (routes) drawRoutes(routes);
    
    // Emit selection changed event (UI panel listens to this)
    emit(EVENTS.SELECTION_CHANGED, { type: 'socket', data: group.userData });
  }
}

export function deselectAll() {
  updateSocketHandlesVisibility();

  store.set('selectedShardKey', null);
  store.set('selectedSocketKey', null);
  store.set('selectedRouteKey', null);
  store.set('connectionMode', 1);

  if (transformControls) transformControls.detach();
  updateFocusVisuals();

  // Hide route editor
  import('./route_editor.js').then(({ hideRouteEditor }) => {
    hideRouteEditor();
  });

  // Redraw routes to clear highlight
  const routes = store.get('routesData');
  if (routes) drawRoutes(routes);

  // Clear somas
  clearSomas();

  // Emit selection changed event
  emit(EVENTS.SELECTION_CHANGED, { type: null, data: null });
  document.body.style.cursor = 'auto';
}

export function selectRoute(routeKey) {
  updateSocketHandlesVisibility();
  if (transformControls) transformControls.detach();
  clearSomas();

  store.set('selectedRouteKey', routeKey);
  store.set('selectedShardKey', null);
  store.set('selectedSocketKey', null);

  updateFocusVisuals();

  // Highlight and show route editor handles
  const placementData = store.get('placementData');
  if (placementData && placementData.connections) {
    const conn = placementData.connections.find(c => 
      `${c.from}.${c.from_socket}→${c.to}.${c.to_socket}` === routeKey ||
      `${c.to}.${c.to_socket}→${c.from}.${c.from_socket}` === routeKey
    );
    if (conn) {
      import('./route_editor.js').then(({ showRouteEditor }) => {
        showRouteEditor(conn);
      });
      // Emit selection changed
      emit(EVENTS.SELECTION_CHANGED, { type: 'connection', data: conn });
    }
  }

  // Redraw routes to highlight active ones
  const routes = store.get('routesData');
  if (routes) drawRoutes(routes);
}

export function updateSocketHandlesVisibility() {
  const selSocketKey = store.get('selectedSocketKey');
  const activeMode = store.get('activeMode');
  const isResizeMode = (activeMode === 'resize');
  
  for (const [key, group] of Object.entries(socketMeshes)) {
    const isSelected = (key === selSocketKey);
    group.traverse(child => {
      if (child.name && child.name.startsWith("handle_")) {
        child.visible = isSelected && isResizeMode;
      }
    });
  }
}

// Reactively sync socket handles visibility when mode changes
on(EVENTS.MODE_CHANGED, () => {
  updateSocketHandlesVisibility();
});
