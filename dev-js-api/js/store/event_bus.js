/**
 * @fileoverview Central event bus for loose coupling of UI, rendering and editor components.
 * Follows rule INV-4: One event — one contract payload.
 */

/** @typedef {import("../contracts/types.js").Vec3} Vec3 */

/**
 * Enumeration of all supported events and their payload schemas.
 * @enum {string}
 */
export const EVENTS = {
  /**
   * Fired when selected shard or socket changes.
   * Payload: { shardKey: string|null, socketKey: string|null }
   */
  SELECTION_CHANGED: 'selection:changed',

  /**
   * Fired when the active editor tool mode changes.
   * Payload: { mode: string }
   */
  MODE_CHANGED: 'mode:changed',

  /**
   * Fired when the active workspace changes.
   * Payload: { workspace: string }
   */
  WORKSPACE_CHANGED: 'workspace:changed',

  /**
   * Fired when a shard's position or size is updated in the editor.
   * Payload: { shardKey: string, position: Vec3, size: {w: number, d: number, h: number} }
   */
  LAYOUT_CHANGED: 'layout:changed',

  /**
   * Fired when the visualizer data is reloaded.
   * Payload: null
   */
  DATA_RELOADED: 'data:reloaded',

  /**
   * Fired to request topology validation.
   * Payload: null
   */
  VALIDATION_REQ: 'validation:requested',

  /**
   * Fired when visualizer reload is requested from UI.
   * Payload: null
   */
  RELOAD_REQ: 'reload:requested',

  /**
   * Fired when shard cortical layers change.
   * Payload: ShardData
   */
  LAYERS_CHANGED: 'layers:changed',

  /**
   * Fired when orbit labels change.
   * Payload: null
   */
  ORBIT_LABELS_CHANGED: 'orbit_labels:changed',

  /**
   * Fired when orbit colors change.
   * Payload: null
   */
  ORBIT_COLORS_CHANGED: 'orbit_colors:changed'
};

/** @type {Map<string, Set<Function>>} */
const listeners = new Map();

/**
 * Subscribe a handler to an event.
 * @param {string} event - Event name from EVENTS enum
 * @param {Function} handler - Callback function
 */
export function on(event, handler) {
  if (!listeners.has(event)) {
    listeners.set(event, new Set());
  }
  listeners.get(event).add(handler);
}

/**
 * Unsubscribe a handler from an event.
 * @param {string} event - Event name from EVENTS enum
 * @param {Function} handler - Callback function to remove
 */
export function off(event, handler) {
  const set = listeners.get(event);
  if (set) {
    set.delete(handler);
    if (set.size === 0) {
      listeners.delete(event);
    }
  }
}

/**
 * Emit an event with a payload to all subscribers.
 * @param {string} event - Event name from EVENTS enum
 * @param {any} [payload] - Event payload
 */
export function emit(event, payload = null) {
  const set = listeners.get(event);
  if (set) {
    // Execute a copy of the set to allow handlers to unsubscribe during emission
    const handlers = Array.from(set);
    for (const handler of handlers) {
      try {
        handler(payload);
      } catch (err) {
        console.error(`Error in event handler for "${event}":`, err);
      }
    }
  }
}
