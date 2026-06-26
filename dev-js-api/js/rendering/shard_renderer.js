import * as THREE from 'three';
import { makeTextSprite } from './mesh_factory.js';
import { THEME, RENDER_BINS } from './theme.js';

/**
 * Creates a THREE.Group representation of a shard with body, wireframe, label, layers and dividers.
 * @param {any} sd - Raw shard data
 * @param {number} visScale - Visualization scale factor
 * @returns {THREE.Group}
 */
export function createShard3D(sd, visScale) {
  const color = 0x6366f1; // Single beautiful Indigo theme color

  // Position is center in Three.js coordinates
  const x = (sd.position.x + sd.size.w / 2) * visScale;
  const y = (sd.position.y + sd.size.h / 2) * visScale;
  const z = (sd.position.z + sd.size.d / 2) * visScale;

  const w = sd.size.w * visScale;
  const d = sd.size.d * visScale; // Three.js D (depth)
  const h = sd.size.h * visScale; // Three.js H (height)

  const shardGroup = new THREE.Group();
  shardGroup.position.set(x, y, z);
  shardGroup.name = `shard_${sd.key}`;

  // Body mesh
  const geo = new THREE.BoxGeometry(w, h, d);
  const mat = new THREE.MeshStandardMaterial({
    color, transparent: false, opacity: 1.0,
    roughness: 0.6, metalness: 0.1,
  });
  const boxMesh = new THREE.Mesh(geo, mat);
  boxMesh.name = "body";
  shardGroup.add(boxMesh);

  // Wireframe
  const edgeGeo = new THREE.EdgesGeometry(geo);
  const edgeMat = new THREE.LineBasicMaterial({
    color, transparent: true, opacity: 0.85,
  });
  const mainWire = new THREE.LineSegments(edgeGeo, edgeMat);
  mainWire.name = "main_wireframe";
  shardGroup.add(mainWire);

  // Text label
  const label = makeTextSprite(sd.key, color);
  label.position.set(0, h / 2 + 1.5, 0);
  shardGroup.add(label);

  // Track key references in group's userData
  shardGroup.userData = {
    label,
    originalColor: color,
    body: boxMesh,
    mainWire: mainWire,
    shardKey: sd.key
  };

  // Build internal layers
  const layers = sd.layers && sd.layers.length > 0
    ? sd.layers
    : [{ name: "default", height_pct: 1.0, density: 1.0 }];

  let currentY = -h / 2;
  layers.forEach((layer, idx) => {
    const layer_vis_h = h * layer.height_pct;
    // Micro-gap to prevent Z-fighting at layer boundaries
    const layer_draw_h = Math.max(0.01 * visScale, layer_vis_h - 0.02 * visScale);
    const layerGeo = new THREE.BoxGeometry(w, 1.0, d);
    
    const layerColor = new THREE.Color(color);
    if (idx % 2 === 1) {
      layerColor.offsetHSL(0, 0, -0.08);
    } else {
      layerColor.offsetHSL(0, 0, 0.04);
    }

    const layerMat = new THREE.MeshStandardMaterial({
      color: layerColor,
      transparent: true,
      opacity: 0.5,
      roughness: 0.15,
      metalness: 0.1,
    });

    const layerMesh = new THREE.Mesh(layerGeo, layerMat);
    const yCenter = currentY + layer_vis_h / 2;
    layerMesh.position.set(0, yCenter, 0);
    layerMesh.scale.set(1.0, layer_draw_h, 1.0);
    layerMesh.visible = false;
    layerMesh.renderOrder = RENDER_BINS.activeLayers + idx;

    layerMesh.userData = {
      layerName: layer.name,
      height_pct: layer.height_pct,
      layerIndex: idx,
      shardKey: sd.key
    };

    shardGroup.add(layerMesh);

    const layerEdgeGeo = new THREE.EdgesGeometry(layerGeo);
    const layerEdgeMat = new THREE.LineBasicMaterial({
      color: layerColor,
      transparent: true,
      opacity: 0.8,
      polygonOffset: true,
      polygonOffsetFactor: -1,
      polygonOffsetUnits: -1
    });
    const layerWireframe = new THREE.LineSegments(layerEdgeGeo, layerEdgeMat);
    layerWireframe.name = "wireframe";
    layerMesh.add(layerWireframe);

    currentY += layer_vis_h;
  });

  // Build layer dividers
  if (layers.length >= 2) {
    let accumY = -h / 2;
    for (let i = 0; i < layers.length - 1; i++) {
      accumY += h * layers[i].height_pct;

      const divGeo = new THREE.PlaneGeometry(w * 1.02, d * 1.02);
      const divMat = new THREE.MeshBasicMaterial({
        color: 0xffaa00,
        transparent: true,
        opacity: 0.0,
        side: THREE.DoubleSide,
        depthWrite: false,
        polygonOffset: true,
        polygonOffsetFactor: -1,
        polygonOffsetUnits: -1
      });
      const divMesh = new THREE.Mesh(divGeo, divMat);
      divMesh.position.set(0, accumY, 0);
      divMesh.rotation.x = -Math.PI / 2;
      divMesh.name = `divider_${i}`;
      divMesh.userData = {
        isDivider: true,
        dividerIndex: i,
        shardKey: sd.key
      };

      const borderGeo = new THREE.BufferGeometry().setFromPoints([
        new THREE.Vector3(-w/2, -d/2, 0),
        new THREE.Vector3(w/2, -d/2, 0),
        new THREE.Vector3(w/2, d/2, 0),
        new THREE.Vector3(-w/2, d/2, 0),
        new THREE.Vector3(-w/2, -d/2, 0)
      ]);
      const borderMat = new THREE.LineBasicMaterial({
        color: 0xffaa00,
        transparent: true,
        opacity: 0.3,
        polygonOffset: true,
        polygonOffsetFactor: -1,
        polygonOffsetUnits: -1
      });
      const divBorder = new THREE.Line(borderGeo, borderMat);
      divBorder.name = "border";
      divMesh.add(divBorder);

      divMesh.visible = false;
      shardGroup.add(divMesh);
    }
  }

  return shardGroup;
}

