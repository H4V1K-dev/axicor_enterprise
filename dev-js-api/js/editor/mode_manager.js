import * as THREE from 'three';
import { getActiveCamera, renderer } from '../viewer.js';
import { store } from '../store/store.js';
import { emit, EVENTS } from '../store/event_bus.js';

export class ModeManager {
  constructor() {
    /** @type {Object<string, any>} */
    this.modes = {};
    /** @type {string|null} */
    this.activeModeName = null;
    /** @type {any} */
    this.activeMode = null;
    
    // Simple mode history stack
    /** @type {string[]} */
    this.modeHistory = [];
    this.ctrlHeld = false;
    this.raycaster = new THREE.Raycaster();
    this.mouse = new THREE.Vector2();

    // Bind event handlers to maintain proper 'this' context
    this.onPointerDown = this.onPointerDown.bind(this);
    this.onPointerMove = this.onPointerMove.bind(this);
    this.onPointerUp = this.onPointerUp.bind(this);
    this.onKeyDown = this.onKeyDown.bind(this);
    this.onKeyUp = this.onKeyUp.bind(this);
  }

  /**
   * Registers a new interaction mode.
   * @param {string} name
   * @param {any} modeInstance
   */
  register(name, modeInstance) {
    this.modes[name] = modeInstance;
  }

  /**
   * Switch the active interaction mode.
   * @param {string} name
   * @param {any} [ctx] - Optional contextual data passed to enter()
   * @param {boolean} [isPop] - Internal flag to indicate switching is via popping history
   */
  setMode(name, ctx, isPop = false) {
    if (this.activeModeName === name) return;
    if (!this.modes[name]) {
      console.error(`Mode "${name}" is not registered in ModeManager`);
      return;
    }

    // Push previous mode to history stack if it doesn't duplicate the last entry
    // and if the top of the stack is not already the target mode
    if (!isPop && this.activeModeName) {
      const lastInHistory = this.modeHistory[this.modeHistory.length - 1];
      if (lastInHistory !== this.activeModeName && lastInHistory !== name) {
        this.modeHistory.push(this.activeModeName);
        if (this.modeHistory.length > 10) {
          this.modeHistory.shift();
        }
      }
    }

    if (this.activeMode) {
      this.activeMode.exit();
    }

    this.activeModeName = name;
    this.activeMode = this.modes[name];

    // Synchronize to the central store
    store.set('activeMode', name);

    this.activeMode.enter(ctx);

    // Notify UI and other listeners
    emit(EVENTS.MODE_CHANGED, { mode: name });
  }

  /**
   * Pop the last mode from history and make it active.
   * Defaults to 'inspect' if history is empty.
   */
  popMode() {
    if (this.modeHistory.length > 0) {
      const prev = this.modeHistory.pop();
      this.setMode(prev, null, true);
    } else {
      this.setMode('inspect', null, true);
    }
  }

  /**
   * Initialize global interaction listeners on the window.
   */
  init() {
    window.addEventListener('pointerdown', this.onPointerDown);
    window.addEventListener('pointermove', this.onPointerMove);
    window.addEventListener('pointerup', this.onPointerUp);
    window.addEventListener('keydown', this.onKeyDown);
    window.addEventListener('keyup', this.onKeyUp);
  }

  /**
   * Cleanup listeners.
   */
  destroy() {
    window.removeEventListener('pointerdown', this.onPointerDown);
    window.removeEventListener('pointermove', this.onPointerMove);
    window.removeEventListener('pointerup', this.onPointerUp);
    window.removeEventListener('keydown', this.onKeyDown);
    window.removeEventListener('keyup', this.onKeyUp);
  }

  /**
   * Check if a pointer click fell on one of the interactive UI panels.
   * @param {EventTarget} target
   * @returns {boolean}
   */
  isUiClick(target) {
    if (!target || !(target instanceof HTMLElement)) return false;

    // Check if target is outside of the WebGL canvas container
    const canvasContainer = document.getElementById('canvas-container');
    if (canvasContainer && !canvasContainer.contains(target)) {
      return true;
    }

    return !!(
      target.closest('#hud') ||
      target.closest('#sidebar') ||
      target.closest('#tools-sidebar') ||
      target.closest('#tooltip') ||
      target.closest('#project-selector-modal') ||
      target.closest('.bottom-floating-panel') ||
      target.closest('#bottom-left-container') ||
      target.closest('#bottom-right-container') ||
      target.closest('#ax-confirm-modal') ||
      target.closest('#ax-settings-modal')
    );
  }

