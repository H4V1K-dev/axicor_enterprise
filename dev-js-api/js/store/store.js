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
  selectedSocketKey: null,
  connectionMode: 1,
  activeWorkspace: 'model-composition',
  activeMode: 'inspect',
  hasUnsavedChanges: false,
  focusedLevelId: null,
  hiddenLevelIds: new Set(),
  soloLevelId: null,
  modalActive: false,
  editorSettings: cachedSettings ? { ...defaultEditorSettings, ...cachedSettings } : defaultEditorSettings
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
