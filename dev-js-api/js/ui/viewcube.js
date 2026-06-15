import * as THREE from 'three';
import { GLTFLoader } from 'three/addons/loaders/GLTFLoader.js';
import { camera as mainCamera, controls, animateCameraTo, setCameraProjection } from '../viewer.js';
import { store } from '../store/store.js';

export let viewcubeScene = null;
export let viewcubeCamera = null;
export let viewcubeRenderer = null;

export function initViewCube() {
  const container = document.getElementById('viewcube-container');
  const canvas = document.getElementById('viewcube-canvas');
  if (!container || !canvas) return;

  // Set canvas dimensions matching container
  canvas.width = 140;
  canvas.height = 140;

  // 1. Scene setup
  viewcubeScene = new THREE.Scene();

  // 2. Camera setup: Orthographic for clean parallel CAD lines
  viewcubeCamera = new THREE.OrthographicCamera(-1.2, 1.2, 1.2, -1.2, 0.1, 100);
  viewcubeCamera.position.set(0, 0, 4);
  viewcubeCamera.lookAt(0, 0, 0);
  viewcubeScene.add(viewcubeCamera);

  // 3. Renderer setup
  viewcubeRenderer = new THREE.WebGLRenderer({
    canvas: canvas,
    antialias: true,
    alpha: true
  });
  viewcubeRenderer.setSize(140, 140);
  viewcubeRenderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));

  // 4. Lighting: simple head light attached to the camera, plus ambient light
  const ambientLight = new THREE.AmbientLight(0xffffff, 0.85);
  viewcubeScene.add(ambientLight);

  const dirLight = new THREE.DirectionalLight(0xffffff, 1.2);
  dirLight.position.set(2, 4, 3);
  viewcubeCamera.add(dirLight);

  // 5. Load GLTF Cube Model
  const loader = new GLTFLoader();
  const meshes = [];
  let hoveredObj = null;

  // Face labels mapping: maps Blender mesh names to determine if it is a face
  const faceNames = ['Yplus', 'Yminus', 'Xplus', 'Xminus', 'Zplus', 'Zminus'];

  loader.load('CubeGizmo.glb', (gltf) => {
    const model = gltf.scene;
    viewcubeScene.add(model);

    // Center and normalize model size to fit frustum perfectly
    const box = new THREE.Box3().setFromObject(model);
    const size = new THREE.Vector3();
    box.getSize(size);
    const maxDim = Math.max(size.x, size.y, size.z);
    const scale = 1.3 / maxDim; // scaled to occupy most of the viewport
    model.scale.setScalar(scale);

    const center = new THREE.Vector3();
    box.getCenter(center);
    model.position.sub(center.multiplyScalar(scale));

    // Configure materials, hover states, and target directions
    model.traverse((child) => {
      if (child.isMesh) {
        const isFace = faceNames.includes(child.name);
        if (isFace) {
          // Face mesh - clean grey material
          child.material = new THREE.MeshStandardMaterial({
            color: 0x4c505c, // lighter grey
            roughness: 0.4,
            metalness: 0.1
          });
          child.userData.isFace = true;
          child.userData.originalColor = 0x4c505c;
        } else {
          // Edge or Corner mesh - darker grey material
          child.material = new THREE.MeshStandardMaterial({
            color: 0x2e3037,
            roughness: 0.3,
            metalness: 0.3,
            transparent: true,
            opacity: 0.8
          });
          child.userData.isFace = false;
          child.userData.originalColor = 0x2e3037;
        }

        // Calculate physical direction vector from bounding box center in model space
        const meshBox = new THREE.Box3().setFromObject(child);
        const meshCenter = new THREE.Vector3();
        meshBox.getCenter(meshCenter);

        // Normalize direction relative to the cube center
        const dir = meshCenter.clone().normalize();
        
        // Snap direction to find target alignment coordinates (dx, dy, dz)
        const dx = Math.round(dir.x * 10) / 10;
        const dy = Math.round(dir.y * 10) / 10;
        const dz = Math.round(dir.z * 10) / 10;

        child.userData.targetDir = new THREE.Vector3(
          Math.abs(dx) > 0.35 ? Math.sign(dx) : 0,
          Math.abs(dy) > 0.35 ? Math.sign(dy) : 0,
          Math.abs(dz) > 0.35 ? Math.sign(dz) : 0
        );

        meshes.push(child);
      }
    });
  }, undefined, (err) => {
    console.error('Error loading CubeGizmo.glb:', err);
  });

  // Material hover feedback functions
  function highlightObject(obj) {
    document.body.style.cursor = 'pointer';
    obj.material.color.setHex(0x6366f1);
    obj.material.opacity = 0.95;
  }

  function resetObject(obj) {
    document.body.style.cursor = 'default';
    obj.material.color.setHex(obj.userData.originalColor);
    obj.material.opacity = obj.userData.isFace ? 1.0 : 0.8;
  }

  // 6. Raycasting for hover & click detection
  const raycaster = new THREE.Raycaster();

  function getIntersectObject(e) {
    if (meshes.length === 0) return null;
    const rect = canvas.getBoundingClientRect();
    const mouse = new THREE.Vector2(
      ((e.clientX - rect.left) / rect.width) * 2 - 1,
      -((e.clientY - rect.top) / rect.height) * 2 + 1
    );
    raycaster.setFromCamera(mouse, viewcubeCamera);
    const intersects = raycaster.intersectObjects(meshes);
    return intersects.length > 0 ? intersects[0].object : null;
  }

  canvas.addEventListener('pointermove', (e) => {
    if (isDragging) return;
    const hit = getIntersectObject(e);
    if (hit) {
      if (hoveredObj !== hit) {
        if (hoveredObj) resetObject(hoveredObj);
        hoveredObj = hit;
        highlightObject(hoveredObj);
      }
    } else {
      if (hoveredObj) {
        resetObject(hoveredObj);
        hoveredObj = null;
      }
    }
  });

  canvas.addEventListener('pointerleave', () => {
    if (hoveredObj) {
      resetObject(hoveredObj);
      hoveredObj = null;
    }
  });

  // Drag-rotation state
  let isDragging = false;
  let hasMoved = false;
  let lastMouseX = 0;
  let lastMouseY = 0;
  let distance = 0;
  let targetCenter = new THREE.Vector3();

  canvas.addEventListener('pointerdown', (e) => {
    if (e.button !== 0) return;

    isDragging = true;
    hasMoved = false;
    lastMouseX = e.clientX;
    lastMouseY = e.clientY;

    targetCenter.copy(controls.target);
    distance = mainCamera.position.distanceTo(targetCenter);

    // Switch off orthographic mode before dragging
    setCameraProjection(false);

    window.addEventListener('pointermove', onPointerMove);
    window.addEventListener('pointerup', onPointerUp);
    e.stopPropagation();
  });

  function onPointerMove(e) {
    if (!isDragging) return;

    const deltaX = e.clientX - lastMouseX;
    const deltaY = e.clientY - lastMouseY;

    if (Math.abs(deltaX) > 4 || Math.abs(deltaY) > 4) {
      hasMoved = true;
    }

    const editorSettings = store.get('editorSettings') || {};
    const sensitivity = editorSettings.viewcube_sensitivity !== undefined ? editorSettings.viewcube_sensitivity : 0.0075;
    const offset = mainCamera.position.clone().sub(targetCenter);

    // Horizontal rotation: around world Y axis (0, 1, 0)
    const theta = -deltaX * sensitivity;
    const qY = new THREE.Quaternion().setFromAxisAngle(new THREE.Vector3(0, 1, 0), theta);
    offset.applyQuaternion(qY);

    // Vertical rotation: around camera local X axis (Right vector)
    const cameraRight = new THREE.Vector3(1, 0, 0).applyQuaternion(mainCamera.quaternion).normalize();
    const phi = -deltaY * sensitivity;
    const qX = new THREE.Quaternion().setFromAxisAngle(cameraRight, phi);

    const tempOffset = offset.clone().applyQuaternion(qX);
    const angle = tempOffset.angleTo(new THREE.Vector3(0, 1, 0));

    // Clamp polar angle to avoid flipping (0.05 to PI - 0.05)
    if (angle > 0.05 && angle < Math.PI - 0.05) {
      offset.copy(tempOffset);
    }

    mainCamera.position.copy(targetCenter).add(offset);
    controls.update();

    lastMouseX = e.clientX;
    lastMouseY = e.clientY;
  }

  function onPointerUp(e) {
    if (!isDragging) return;
    isDragging = false;

    window.removeEventListener('pointermove', onPointerMove);
    window.removeEventListener('pointerup', onPointerUp);

    if (!hasMoved) {
      // Click event
      const hit = getIntersectObject(e);
      if (hit) {
        const targetDir = hit.userData.targetDir;
        const isPlane = hit.userData.isFace;
        setCameraProjection(isPlane);
        animateCameraTo(targetDir);
      }
    }

    setTimeout(() => {
      hasMoved = false;
    }, 50);
  }
}