/**
 * Updates a shard's geometry and internal layers/dividers locally in real-time.
 * @param {THREE.Group} shardGroup 
 * @param {Object} position 
 * @param {Object} size 
 * @param {number} visScale 
 */
export function updateShardTransform(shardGroup, position, size, visScale) {
  const w = size.w * visScale;
  const h = size.h * visScale;
  const d = size.d * visScale;

  const x = (position.x + size.w / 2) * visScale;
  const y = (position.y + size.h / 2) * visScale;
  const z = (position.z + size.d / 2) * visScale;

  // Move the container group (automatically shifts children)
  shardGroup.position.set(x, y, z);

  // Resize body BoxGeometry
  const boxMesh = shardGroup.userData.body;
  if (boxMesh) {
    boxMesh.geometry.dispose();
    boxMesh.geometry = new THREE.BoxGeometry(w, h, d);
  }

  // Resize main wireframe
  const mainWire = shardGroup.userData.mainWire;
  if (mainWire && boxMesh) {
    mainWire.geometry.dispose();
    mainWire.geometry = new THREE.EdgesGeometry(boxMesh.geometry);
  }

  // Adjust label position
  const label = shardGroup.userData.label;
  if (label) {
    label.position.set(0, h / 2 + 1.5, 0);
  }

  // Scale and shift horizontal layers
  const layerMeshes = [];
  shardGroup.children.forEach(child => {
    if (child.userData && child.userData.layerIndex !== undefined) {
      layerMeshes.push(child);
    }
  });
  layerMeshes.sort((a, b) => a.userData.layerIndex - b.userData.layerIndex);

  let currentY = -h / 2;
  layerMeshes.forEach(layerMesh => {
    const layer_vis_h = h * layerMesh.userData.height_pct;
    const yCenter = currentY + layer_vis_h / 2;
    // Micro-gap to prevent Z-fighting at layer boundaries
    const layer_draw_h = Math.max(0.01 * visScale, layer_vis_h - 0.02 * visScale);

    layerMesh.position.set(0, yCenter, 0);
    layerMesh.scale.set(1.0, layer_draw_h, 1.0);

    layerMesh.geometry.dispose();
    layerMesh.geometry = new THREE.BoxGeometry(w, 1.0, d);

    const wire = layerMesh.children.find(c => c.name === "wireframe");
    if (wire) {
      wire.geometry.dispose();
      wire.geometry = new THREE.EdgesGeometry(layerMesh.geometry);
    }

    currentY += layer_vis_h;
  });

  // Scale and shift dividers
  const dividers = [];
  shardGroup.children.forEach(child => {
    if (child.userData && child.userData.isDivider) {
      dividers.push(child);
    }
  });
  dividers.sort((a, b) => a.userData.dividerIndex - b.userData.dividerIndex);

  let accumY = -h / 2;
  dividers.forEach((divMesh, idx) => {
    accumY += h * layerMeshes[idx].userData.height_pct;
    divMesh.position.set(0, accumY, 0);

    divMesh.geometry.dispose();
    divMesh.geometry = new THREE.PlaneGeometry(w * 1.02, d * 1.02);

    const border = divMesh.children.find(c => c.name === "border");
    if (border) {
      border.geometry.dispose();
      border.geometry = new THREE.BufferGeometry().setFromPoints([
        new THREE.Vector3(-w / 2, -d / 2, 0),
        new THREE.Vector3(w / 2, -d / 2, 0),
        new THREE.Vector3(w / 2, d / 2, 0),
        new THREE.Vector3(-w / 2, d / 2, 0),
        new THREE.Vector3(-w / 2, -d / 2, 0)
      ]);
    }
  });
}

/**
 * Updates a shard's position during active drag without altering store state.
 * @param {THREE.Group} shardGroup 
 * @param {Object} position 
 * @param {Object} sd - Shard raw data (size etc.)
 * @param {number} visScale 
 */
export function updateShardDragging(shardGroup, position, sd, visScale) {
  const x = (position.x + sd.size.w / 2) * visScale;
  const y = (position.y + sd.size.h / 2) * visScale;
  const z = (position.z + sd.size.d / 2) * visScale;

  shardGroup.position.set(x, y, z);
}
