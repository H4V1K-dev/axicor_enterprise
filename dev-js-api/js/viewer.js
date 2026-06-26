import { sceneManager } from './rendering/scene_manager.js';
import { store } from './store/store.js';

export let scene;
export let camera;
export let perspCamera;
export let orthoCamera;
export let isOrthographic;
export let renderer;
export let controls;
export let dirLight;
export let pointLight;

export function addFrameCallback(cb) {
  sceneManager.addFrameCallback(cb);
}

export function initViewer(container) {
  sceneManager.init(container);
  syncViewerBindings();
}

export function syncViewerBindings() {
  scene = sceneManager.scene;
  camera = sceneManager.camera;
  perspCamera = sceneManager.perspCamera;
  orthoCamera = sceneManager.orthoCamera;
  isOrthographic = sceneManager.isOrthographic;
  renderer = sceneManager.renderer;
  controls = sceneManager.controls;
  dirLight = sceneManager.dirLight;
  pointLight = sceneManager.pointLight;
}

export function setCameraProjection(toOrtho) {
  sceneManager.setCameraProjection(toOrtho);
  syncViewerBindings();
}

export function getActiveCamera() {
  return sceneManager.camera;
}

export function animateViewer(onUpdate) {
  sceneManager.animate(() => {
    syncViewerBindings();
    if (onUpdate) onUpdate();
  });
}

export function fitCameraToScene(bbox) {
  sceneManager.fitCameraToScene(bbox);
  syncViewerBindings();
}

export function animateCameraTo(targetDirection, duration = 500) {
  sceneManager.animateCameraTo(targetDirection, duration);
}

// Reactively disable orbit controls when modal window is active
store.on('modalActive', (active) => {
  if (controls) {
    controls.enabled = !active;
  }
});
