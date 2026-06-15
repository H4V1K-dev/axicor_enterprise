/**
 * @fileoverview route_renderer.js — Draws connecting 3D curves and lines between socket pins in simple, pin, or pixel routing modes.
 */

import * as THREE from 'three';
import { scene } from '../viewer.js';
import { routeAllCables } from '../cable_router.js';
import { shardMeshes, socketMeshes, VIS_SCALE } from '../scene_builder.js';
import { store } from '../store/store.js';

export let routesGroup = null;

/**
 * Maximum number of pin-to-pin fibers rendered per connection in Mode 3.
 * Sockets larger than this are subsampled to keep the frame budget.
 */
const MAX_FIBERS_PER_CONN = 512;

/**
 * Generates custom variable-radius rectangular geometry along a curve.
 * @param {Array<Array<number>>} controlPoints - Control points with [x, y, z, radius_scale]
 * @param {object} socketFrom - Source socket metadata
 * @param {object} socketTo - Target socket metadata
 * @param {number} tubularSegments - Segment count
 */
export function createVariableRadiusTubeGeometry(controlPoints, socketFrom, socketTo, tubularSegments = 64) {
  const points = controlPoints.map(pt => new THREE.Vector3(pt[0] * VIS_SCALE, pt[1] * VIS_SCALE, pt[2] * VIS_SCALE));
  const curve = new THREE.CatmullRomCurve3(points);
  
  const frames = curve.computeFrenetFrames(tubularSegments, false);
  const tangents = frames.tangents;
  const normals = frames.normals;
  const binormals = frames.binormals;
  
  const vertices = [];
  const indices = [];
  const uvs = [];
  
  const wFrom = socketFrom ? socketFrom.width : 4;
  const hFrom = socketFrom ? socketFrom.height : 4;
  const pitchFrom = socketFrom ? socketFrom.pitch : 2;
  
  const wTo = socketTo ? socketTo.width : 4;
  const hTo = socketTo ? socketTo.height : 4;
  const pitchTo = socketTo ? socketTo.pitch : 2;

  // Generate vertices
  for (let i = 0; i <= tubularSegments; i++) {
    const u = i / tubularSegments;
    const p = curve.getPoint(u);
    const N = normals[i];
    const B = binormals[i];
    
    // Interpolate control point radius scale
    const idxFloat = u * (controlPoints.length - 1);
    const idx0 = Math.floor(idxFloat);
    const idx1 = Math.min(controlPoints.length - 1, idx0 + 1);
    const frac = idxFloat - idx0;
    const scale0 = controlPoints[idx0][3] !== undefined ? controlPoints[idx0][3] : 1.0;
    const scale1 = controlPoints[idx1][3] !== undefined ? controlPoints[idx1][3] : 1.0;
    const radiusScale = scale0 * (1 - frac) + scale1 * frac;
    
    // Interpolate socket dimensions
    const wInterp = wFrom * (1 - u) + wTo * u;
    const hInterp = hFrom * (1 - u) + hTo * u;
    const pitchInterp = pitchFrom * (1 - u) + pitchTo * u;
    
    const wWorld = wInterp * pitchInterp * VIS_SCALE;
    const hWorld = hInterp * pitchInterp * VIS_SCALE;
    
    const halfW = (wWorld / 2) * radiusScale;
    const halfH = (hWorld / 2) * radiusScale;
    
    // 5 vertices for rectangular cross section (last is same as first for seam)
    const offsets = [
      new THREE.Vector3().addScaledVector(N, halfW).addScaledVector(B, halfH),
      new THREE.Vector3().addScaledVector(N, -halfW).addScaledVector(B, halfH),
      new THREE.Vector3().addScaledVector(N, -halfW).addScaledVector(B, -halfH),
      new THREE.Vector3().addScaledVector(N, halfW).addScaledVector(B, -halfH),
    ];
    offsets.push(offsets[0].clone()); // seam
    
    for (let j = 0; j < 5; j++) {
      const v = p.clone().add(offsets[j]);
      vertices.push(v.x, v.y, v.z);
      uvs.push(u, j / 4);
    }
  }
  
  // Generate indices for the tube walls
  for (let i = 0; i < tubularSegments; i++) {
    for (let j = 0; j < 4; j++) {
      const a = i * 5 + j;
      const b = i * 5 + (j + 1);
      const c = (i + 1) * 5 + j;
      const d = (i + 1) * 5 + (j + 1);
      
      indices.push(a, c, b);
      indices.push(b, c, d);
    }
  }
  
  // End caps
  // Start cap (i = 0)
  const startCenterIdx = vertices.length / 3;
  const startPt = curve.getPoint(0);
  vertices.push(startPt.x, startPt.y, startPt.z);
  uvs.push(0, 0.5);
  for (let j = 0; j < 4; j++) {
    const a = j;
    const b = j + 1;
    indices.push(startCenterIdx, b, a); // Facing outwards at start
  }
  
  // End cap (i = tubularSegments)
  const endCenterIdx = vertices.length / 3;
  const endPt = curve.getPoint(1);
  vertices.push(endPt.x, endPt.y, endPt.z);
  uvs.push(1, 0.5);
  const baseOffset = tubularSegments * 5;
  for (let j = 0; j < 4; j++) {
    const a = baseOffset + j;
    const b = baseOffset + j + 1;
    indices.push(endCenterIdx, a, b); // Facing outwards at end
  }
  
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute('position', new THREE.Float32BufferAttribute(vertices, 3));
  geometry.setAttribute('uv', new THREE.Float32BufferAttribute(uvs, 2));
  geometry.setIndex(indices);
  geometry.computeVertexNormals();
  
  return geometry;
}