  /**
   * Update the internal Raycaster coordinates from client pointer event.
   * @param {PointerEvent} event
   */
  updateRaycaster(event) {
    if (renderer && renderer.domElement) {
      const rect = renderer.domElement.getBoundingClientRect();
      this.mouse.x = ((event.clientX - rect.left) / rect.width) * 2 - 1;
      this.mouse.y = -((event.clientY - rect.top) / rect.height) * 2 + 1;
    } else {
      this.mouse.x = (event.clientX / window.innerWidth) * 2 - 1;
      this.mouse.y = -(event.clientY / window.innerHeight) * 2 + 1;
    }
    const activeCamera = getActiveCamera();
    activeCamera.updateMatrixWorld();
    this.raycaster.setFromCamera(this.mouse, activeCamera);
  }

  onPointerDown(event) {
    if (store.get('modalActive')) return;
    if (this.isUiClick(event.target)) return;
    this.updateRaycaster(event);

    if (this.activeMode) {
      this.activeMode.onPointerDown(event, this.raycaster);
    }
  }

  onPointerMove(event) {
    if (store.get('modalActive')) return;
    
    const isUi = this.isUiClick(event.target);
    this.updateRaycaster(event);

    if (this.activeMode) {
      if (isUi) {
        // Clear any stuck hover states when cursor moves over UI overlays
        if (typeof this.activeMode.resetHover === 'function') {
          this.activeMode.resetHover();
        }
        document.body.style.cursor = 'default';
        return;
      }
      this.activeMode.onPointerMove(event, this.raycaster);
    }
  }

  onPointerUp(event) {
    if (store.get('modalActive')) return;
    this.updateRaycaster(event);

    if (this.activeMode) {
      this.activeMode.onPointerUp(event, this.raycaster);
    }
  }

  onKeyDown(event) {
    if (store.get('modalActive')) return;
    if (event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement) {
      return;
    }

    if (event.ctrlKey || event.metaKey) {
      if (event.code === 'KeyZ') {
        event.preventDefault();
        import('../store/history_manager.js').then(({ historyManager }) => {
          historyManager.undoGlobal();
        });
        return;
      }
      if (event.code === 'KeyY') {
        event.preventDefault();
        import('../store/history_manager.js').then(({ historyManager }) => {
          historyManager.redoGlobal();
        });
        return;
      }
    }

    switch (event.code) {
      case 'KeyQ':
        this.setMode('select');
        event.preventDefault();
        return;
      case 'KeyW':
        this.setMode('translate');
        event.preventDefault();
        return;
      case 'KeyT':
        this.setMode('resize');
        event.preventDefault();
        return;
      case 'KeyE':
        this.setMode('add_socket');
        event.preventDefault();
        return;
      case 'KeyR':
        if (store.get('selectedRouteKey')) {
          import('./transform.js').then(({ transformControls }) => {
            if (transformControls) {
              const currentMode = transformControls.mode;
              transformControls.setMode(currentMode === 'translate' ? 'scale' : 'translate');
              import('../ui/toast.js').then(({ showToast }) => {
                showToast(`Режим трансформации узла: ${transformControls.mode === 'translate' ? 'Перемещение' : 'Масштабирование'}`, 'success');
              });
            }
          });
          event.preventDefault();
          return;
        }
        this.setMode('add_shard');
        event.preventDefault();
        return;
      case 'ControlLeft':
      case 'ControlRight':
        if (!this.ctrlHeld) {
          if (this.activeModeName !== 'select') {
            this.ctrlHeld = true;
            this.setMode('select');
          }
        }
        return;
      case 'Delete':
      case 'Backspace': {
        const deleteBtn = document.getElementById('delete-selected-btn');
        if (deleteBtn && deleteBtn.style.display !== 'none') {
          deleteBtn.click();
          event.preventDefault();
        }
        return;
      }
    }

    // Forward to active mode first
    let handled = false;
    if (this.activeMode && typeof this.activeMode.onKeyDown === 'function') {
      handled = this.activeMode.onKeyDown(event);
    }

    // Global fallback for Escape to pop mode stack
    if (!handled && event.code === 'Escape') {
      this.popMode();
      event.preventDefault();
    }
  }

  onKeyUp(event) {
    if (store.get('modalActive')) return;
    if (event.code === 'ControlLeft' || event.code === 'ControlRight') {
      if (this.ctrlHeld) {
        this.ctrlHeld = false;
        this.popMode();
      }
    }
  }

  /**
   * Called on every frame inside the viewer animation cycle.
   * @param {number} dt
   */
  update(dt) {
    if (this.activeMode && typeof this.activeMode.onUpdate === 'function') {
      this.activeMode.onUpdate(dt);
    }
  }
}
