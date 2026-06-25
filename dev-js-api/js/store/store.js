/**
 * @fileoverview Central store for managing application state.
 * Dumb container with a simple key-value state and change listeners.
 */

// Load editorSettings from localStorage if available
let cachedSettings = null;
try {
  const raw = localStorage.getItem('axicor_editor_settings');
  if (raw) {
    cachedSettings = JSON.parse(raw);
  }
} catch (e) {
  console.warn('Failed to parse cached editor settings:', e);
}

const defaultEditorSettings = {
  grid_step: 100,
  snap_step: 1,
  resize_step: 10,
  cable_subdivision_step: 30,
  history_mode: 'global',
  default_shard_w: 32,
  default_shard_d: 32,
  default_shard_h: 16,
  default_socket_w: 4,
  default_socket_h: 4,
  default_socket_pitch: 2,
  viewcube_sensitivity: 0.0075
};

const state = {
  projectName: null,
  placementData: null,
  routesData: null,
  selectedShardKey: null,
  selectedShardKeys: new Set(),
  selectedSocketKey: null,
  connectionMode: 1,
  activeWorkspace: 'model-composition',
  activeMode: 'inspect',
  hasUnsavedChanges: false,
  focusedLevelId: null,
  hiddenLevelIds: new Set(),
  soloLevelId: null,
  selectedDeptName: null,
  focusedShardKey: null,
  visScale: 1.0,
  modalActive: false,
  editorSettings: cachedSettings ? { ...defaultEditorSettings, ...cachedSettings } : defaultEditorSettings,
  gridSnapStep: (cachedSettings ? cachedSettings.snap_step : null) ?? 1,
  resizeSnapStep: (cachedSettings ? cachedSettings.resize_step : null) ?? 10
};

/** @type {Map<string, Set<Function>>} */
const listeners = new Map();

export const store = {
  /**
   * Get a value from the state.
   * @param {string} key
   * @returns {any}
   */
  get(key) {
    return state[key];
  },

  /**
   * Set a value in the state and notify listeners if the value changed.
   * @param {string} key
   * @param {any} value
   */
  set(key, value) {
    if (state[key] !== value) {
      const oldValue = state[key];
      state[key] = value;

      const set = listeners.get(key);
      if (set) {
        const callbacks = Array.from(set);
        for (const cb of callbacks) {
          try {
            cb(value, oldValue);
          } catch (err) {
            console.error(`Error in store listener for "${key}":`, err);
          }
        }
      }
    }
  },

  /**
   * Set multiple values in the state atomically, then notify listeners for changed keys.
   * @param {Object<string, any>} updates
   */
  setMultiple(updates) {
    const changes = [];
    for (const [key, value] of Object.entries(updates)) {
      if (state[key] !== value) {
        const oldValue = state[key];
        state[key] = value;
        changes.push({ key, value, oldValue });
      }
    }
    // Notify listeners after all updates are applied
    for (const { key, value, oldValue } of changes) {
      const set = listeners.get(key);
      if (set) {
        const callbacks = Array.from(set);
        for (const cb of callbacks) {
          try {
            cb(value, oldValue);
          } catch (err) {
            console.error(`Error in store listener for "${key}":`, err);
          }
        }
      }
    }
  },

  /**
   * Subscribe to a state property change.
   * @param {string} key
   * @param {Function} callback
   */
  on(key, callback) {
    if (!listeners.has(key)) {
      listeners.set(key, new Set());
    }
    listeners.get(key).add(callback);
  },

  /**
   * Unsubscribe from a state property change.
   * @param {string} key
   * @param {Function} callback
   */
  off(key, callback) {
    const set = listeners.get(key);
    if (set) {
      set.delete(callback);
      if (set.size === 0) {
        listeners.delete(key);
      }
    }
  }
};

// Auto-sync store values to localStorage editorSettings
store.on('gridSnapStep', (val) => {
  const settings = { ...store.get('editorSettings'), snap_step: val };
  state.editorSettings = settings; // Update in-place to avoid redundant state events, or call store.set if needed. We update direct to avoid cycles.
  try {
    localStorage.setItem('axicor_editor_settings', JSON.stringify(settings));
  } catch (e) {
    console.warn('Failed to save snap_step to localStorage:', e);
  }
});

store.on('resizeSnapStep', (val) => {
  const settings = { ...store.get('editorSettings'), resize_step: val };
  state.editorSettings = settings;
  try {
    localStorage.setItem('axicor_editor_settings', JSON.stringify(settings));
  } catch (e) {
    console.warn('Failed to save resize_step to localStorage:', e);
  }
});

// Update snap/resize steps if settings object is modified/replaced (e.g. from Settings panel modal)
store.on('editorSettings', (settings) => {
  if (settings) {
    if (settings.snap_step !== undefined && settings.snap_step !== state.gridSnapStep) {
      store.set('gridSnapStep', settings.snap_step);
    }
    if (settings.resize_step !== undefined && settings.resize_step !== state.resizeSnapStep) {
      store.set('resizeSnapStep', settings.resize_step);
    }
  }
});