/**
 * Draws all cables and connections in the 3D scene based on selected routing mode.
 * @param {Array<import("../contracts/types.js").Route>} routes 
 */
export function drawRoutes(routes) {
  // Force Three.js to update all world matrices
  scene.updateMatrixWorld(true);

  // Clear existing routes
  if (routesGroup) scene.remove(routesGroup);
  
  const editorSettings = store.get('editorSettings') || {};
  if (editorSettings.show_connections === false) {
    routesGroup = null;
    return;
  }
  
  routesGroup = new THREE.Group();
  scene.add(routesGroup);

  const placementData = store.get('placementData');
  if (!placementData || !placementData.connections) return;

  const selShardKey = store.get('selectedShardKey');
  const selSocketKey = store.get('selectedSocketKey');
  const isAnySelected = (selShardKey || selSocketKey);
  const mode = store.get('connectionMode') || 1;

  console.time(`drawRoutes mode=${mode}`);

  // Fetch routes lookup map for faster checking
  const routesMap = new Map();
  routes.forEach(r => {
    const key1 = `${r.from}.${r.from_socket}→${r.to}.${r.to_socket}`;
    const key2 = `${r.to}.${r.to_socket}→${r.from}.${r.from_socket}`;
    routesMap.set(key1, r);
    routesMap.set(key2, r);
  });

  // Dynamic routed curves calculation for Mode 2 & Mode 3 (only automatic non-manual routes)
  let routedCurves = null;
  if (mode === 2 || mode === 3) {
    const autoRoutes = routes.filter(r => {
      const conn = placementData.connections.find(c => 
        (c.from === r.from && c.from_socket === r.from_socket && c.to === r.to && c.to_socket === r.to_socket) ||
        (c.from === r.to && c.from_socket === r.to_socket && c.to === r.from && c.to_socket === r.from_socket)
      );
      return conn && !conn.manual;
    });

    const segmentsCount = (mode === 3) ? 16 : 60;
    routedCurves = routeAllCables(autoRoutes, shardMeshes, socketMeshes, VIS_SCALE, {
      numCurvePoints: segmentsCount,
      tractRadius: 25.0,
      tractStrength: 0.25,
      avoidanceBuffer: 5.0
    });
  }

  // Draw each connection in placementData.connections
  placementData.connections.forEach(conn => {
    // Determine active highlight state
    let isActive = true;
    const socketFromKey = `${conn.from}.${conn.from_socket}`;
    const socketToKey = `${conn.to}.${conn.to_socket}`;

    if (isAnySelected) {
      if (selSocketKey) {
        isActive = (socketFromKey === selSocketKey || socketToKey === selSocketKey);
      } else if (selShardKey) {
        isActive = (conn.from === selShardKey || conn.to === selShardKey);
      }
    }

    const opacity = isActive ? 0.9 : 0.05;
    const color = isActive ? 0x8b9cf7 : 0x444455;

    const socketFrom = socketMeshes[socketFromKey];
    const socketTo = socketMeshes[socketToKey];

    // Case A: Connection is marked as manual with custom control points
    if (conn.manual && conn.control_points && conn.control_points.length >= 2) {
      const geom = createVariableRadiusTubeGeometry(
        conn.control_points,
        socketFrom ? socketFrom.userData : null,
        socketTo ? socketTo.userData : null
      );
      const mat = new THREE.MeshStandardMaterial({
        color,
        roughness: 0.35,
        metalness: 0.1,
        transparent: true,
        opacity: opacity,
      });
      const routeMesh = new THREE.Mesh(geom, mat);
      routeMesh.userData = {
        isRoute: true,
        routeKey: `${socketFromKey}→${socketToKey}`,
        routeData: conn
      };
      routesGroup.add(routeMesh);
    } 
    // Case B: Connection is automatic, draw based on visual mode
    else {
      const route = routesMap.get(`${socketFromKey}→${socketToKey}`);
      if (!route) return;

      if (mode === 1) {
        // Mode 1: simple graph lines using route.points
        if (!route.points || route.points.length < 2) return;
        const points = route.points.map(pt => new THREE.Vector3(pt[0] * VIS_SCALE, pt[1] * VIS_SCALE, pt[2] * VIS_SCALE));
        const curve = new THREE.CatmullRomCurve3(points);
        const curvePoints = curve.getPoints(60);
        const geometry = new THREE.BufferGeometry().setFromPoints(curvePoints);
        const material = new THREE.LineBasicMaterial({
          color, transparent: true, opacity, blending: THREE.AdditiveBlending
        });
        const lineMesh = new THREE.Line(geometry, material);
        lineMesh.userData = {
          isRoute: true,
          routeKey: `${socketFromKey}→${socketToKey}`,
          routeData: conn
        };
        routesGroup.add(lineMesh);
      } 
      else if (mode === 2) {
        // Mode 2: routed curves between socket centers
        const routeKey = `${socketFromKey}→${socketToKey}`;
        const curvePoints = routedCurves ? routedCurves.get(routeKey) : null;
        if (!curvePoints) return;

        const geometry = new THREE.BufferGeometry().setFromPoints(curvePoints);
        const material = new THREE.LineBasicMaterial({
          color, transparent: true, opacity, blending: THREE.AdditiveBlending
        });
        const lineMesh = new THREE.Line(geometry, material);
        lineMesh.userData = {
          isRoute: true,
          routeKey: `${socketFromKey}→${socketToKey}`,
          routeData: conn
        };
        routesGroup.add(lineMesh);
      } 
      else if (mode === 3) {
        // Mode 3: lines between individual corresponding pins
        if (!socketFrom || !socketTo) return;
        const fromMesh = shardMeshes[conn.from];
        const toMesh = shardMeshes[conn.to];
        if (!fromMesh || !toMesh) return;

        const routeKey = `${socketFromKey}→${socketToKey}`;
        const backbonePoints = routedCurves ? routedCurves.get(routeKey) : null;
        if (!backbonePoints || backbonePoints.length < 2) return;

        const wFrom = socketFrom.userData.width;
        const hFrom = socketFrom.userData.height;
        const wTo = socketTo.userData.width;
        const hTo = socketTo.userData.height;
        const spacingFrom = VIS_SCALE * socketFrom.userData.pitch;
        const spacingTo = VIS_SCALE * socketTo.userData.pitch;
        const fsFrom = socketFrom.userData.faceSign;
        const fsTo = socketTo.userData.faceSign;

        const socketCenter0 = new THREE.Vector3();
        const socketCenter1 = new THREE.Vector3();
        socketFrom.getWorldPosition(socketCenter0);
        socketTo.getWorldPosition(socketCenter1);

        const totalPins = wFrom * hFrom;
        let stepC = 1;
        let stepR = 1;
        if (totalPins > MAX_FIBERS_PER_CONN) {
          const ratio = Math.sqrt(totalPins / MAX_FIBERS_PER_CONN);
          stepC = Math.max(1, Math.ceil(ratio));
          stepR = Math.max(1, Math.ceil(ratio));
          while (Math.ceil(hFrom / stepR) * Math.ceil(wFrom / stepC) > MAX_FIBERS_PER_CONN) {
            if (stepC <= stepR) stepC++;
            else stepR++;
          }
        }

        const linePoints = [];
        const L = backbonePoints.length;

        for (let r = 0; r < hFrom; r += stepR) {
          for (let c = 0; c < wFrom; c += stepC) {
            const u = wFrom > 1 ? c / (wFrom - 1) : 0.5;
            const v = hFrom > 1 ? r / (hFrom - 1) : 0.5;

            const targetC = wTo > 1 ? Math.round(u * (wTo - 1)) : 0;
            const targetR = hTo > 1 ? Math.round(v * (hTo - 1)) : 0;

            const lfX = (c - (wFrom - 1) / 2) * spacingFrom;
            const lfY = (r - (hFrom - 1) / 2) * spacingFrom;
            const p0 = new THREE.Vector3(lfX, lfY, fsFrom * 0.1).applyMatrix4(socketFrom.matrixWorld);

            const ltX = (targetC - (wTo - 1) / 2) * spacingTo;
            const ltY = (targetR - (hTo - 1) / 2) * spacingTo;
            const p3 = new THREE.Vector3(ltX, ltY, fsTo * 0.1).applyMatrix4(socketTo.matrixWorld);

            const offset0 = p0.clone().sub(socketCenter0);
            const offset1 = p3.clone().sub(socketCenter1);

            const pinPoints = [];
            for (let i = 0; i < L; i++) {
              const t = i / (L - 1);
              const pt = backbonePoints[i].clone()
                .addScaledVector(offset0, 1 - t)
                .addScaledVector(offset1, t);
              pinPoints.push(pt);
            }

            for (let i = 0; i < L - 1; i++) {
              linePoints.push(pinPoints[i], pinPoints[i+1]);
            }
          }
        }

        if (linePoints.length > 0) {
          const geometry = new THREE.BufferGeometry().setFromPoints(linePoints);
          const material = new THREE.LineBasicMaterial({
            color, transparent: true,
            opacity: isActive ? opacity * 0.5 : opacity,
            blending: THREE.AdditiveBlending
          });
          const lineMesh = new THREE.LineSegments(geometry, material);
          lineMesh.userData = {
            isRoute: true,
            routeKey: `${socketFromKey}→${socketToKey}`,
            routeData: conn
          };
          routesGroup.add(lineMesh);
        }
      }
    }
  });

  console.timeEnd(`drawRoutes mode=${mode}`);
}
