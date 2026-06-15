import * as THREE from 'three';
import { scene, fitCameraToScene } from './viewer.js';
import { store } from './store/store.js';
import { 
  initSharedResources, 
  createLayerPlane, 
  makeTextSprite, 
  rebuildSocket
} from './rendering/mesh_factory.js';
import { drawRoutes } from './rendering/route_renderer.js';

// Configuration
export const ORBIT_COLORS = [
  0xef4444,  // L0 — Core
  0xf59e0b,  // L1 — Inner
  0x10b981,  // L2 — Mid
  0x6366f1,  // L3 — Outer
];
export const ORBIT_LABELS = ['Core', 'Inner', 'Mid', 'Outer'];

// Scene elements tracking
export let shardMeshes = {};        // key -> mesh
export let shardDataMap = {};       // mesh.uuid -> raw data
export let socketMeshes = {};       // socketKey -> THREE.Group containing instanced mesh & backing
export let VIS_SCALE = 1.0;

export const SOMA_COLORS = {
  "bio/sensory/photoreceptor": 0x38bdf8,
  "bio/sensory/hair_cell": 0xfb7185,
  "bio/cortex/pyramidal_exc": 0x34d399,
  "bio/cortex/basket_inh": 0xfbbf24,
  "bio/motor/purkinje": 0xa78bfa,
  "bio/motor/spinal_motor": 0xf472b6
};

// Reusable groups to prevent memory leaks and duplicate objects
let planesGroup = null;

// Re-export drawRoutes and rebuildSocket for consumer modules
export { drawRoutes, rebuildSocket };

/**
 * Dynamically toggles visibility of a flat layer plane in the 3D scene.
 * @param {number} orbitIndex 
 * @param {boolean} visible 
 */
export function setLayerPlaneVisibility(orbitIndex, visible) {
  const layersVis = store.get('layersVisibility') || {};
  layersVis[orbitIndex] = visible;
  store.set('layersVisibility', layersVis);

  if (planesGroup) {
    planesGroup.children.forEach(child => {
      if (child.userData && child.userData.orbitIndex === orbitIndex) {
        child.visible = visible;
      }
    });
  }
}

/**
 * Builds the 3D visual scene objects (layer planes, shards, and socket groups) from placement data.
 * @param {import("./contracts/types.js").PlacementData} data 
 */
