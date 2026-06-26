import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import { THEME } from './theme.js';

export class SceneManager {
  constructor() {
    this.scene = null;
    this.camera = null;
    this.perspCamera = null;
    this.orthoCamera = null;
    this.isOrthographic = false;
    this.renderer = null;
    this.controls = null;
    
    this.dirLight = null;
    this.pointLight = null;

    this.frustumSize = 300;
    this.frameCallbacks = [];

    // Groups
    this.shardsGroup = new THREE.Group();
    this.levelsGroup = new THREE.Group();
    this.deptsGroup = new THREE.Group();

    // Maps & Tracking
    this.shardMeshes = new Map();
    this.shardDataMap = new Map();
    this.socketMeshes = new Map();
    this.shardsByLevel = new Map();
    this.shardsByDept = new Map();
    this.socketsByLevel = new Map();
    this.socketsByDept = new Map();
    this.levelsMeshes = new Map();
    this.deptsMeshes = new Map();
    
    this.visScale = 1.0;
    this.cameraAnimation = null;
  }

  init(container) {
    this.scene = new THREE.Scene();

    // Add groups to scene
    this.scene.add(this.shardsGroup);
    this.scene.add(this.levelsGroup);
    this.scene.add(this.deptsGroup);

    // Perspective Camera
    this.perspCamera = new THREE.PerspectiveCamera(55, window.innerWidth / window.innerHeight, 0.1, 2000);
    this.perspCamera.position.set(40, 30, 40);
    this.perspCamera.layers.enable(1);

    // Orthographic Camera
    const aspect = window.innerWidth / window.innerHeight;
    this.orthoCamera = new THREE.OrthographicCamera(
      -this.frustumSize * aspect / 2,
      this.frustumSize * aspect / 2,
      this.frustumSize / 2,
      -this.frustumSize / 2,
      0.1,
      3000
    );
    this.orthoCamera.position.copy(this.perspCamera.position);
    this.orthoCamera.layers.enable(1);

    this.camera = this.perspCamera;

    // Renderer
    this.renderer = new THREE.WebGLRenderer({ antialias: true, alpha: false, preserveDrawingBuffer: true });
    this.renderer.setSize(window.innerWidth, window.innerHeight);
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    this.renderer.setClearColor(0x1e2025);
    this.renderer.toneMapping = THREE.ACESFilmicToneMapping;
    this.renderer.toneMappingExposure = 1.25;
    container.appendChild(this.renderer.domElement);

    // Controls
    this.controls = new OrbitControls(this.camera, this.renderer.domElement);
    this.controls.enableDamping = true;
    this.controls.dampingFactor = 0.05;
    this.controls.minDistance = 2;
    this.controls.maxDistance = 1500;
    this.controls.target.set(0, 0, 0);

    this.controls.mouseButtons = {
      LEFT: THREE.MOUSE.NONE,
      MIDDLE: THREE.MOUSE.ROTATE,
      RIGHT: THREE.MOUSE.PAN
    };

    this.controls.addEventListener('start', () => {
      if (this.isOrthographic && (this.controls.state === 0 || this.controls.state === 3)) {
        this.setCameraProjection(false);
      }
    });

    // Lights
    const ambientLight = new THREE.AmbientLight(0xffffff, 1.2);
    this.scene.add(ambientLight);

    this.dirLight = new THREE.DirectionalLight(0xc8d0ff, 1.5);
    this.dirLight.position.set(30, 50, 20);
    this.scene.add(this.dirLight);

    this.pointLight = new THREE.PointLight(0x6366f1, 1.2, 1000);
    this.pointLight.position.set(0, 20, 0);
    this.scene.add(this.pointLight);

    // Center Anchor
    const centerGeo = new THREE.SphereGeometry(0.15, 32, 32);
    const centerMat = new THREE.MeshStandardMaterial({
      color: 0x8b9cf7,
      emissive: 0x8b9cf7,
      emissiveIntensity: 1.5,
      roughness: 0.1
    });
    const centerMesh = new THREE.Mesh(centerGeo, centerMat);
    this.scene.add(centerMesh);

    // Resize Handler
    window.addEventListener('resize', () => {
      const width = window.innerWidth;
      const height = window.innerHeight;
      const currentAspect = width / height;

      this.perspCamera.aspect = currentAspect;
      this.perspCamera.updateProjectionMatrix();

      this.orthoCamera.left = -this.frustumSize * currentAspect / 2;
      this.orthoCamera.right = this.frustumSize * currentAspect / 2;
      this.orthoCamera.top = this.frustumSize / 2;
      this.orthoCamera.bottom = -this.frustumSize / 2;
      this.orthoCamera.updateProjectionMatrix();

      this.renderer.setSize(width, height);
    });
  }

