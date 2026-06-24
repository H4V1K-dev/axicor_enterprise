import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import { viewcubeScene, viewcubeCamera, viewcubeRenderer } from './ui/viewcube.js';

export let scene;
export let camera;
export let perspCamera;
export let orthoCamera;
export let isOrthographic = false;
const frustumSize = 300; // Frustum size matching AxiCAD typical scene scales

export let renderer;
export let controls;
export let dirLight;
export let pointLight;

export function initViewer(container) {
  // Scene
  scene = new THREE.Scene();

  // Perspective Camera
  perspCamera = new THREE.PerspectiveCamera(55, window.innerWidth / window.innerHeight, 0.1, 2000);
  perspCamera.position.set(40, 30, 40);
  perspCamera.layers.enable(1);

  // Orthographic Camera
  const aspect = window.innerWidth / window.innerHeight;
  orthoCamera = new THREE.OrthographicCamera(
    -frustumSize * aspect / 2,
    frustumSize * aspect / 2,
    frustumSize / 2,
    -frustumSize / 2,
    0.1,
    3000
  );
  orthoCamera.position.copy(perspCamera.position);
  orthoCamera.layers.enable(1);

  camera = perspCamera;

  // Renderer
  renderer = new THREE.WebGLRenderer({ antialias: true, alpha: false, preserveDrawingBuffer: true });
  renderer.setSize(window.innerWidth, window.innerHeight);
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
  renderer.setClearColor(0x1e2025);
  renderer.toneMapping = THREE.ACESFilmicToneMapping;
  renderer.toneMappingExposure = 1.25;
  container.appendChild(renderer.domElement);

  // Controls
  controls = new OrbitControls(camera, renderer.domElement);
  controls.enableDamping = true;
  controls.dampingFactor = 0.05;
  controls.minDistance = 2;
  controls.maxDistance = 1500;
  controls.target.set(0, 0, 0);

  // Configure controls: MMB to rotate, RMB to pan, disable LMB rotation to prevent misclick camera rotation
  controls.mouseButtons = {
    LEFT: THREE.MOUSE.NONE,
    MIDDLE: THREE.MOUSE.ROTATE,
    RIGHT: THREE.MOUSE.PAN
  };

  // Switch camera projection back to perspective when manually rotating (not when panning or zooming)
  controls.addEventListener('start', () => {
    if (isOrthographic && (controls.state === 0 || controls.state === 3)) {
      setCameraProjection(false);
    }
  });

  // Lights
  const ambientLight = new THREE.AmbientLight(0xffffff, 1.2);
  scene.add(ambientLight);

  dirLight = new THREE.DirectionalLight(0xc8d0ff, 1.5);
  dirLight.position.set(30, 50, 20);
  scene.add(dirLight);

  pointLight = new THREE.PointLight(0x6366f1, 1.2, 1000);
  pointLight.position.set(0, 20, 0);
  scene.add(pointLight);

  // Center Marker (a subtle grid anchor point)
  const centerGeo = new THREE.SphereGeometry(0.15, 32, 32);
  const centerMat = new THREE.MeshStandardMaterial({
    color: 0x8b9cf7,
    emissive: 0x8b9cf7,
    emissiveIntensity: 1.5,
    roughness: 0.1
  });
  const centerMesh = new THREE.Mesh(centerGeo, centerMat);
  scene.add(centerMesh);

  // Window Resize
  window.addEventListener('resize', () => {
    const width = window.innerWidth;
    const height = window.innerHeight;
    const currentAspect = width / height;

    perspCamera.aspect = currentAspect;
    perspCamera.updateProjectionMatrix();

    orthoCamera.left = -frustumSize * currentAspect / 2;
    orthoCamera.right = frustumSize * currentAspect / 2;
    orthoCamera.top = frustumSize / 2;
    orthoCamera.bottom = -frustumSize / 2;
    orthoCamera.updateProjectionMatrix();

    renderer.setSize(width, height);
  });
}