export function buildSceneData(data, preserveCamera = false) {
  // Clear any existing groups to avoid duplication
  if (planesGroup) scene.remove(planesGroup);

  planesGroup = new THREE.Group();
  scene.add(planesGroup);

  shardMeshes = {};
  shardDataMap = {};
  socketMeshes = {};
  
  const maxRadius = Math.max(...data.orbits.map(o => o.radius), 1.0);
  VIS_SCALE = 35.0 / maxRadius;
  initSharedResources(VIS_SCALE);

  const outgoingConnections = new Map();
  const incomingConnections = new Map();
  data.connections.forEach(conn => {
    outgoingConnections.set(`${conn.from}.${conn.from_socket}`, conn.to);
    incomingConnections.set(`${conn.to}.${conn.to_socket}`, conn.from);
  });

  const getSocketFaceSign = (shardKey, socketName, shardOrbit) => {
    const sockKey = `${shardKey}.${socketName}`;
    if (incomingConnections.has(sockKey)) {
      const sourceKey = incomingConnections.get(sockKey);
      const sourceShard = data.shards.find(s => s.key === sourceKey);
      const sourceOrbit = sourceShard ? sourceShard.orbit : shardOrbit;
      return sourceOrbit < shardOrbit ? -1 : 1;
    } else if (outgoingConnections.has(sockKey)) {
      const targetKey = outgoingConnections.get(sockKey);
      const targetShard = data.shards.find(s => s.key === targetKey);
      const targetOrbit = targetShard ? targetShard.orbit : shardOrbit;
      return targetOrbit < shardOrbit ? -1 : 1;
    }
    return 1;
  };

  // 1. Draw flat layer planes inside planesGroup
  const layersVis = store.get('layersVisibility') || {};
  data.orbits.forEach(orb => {
    const color = ORBIT_COLORS[orb.index] || 0x888888;
    const w = (orb.w || 200) * VIS_SCALE;
    const d = (orb.d || 200) * VIS_SCALE;
    const yPos = orb.radius * VIS_SCALE;
    const plane = createLayerPlane(w, d, yPos, color);
    plane.userData = { orbitIndex: orb.index };
    
    // Restore visibility
    if (layersVis[orb.index] !== undefined) {
      plane.visible = layersVis[orb.index];
    } else {
      plane.visible = true;
      layersVis[orb.index] = true;
    }
    
    planesGroup.add(plane);
  });
  store.set('layersVisibility', layersVis);

  // 2. Build shards nested inside their respective level plane groups
  data.shards.forEach(sd => {
    const orb = data.orbits.find(o => o.index === sd.orbit);
    const radius = orb ? orb.radius : 0.0;

    const color = ORBIT_COLORS[sd.orbit] || 0x888888;
    const x = sd.position.x * VIS_SCALE;
    const y = (sd.position.y - radius) * VIS_SCALE; // local height relative to level floor
    const z = sd.position.z * VIS_SCALE;
    const w = sd.size.w * VIS_SCALE;
    const d = sd.size.d * VIS_SCALE;
    const h = sd.size.h * VIS_SCALE;

    // Shard main mesh (rendered as monolith by default)
    const geo = new THREE.BoxGeometry(w, d, h);
    const mat = new THREE.MeshStandardMaterial({
      color, transparent: false, opacity: 1.0,
      roughness: 0.6, metalness: 0.1,
    });
    const mesh = new THREE.Mesh(geo, mat);
    mesh.position.set(x, y, z);
    
    if (sd.quaternion) {
      mesh.quaternion.set(sd.quaternion.x, sd.quaternion.y, sd.quaternion.z, sd.quaternion.w);
    } else {
      mesh.rotation.set(0, 0, 0);
    }

    // Add mesh to corresponding level plane group
    const levelGroup = planesGroup.children.find(c => c.userData && c.userData.orbitIndex === sd.orbit);
    if (levelGroup) {
      levelGroup.add(mesh);
    } else {
      scene.add(mesh);
    }

    // Monolith wireframe (visible when not selected)
    const edgeGeo = new THREE.EdgesGeometry(geo);
    const edgeMat = new THREE.LineBasicMaterial({
      color, transparent: true, opacity: 0.85,
    });
    const mainWire = new THREE.LineSegments(edgeGeo, edgeMat);
    mainWire.name = "main_wireframe";
    mesh.add(mainWire);

    // Label added as child of mesh
    const label = makeTextSprite(sd.key, color);
    label.position.set(0, 0, h / 2 + 1.5);
    mesh.add(label);

    // Track for rendering and raycasting
    shardMeshes[sd.key] = mesh;
    shardDataMap[mesh.uuid] = sd;

    // Store references in mesh.userData
    mesh.userData = { label, originalColor: color };

    // Build layers as child meshes of the main shard mesh
    const layers = sd.layers && sd.layers.length > 0
      ? sd.layers
      : [{ name: "default", height_pct: 1.0, density: 1.0 }];

    let currentZ = -h / 2;
    layers.forEach((layer, idx) => {
      const layer_vis_h = h * layer.height_pct;
      const layerGeo = new THREE.BoxGeometry(w, d, 1.0);
      
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
      const zCenter = currentZ + layer_vis_h / 2;
      layerMesh.position.set(0, 0, zCenter);
      layerMesh.scale.set(1.0, 1.0, layer_vis_h);
      layerMesh.visible = false;

      layerMesh.userData = {
        layerName: layer.name,
        height_pct: layer.height_pct,
        layerIndex: idx,
        shardKey: sd.key
      };

      mesh.add(layerMesh);

      // Wired border for the layer
      const edgeGeo = new THREE.EdgesGeometry(layerGeo);
      const edgeMat = new THREE.LineBasicMaterial({
        color: layerColor,
        transparent: true,
        opacity: 0.8,
      });
      const layerWireframe = new THREE.LineSegments(edgeGeo, edgeMat);
      layerWireframe.name = "wireframe";
      layerMesh.add(layerWireframe);

      currentZ += layer_vis_h;
    });

    // Build interactive layer dividers if layers >= 2
    if (layers.length >= 2) {
      let accumZ = -h / 2;
      for (let i = 0; i < layers.length - 1; i++) {
        accumZ += h * layers[i].height_pct;

        const divGeo = new THREE.PlaneGeometry(w * 1.02, d * 1.02);
        const divMat = new THREE.MeshBasicMaterial({
          color: 0xffaa00,
          transparent: true,
          opacity: 0.0,
          side: THREE.DoubleSide,
          depthWrite: false
        });
        const divMesh = new THREE.Mesh(divGeo, divMat);
        divMesh.position.set(0, 0, accumZ);
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
          opacity: 0.3
        });
        const divBorder = new THREE.Line(borderGeo, borderMat);
        divBorder.name = "border";
        divMesh.add(divBorder);

        divMesh.visible = false;
        mesh.add(divMesh);
      }
    }

    // Sockets splitting by face (Top/Bottom)
    const topSockets = [];
    const bottomSockets = [];

    if (sd.sockets) {
      sd.sockets.forEach(sock => {
        const faceSign = (sock.faceSign !== undefined && sock.faceSign !== null)
          ? sock.faceSign
          : getSocketFaceSign(sd.key, sock.name, sd.orbit);
        
        if (faceSign === 1) {
          topSockets.push({ sock, faceSign });
        } else {
          bottomSockets.push({ sock, faceSign });
        }
      });
    }

    // Build sockets using rebuildSocket from mesh_factory (supports GLB pin models)
    const buildSocketViaFactory = (sock, faceSign) => {
      const offset = sock.offset
        ? { x: sock.offset.x, y: sock.offset.y, z: sock.offset.z }
        : { x: 0, y: 0 };
      const rotation = sock.rotation || 0;
      rebuildSocket(sd.key, sock.name, sock.width, sock.height, sock.pitch || 1, offset, faceSign, rotation);
    };

    // Build Top face sockets
    topSockets.forEach(item => {
      buildSocketViaFactory(item.sock, item.faceSign);
    });

    // Build Bottom face sockets
    bottomSockets.forEach(item => {
      buildSocketViaFactory(item.sock, item.faceSign);
    });
  });

  // 3. Compute auto-camera fit
  const bbox = new THREE.Box3();
  data.shards.forEach(sd => {
    const x = sd.position.x * VIS_SCALE;
    const y = sd.position.y * VIS_SCALE;
    const z = sd.position.z * VIS_SCALE;
    const w = sd.size.w * VIS_SCALE;
    const d = sd.size.d * VIS_SCALE;
    const h = sd.size.h * VIS_SCALE;
    bbox.expandByPoint(new THREE.Vector3(x - w/2, y - h/2, z - d/2));
    bbox.expandByPoint(new THREE.Vector3(x + w/2, y + h/2, z + d/2));
  });

  if (!preserveCamera) {
    fitCameraToScene(bbox);
  }
}

export function updateAllSocketVisuals() {
  const placementData = store.get('placementData');
  if (!placementData) return;

  placementData.shards.forEach(shard => {
    (shard.sockets || []).forEach(sock => {
      const socketKey = `${shard.key}.${sock.name}`;
      const group = socketMeshes[socketKey];
      if (group) {
        rebuildSocket(
          shard.key,
          sock.name,
          group.userData.width,
          group.userData.height,
          group.userData.pitch,
          group.userData.originalOffset,
          group.userData.faceSign,
          group.userData.rotation
        );
      }
    });
  });
}