  setCameraProjection(toOrtho) {
    if (this.isOrthographic === toOrtho) return;

    const target = this.controls.target.clone();
    const startPos = this.camera.position.clone();
    const distance = startPos.distanceTo(target);
    const dir = startPos.clone().sub(target).normalize();

    if (toOrtho) {
      const fovRad = this.perspCamera.fov * (Math.PI / 180);
      const visibleHeight = 2 * distance * Math.tan(fovRad / 2);
      
      this.orthoCamera.zoom = this.frustumSize / visibleHeight;
      this.orthoCamera.position.copy(this.perspCamera.position);
      this.orthoCamera.quaternion.copy(this.perspCamera.quaternion);
      this.orthoCamera.updateProjectionMatrix();

      this.camera = this.orthoCamera;
      this.isOrthographic = true;
    } else {
      const fovRad = this.perspCamera.fov * (Math.PI / 180);
      const visibleHeight = this.frustumSize / this.orthoCamera.zoom;
      const targetDistance = visibleHeight / (2 * Math.tan(fovRad / 2));

      this.perspCamera.position.copy(target).add(dir.multiplyScalar(targetDistance));
      this.perspCamera.quaternion.copy(this.orthoCamera.quaternion);
      this.perspCamera.updateProjectionMatrix();

      this.camera = this.perspCamera;
      this.isOrthographic = false;
    }

    this.controls.object = this.camera;
    this.controls.update();

    import('../editor/transform.js').then(({ transformControls }) => {
      if (transformControls) {
        transformControls.camera = this.camera;
        transformControls.update();
      }
    }).catch(() => {});
  }

  fitCameraToScene(bbox) {
    const center = new THREE.Vector3();
    bbox.getCenter(center);
    const size = new THREE.Vector3();
    bbox.getSize(size);

    const maxDim = Math.max(size.x, size.y, size.z);
    
    this.dirLight.position.set(center.x + maxDim, center.y + maxDim * 1.5, center.z + maxDim);
    this.dirLight.lookAt(center);
    
    this.pointLight.position.copy(center);
    this.pointLight.distance = maxDim * 4.0;

    const fov = this.perspCamera.fov * (Math.PI / 180);
    let cameraDist = maxDim / (2 * Math.tan(fov / 2));
    cameraDist *= 1.45;

    this.perspCamera.position.set(center.x + cameraDist * 0.8, center.y + cameraDist * 0.6, center.z + cameraDist * 0.8);
    this.orthoCamera.position.copy(this.perspCamera.position);
    
    if (this.isOrthographic) {
      this.orthoCamera.zoom = this.frustumSize / (maxDim * 1.45 * 2);
      this.orthoCamera.updateProjectionMatrix();
    }

    this.controls.target.copy(center);
    this.controls.update();
  }

  animateCameraTo(targetDirection, duration = 500) {
    if (this.cameraAnimation) {
      cancelAnimationFrame(this.cameraAnimation.id);
    }

    const startPos = this.camera.position.clone();
    const targetCenter = this.controls.target.clone();
    const distance = startPos.distanceTo(targetCenter);
    
    const startDir = startPos.clone().sub(targetCenter).normalize();
    const endDir = targetDirection.clone().normalize();

    if (startDir.dot(endDir) < -0.999) {
      startDir.add(new THREE.Vector3(0.01, 0.01, 0.01).normalize()).normalize();
    }

    const startTime = performance.now();

    const easeInOutCubic = (t) => {
      return t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2;
    };

    const tick = () => {
      const now = performance.now();
      const elapsed = now - startTime;
      const progress = Math.min(elapsed / duration, 1);
      const t = easeInOutCubic(progress);

      const currentDir = new THREE.Vector3().lerpVectors(startDir, endDir, t).normalize();
      this.camera.position.copy(targetCenter).add(currentDir.multiplyScalar(distance));
      this.controls.update();

      if (progress < 1) {
        this.cameraAnimation = { id: requestAnimationFrame(tick) };
      } else {
        this.cameraAnimation = null;
      }
    };

    tick();
  }

  addFrameCallback(cb) {
    this.frameCallbacks.push(cb);
  }

  animate(onUpdate) {
    const tick = () => {
      requestAnimationFrame(tick);
      
      this.controls.update();
      
      this.frameCallbacks.forEach(cb => {
        try {
          cb();
        } catch (err) {
          console.error("Error in scene manager frame callback:", err);
        }
      });
      
      if (onUpdate) {
        onUpdate();
      }
      
      if (this.renderer && this.scene && this.camera) {
        this.renderer.render(this.scene, this.camera);
      }
    };
    tick();
  }

  clearScene() {
    this.disposeHierarchy(this.shardsGroup);
    this.disposeHierarchy(this.levelsGroup);
    this.disposeHierarchy(this.deptsGroup);

    this.shardsGroup.clear();
    this.levelsGroup.clear();
    this.deptsGroup.clear();

    this.shardMeshes.clear();
    this.shardDataMap.clear();
    this.socketMeshes.clear();
    this.shardsByLevel.clear();
    this.shardsByDept.clear();
    this.socketsByLevel.clear();
    this.socketsByDept.clear();
    this.levelsMeshes.clear();
    this.deptsMeshes.clear();
  }

  disposeHierarchy(obj) {
    if (!obj) return;
    obj.traverse(child => {
      if (child.geometry) {
        child.geometry.dispose();
      }
      if (child.material) {
        if (Array.isArray(child.material)) {
          child.material.forEach(m => m.dispose());
        } else {
          child.material.dispose();
        }
      }
    });
  }
}

export const sceneManager = new SceneManager();
