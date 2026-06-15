import * as THREE from 'three';
import { scene, camera, renderer, controls } from '../viewer.js';
import { shardMeshes, socketMeshes, VIS_SCALE } from '../scene_builder.js';
import { store } from '../store/store.js';
import { emit, EVENTS } from '../store/event_bus.js';
import { transformControls } from './transform.js';
import { createVariableRadiusTubeGeometry } from '../rendering/route_renderer.js';
import { drawRoutes } from '../rendering/route_renderer.js';

export const routeEditorGroup = new THREE.Group();
let routeEditorGroupAdded = false;

function ensureGroupInScene() {
  if (!routeEditorGroupAdded && scene) {
    scene.add(routeEditorGroup);
    routeEditorGroupAdded = true;
  }
}

let activeRoute = null;
const controlSpheres = [];

// Helper to subdivide points
function subdividePoints(points, step) {
  if (!points || points.length < 2) return [];
  const controlPoints = [];
  
  // Add start point
  controlPoints.push([...points[0], 1.0]); // [x, y, z, radius_scale]
  
  let accumDist = 0;
  let lastPt = new THREE.Vector3(...points[0]);
  
  for (let i = 1; i < points.length - 1; i++) {
    const pt = new THREE.Vector3(...points[i]);
    const dist = lastPt.distanceTo(pt);
    accumDist += dist;
    if (accumDist >= step) {
      controlPoints.push([...points[i], 1.0]);
      accumDist = 0;
    }
    lastPt = pt;
  }
  
  // Add end point if it's not too close to the last added control point
  const endPt = points[points.length - 1];
  const lastAdded = controlPoints[controlPoints.length - 1];
  const distToEnd = Math.sqrt(
    Math.pow(endPt[0] - lastAdded[0], 2) +
    Math.pow(endPt[1] - lastAdded[1], 2) +
    Math.pow(endPt[2] - lastAdded[2], 2)
  );
  
  if (distToEnd > 2) {
    controlPoints.push([...endPt, 1.0]);
  } else {
    controlPoints[controlPoints.length - 1] = [...endPt, 1.0];
  }
  
  return controlPoints;
}

// Compute tangent and Frenet vectors at a specific control point index
function computeTangentAtNode(points, index) {
  const p = points.map(pt => new THREE.Vector3(pt[0], pt[1], pt[2]));
  const T = new THREE.Vector3();
  if (points.length < 2) {
    T.set(0, 0, 1);
    return T;
  }
  if (index === 0) {
    T.subVectors(p[1], p[0]).normalize();
  } else if (index === p.length - 1) {
    T.subVectors(p[p.length - 1], p[p.length - 2]).normalize();
  } else {
    const T1 = new THREE.Vector3().subVectors(p[index], p[index - 1]).normalize();
    const T2 = new THREE.Vector3().subVectors(p[index + 1], p[index]).normalize();
    T.addVectors(T1, T2).normalize();
  }
  return T;
}

/**
 * Activate the interactive route editing for a given connection/route.
 */
export function showRouteEditor(route) {
  ensureGroupInScene();
  hideRouteEditor();
  activeRoute = route;

  const placementData = store.get('placementData');
  if (!placementData) return;

  // Sync route data with placementData.connections
  const conn = placementData.connections.find(c => 
    `${c.from}.${c.from_socket}→${c.to}.${c.to_socket}` === `${route.from}.${route.from_socket}→${route.to}.${route.to_socket}` ||
    `${c.to}.${c.to_socket}→${c.from}.${c.from_socket}` === `${route.from}.${route.from_socket}→${route.to}.${route.to_socket}`
  );
  if (!conn) return;

  const editorSettings = store.get('editorSettings') || {};
  const subdivStep = editorSettings.cable_subdivision_step || 30;

  // Initialize control points if they don't exist yet
  if (!conn.control_points || conn.control_points.length < 2) {
    conn.control_points = subdividePoints(route.points || [], subdivStep);
    conn.manual = true;
    route.control_points = conn.control_points;
    route.manual = true;
    
    // Save state changes
    store.set('placementData', placementData);
    store.set('hasUnsavedChanges', true);
    emit(EVENTS.LAYOUT_CHANGED);
    
    // Force redraw of routes using the new manual geometry
    const routes = store.get('routesData') || [];
    drawRoutes(routes);
  }

  // Get socket configurations
  const sockFromKey = `${conn.from}.${conn.from_socket}`;
  const sockToKey = `${conn.to}.${conn.to_socket}`;
  const sockFromMesh = socketMeshes[sockFromKey];
  const sockToMesh = socketMeshes[sockToKey];

  const wFrom = sockFromMesh ? sockFromMesh.userData.width : 4;
  const hFrom = sockFromMesh ? sockFromMesh.userData.height : 4;
  const pitchFrom = sockFromMesh ? sockFromMesh.userData.pitch : 2;

  const wTo = sockToMesh ? sockToMesh.userData.width : 4;
  const hTo = sockToMesh ? sockToMesh.userData.height : 4;
  const pitchTo = sockToMesh ? sockToMesh.userData.pitch : 2;

  const M = conn.control_points.length;

  for (let k = 0; k < M; k++) {
    const cp = conn.control_points[k];
    const px = cp[0] * VIS_SCALE;
    const py = cp[1] * VIS_SCALE;
    const pz = cp[2] * VIS_SCALE;
    const radiusScale = cp[3] !== undefined ? cp[3] : 1.0;

    // Create node sphere handle
    const sphereGeo = new THREE.SphereGeometry(0.5 * VIS_SCALE, 16, 16);
    const sphereMat = new THREE.MeshBasicMaterial({
      color: 0xffaa00,
      transparent: true,
      opacity: 0.65,
      depthTest: true
    });
    const sphere = new THREE.Mesh(sphereGeo, sphereMat);
    sphere.position.set(px, py, pz);
    sphere.scale.setScalar(radiusScale);

    // Compute tangent at this control node to align the frame
    const tangent = computeTangentAtNode(conn.control_points, k);
    sphere.quaternion.setFromUnitVectors(new THREE.Vector3(0, 0, 1), tangent);

    // Create the bold green cross-section frame
    const u = k / (M - 1 || 1);
    const wInterp = wFrom * (1 - u) + wTo * u;
    const hInterp = hFrom * (1 - u) + hTo * u;
    const pitchInterp = pitchFrom * (1 - u) + pitchTo * u;

    const wWorld = wInterp * pitchInterp * VIS_SCALE;
    const hWorld = hInterp * pitchInterp * VIS_SCALE;

    const halfW = wWorld / 2;
    const halfH = hWorld / 2;

    const framePoints = [
      new THREE.Vector3(halfW, halfH, 0),
      new THREE.Vector3(-halfW, halfH, 0),
      new THREE.Vector3(-halfW, -halfH, 0),
      new THREE.Vector3(halfW, -halfH, 0)
    ];
    const frameGeo = new THREE.BufferGeometry().setFromPoints(framePoints);
    const frameMat = new THREE.LineBasicMaterial({
      color: 0x00ff00,
      linewidth: 3,
      transparent: true,
      opacity: 0.9,
      depthTest: true
    });
    const frameLoop = new THREE.LineLoop(frameGeo, frameMat);
    sphere.add(frameLoop);

    // Store metadata
    sphere.userData = {
      isControlPoint: true,
      connectionKey: `${conn.from}.${conn.from_socket}→${conn.to}.${conn.to_socket}`,
      pointIndex: k,
      routeData: conn
    };

    routeEditorGroup.add(sphere);
    controlSpheres.push(sphere);
  }

  // Attach TransformControls to the first control node by default
  if (controlSpheres.length > 0) {
    transformControls.attach(controlSpheres[0]);
    transformControls.space = 'local';
    transformControls.showX = true;
    transformControls.showY = true;
    transformControls.showZ = true;
    
    const snapStep = editorSettings.snap_step || 1;
    transformControls.translationSnap = snapStep * VIS_SCALE;
  }
}