export function setCameraProjection(toOrtho) {
  if (isOrthographic === toOrtho) return;

  const target = controls.target.clone();
  const startPos = camera.position.clone();
  const distance = startPos.distanceTo(target);
  const dir = startPos.clone().sub(target).normalize();

  if (toOrtho) {
    // Switch to Orthographic
    // Calculate matching zoom to prevent abrupt jumps in object scale
    const fovRad = perspCamera.fov * (Math.PI / 180);
    const visibleHeight = 2 * distance * Math.tan(fovRad / 2);
    
    orthoCamera.zoom = frustumSize / visibleHeight;
    orthoCamera.position.copy(perspCamera.position);
    orthoCamera.quaternion.copy(perspCamera.quaternion);
    orthoCamera.updateProjectionMatrix();

    camera = orthoCamera;
    isOrthographic = true;
  } else {
    // Switch to Perspective
    // Calculate distance matching current ortho zoom scale
    const fovRad = perspCamera.fov * (Math.PI / 180);
    const visibleHeight = frustumSize / orthoCamera.zoom;
    const targetDistance = visibleHeight / (2 * Math.tan(fovRad / 2));

    perspCamera.position.copy(target).add(dir.multiplyScalar(targetDistance));
    perspCamera.quaternion.copy(orthoCamera.quaternion);
    perspCamera.updateProjectionMatrix();

    camera = perspCamera;
    isOrthographic = false;
  }

  // Update OrbitControls reference to active camera
  controls.object = camera;
  controls.update();

  // Update TransformControls camera to avoid projection mismatch during drag/resize
  import('./editor/transform.js').then(({ transformControls }) => {
    if (transformControls) {
      transformControls.camera = camera;
      transformControls.update();
    }
  }).catch(() => {});
}

export function getActiveCamera() {
  return camera;
}

export function animateViewer(onUpdate) {
  function animate() {
    requestAnimationFrame(animate);
    
    // Smooth controls damping
    controls.update();
    
    // Sync and render WebGL ViewCube
    if (viewcubeRenderer && viewcubeScene && viewcubeCamera && camera) {
      viewcubeCamera.quaternion.copy(camera.quaternion);
      viewcubeCamera.position.set(0, 0, 4).applyQuaternion(camera.quaternion);
      viewcubeRenderer.render(viewcubeScene, viewcubeCamera);
    }
    
    // Call external updates (e.g. selection animations, editor updates)
    if (onUpdate) {
      onUpdate();
    }
    
    renderer.render(scene, camera);
  }
  animate();
}

/**
 * Automatically adjusts the camera position to frame the loaded bounding box.
 */
export function fitCameraToScene(bbox) {
  const center = new THREE.Vector3();
  bbox.getCenter(center);
  const size = new THREE.Vector3();
  bbox.getSize(size);

  const maxDim = Math.max(size.x, size.y, size.z);
  
  // Position lights relative to the scene center
  dirLight.position.set(center.x + maxDim, center.y + maxDim * 1.5, center.z + maxDim);
  dirLight.lookAt(center);
  
  pointLight.position.copy(center);
  pointLight.distance = maxDim * 4.0;

  const fov = perspCamera.fov * (Math.PI / 180);
  let cameraDist = maxDim / (2 * Math.tan(fov / 2));
  cameraDist *= 1.45; // Add visual padding

  perspCamera.position.set(center.x + cameraDist * 0.8, center.y + cameraDist * 0.6, center.z + cameraDist * 0.8);
  orthoCamera.position.copy(perspCamera.position);
  
  if (isOrthographic) {
    orthoCamera.zoom = frustumSize / (maxDim * 1.45 * 2);
    orthoCamera.updateProjectionMatrix();
  }

  controls.target.copy(center);
  controls.update();
}

let cameraAnimation = null;

export function animateCameraTo(targetDirection, duration = 500) {
  if (cameraAnimation) {
    cancelAnimationFrame(cameraAnimation.id);
  }

  const startPos = camera.position.clone();
  const targetCenter = controls.target.clone();
  const distance = startPos.distanceTo(targetCenter);
  
  const startDir = startPos.clone().sub(targetCenter).normalize();
  const endDir = targetDirection.clone().normalize();

  // Handle case when startDir and endDir are exactly opposite to avoid division by zero or weird flip
  if (startDir.dot(endDir) < -0.999) {
    // Add a tiny offset to the start direction to force a rotation path
    startDir.add(new THREE.Vector3(0.01, 0.01, 0.01).normalize()).normalize();
  }

  const startTime = performance.now();

  function easeInOutCubic(t) {
    return t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2;
  }

  function tick() {
    const now = performance.now();
    const elapsed = now - startTime;
    const progress = Math.min(elapsed / duration, 1);
    const t = easeInOutCubic(progress);

    const currentDir = new THREE.Vector3().lerpVectors(startDir, endDir, t).normalize();
    camera.position.copy(targetCenter).add(currentDir.multiplyScalar(distance));
    controls.update();

    if (progress < 1) {
      cameraAnimation = { id: requestAnimationFrame(tick) };
    } else {
      cameraAnimation = null;
    }
  }

  tick();
}