/**
 * Remove all handles and widgets for route editing.
 */
export function hideRouteEditor() {
  if (transformControls) {
    const activeObj = transformControls.object;
    if (activeObj && activeObj.userData && activeObj.userData.isControlPoint) {
      transformControls.detach();
    }
  }

  // Remove all child meshes from scene
  while (routeEditorGroup.children.length > 0) {
    routeEditorGroup.remove(routeEditorGroup.children[0]);
  }

  controlSpheres.length = 0;
  activeRoute = null;
}

/**
 * Update the active connection/route geometry on the fly.
 */
export function updateRouteSpline(sphere) {
  if (!activeRoute) return;

  const placementData = store.get('placementData');
  if (!placementData) return;

  const conn = placementData.connections.find(c => 
    `${c.from}.${c.from_socket}→${c.to}.${c.to_socket}` === sphere.userData.connectionKey
  );
  if (!conn || !conn.control_points) return;

  const pointIndex = sphere.userData.pointIndex;
  const editorSettings = store.get('editorSettings') || {};
  const snapStep = editorSettings.snap_step || 1;

  // If we are in translate mode, snap coordinates to voxel grid
  if (transformControls.mode === 'translate') {
    const vx = Math.round(sphere.position.x / (snapStep * VIS_SCALE)) * snapStep;
    const vy = Math.round(sphere.position.y / (snapStep * VIS_SCALE)) * snapStep;
    const vz = Math.round(sphere.position.z / (snapStep * VIS_SCALE)) * snapStep;

    sphere.position.set(vx * VIS_SCALE, vy * VIS_SCALE, vz * VIS_SCALE);
    conn.control_points[pointIndex][0] = vx;
    conn.control_points[pointIndex][1] = vy;
    conn.control_points[pointIndex][2] = vz;
  } 
  // If we are in scale mode, translate scale to radius_scale
  else if (transformControls.mode === 'scale') {
    // Keep scale uniform and save to control point
    const radiusScale = sphere.scale.x;
    sphere.scale.set(radiusScale, radiusScale, radiusScale);
    conn.control_points[pointIndex][3] = radiusScale;
  }

  // Re-align orientations of ALL control spheres along the new curve
  const M = conn.control_points.length;
  for (let k = 0; k < M; k++) {
    const sp = controlSpheres[k];
    if (sp) {
      const tangent = computeTangentAtNode(conn.control_points, k);
      sp.quaternion.setFromUnitVectors(new THREE.Vector3(0, 0, 1), tangent);
    }
  }

  // Update placementData and redraw routes
  store.set('placementData', placementData);
  const routes = store.get('routesData') || [];
  drawRoutes(routes);
}

/**
 * Handle end of dragging for history recording
 */
export function handleDragEnd(sphere, undoState) {
  const placementData = store.get('placementData');
  if (!placementData) return;

  const conn = placementData.connections.find(c => 
    `${c.from}.${c.from_socket}→${c.to}.${c.to_socket}` === sphere.userData.connectionKey
  );
  if (!conn) return;

  const redoState = JSON.parse(JSON.stringify(conn));
  
  import('../store/history_manager.js').then(({ historyManager }) => {
    historyManager.pushAction(
      'move',
      'connection',
      sphere.userData.connectionKey,
      `Редактирование кабеля ${sphere.userData.connectionKey}`,
      undoState,
      redoState
    );
  });
}
