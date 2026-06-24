/**
 * @fileoverview history_manager.js — Global and object-level selective history management.
 */

import { store } from './store.js';
import { buildSceneData, drawRoutes } from '../scene_builder.js';
import { deselectAll } from '../editor/selection.js';
import { historyToThree } from '../editor/coordinate_adapter.js';
import { emit, EVENTS } from './event_bus.js';

class HistoryManager {
  constructor() {
    /** @type {Array<any>} */
    this.globalStack = [];
    /** @type {number} */
    this.globalIndex = -1;
    /** @type {Object<string, Array<any>>} */
    this.objectHistory = {};

    // Temporary preview storage
    this.previewActive = false;
    this.previewIndex = -1;
    this.originalPlacement = null;
    this.originalRoutes = null;
  }

  /**
   * Logs a new action to global and local stacks.
   * @param {string} type - 'move' | 'resize' | 'delete' | 'create' | 'disconnect' | 'delete_pair' | 'delete_with_connections'
   * @param {string} targetType - 'shard' | 'socket' | 'connection'
   * @param {string} targetKey - Unique key of the modified object
   * @param {string} description - User-friendly message
   * @param {any} undoState - State before modification
   * @param {any} redoState - State after modification
   */
  pushAction(type, targetType, targetKey, description, undoState, redoState) {
    if (this.previewActive) {
      // Discard active preview copy if a normal action is pushed
      this.previewActive = false;
      this.originalPlacement = null;
      this.originalRoutes = null;
    }

    // Truncate any forward history if we were in an undone state
    if (this.globalIndex < this.globalStack.length - 1) {
      const actionsToInvalidate = this.globalStack.slice(this.globalIndex + 1);
      this.globalStack = this.globalStack.slice(0, this.globalIndex + 1);

      // Clean up these actions from local histories of the target objects
      actionsToInvalidate.forEach(act => {
        if (act.targetKey && this.objectHistory[act.targetKey]) {
          this.objectHistory[act.targetKey] = this.objectHistory[act.targetKey].filter(a => a.id !== act.id);
        }
      });
    }

    const action = {
      id: `act_${Date.now()}_${Math.random().toString(36).substring(2, 7)}`,
      timestamp: Date.now(),
      type,
      targetType,
      targetKey,
      description,
      undoState: JSON.parse(JSON.stringify(undoState)),
      redoState: JSON.parse(JSON.stringify(redoState))
    };

    // Push to global
    this.globalStack.push(action);
    if (this.globalStack.length > 200) {
      this.globalStack.shift();
    }
    this.globalIndex = this.globalStack.length - 1;

    // Push to local object history
    if (targetKey) {
      if (!this.objectHistory[targetKey]) {
        this.objectHistory[targetKey] = [];
      }
      this.objectHistory[targetKey].push(action);
      if (this.objectHistory[targetKey].length > 1000) {
        this.objectHistory[targetKey].shift();
      }
    }

    store.set('hasUnsavedChanges', true);
    // Trigger reactive updates in UI
    store.set('historyUpdated', Date.now());
  }

  /**
   * Helper to deep clone placementData from store.
   */
  clonePlacement() {
    return JSON.parse(JSON.stringify(store.get('placementData')));
  }

  /**
   * Applies target state changes to the placementData structure.
   */
  applyActionState(placementData, action, isUndo) {
    const state = isUndo ? action.undoState : action.redoState;
    const type = action.type;
    const targetKey = action.targetKey;

    if (action.targetType === 'shard') {
      const shardKey = targetKey;

      if (type === 'create') {
        if (isUndo) {
          // Remove shard
          placementData.shards = placementData.shards.filter(s => s.key !== shardKey);
        } else {
          // Recreate shard
          if (state && !placementData.shards.some(s => s.key === shardKey)) {
            placementData.shards.push(state);
          }
        }
      } 
      else if (type === 'delete') {
        if (isUndo) {
          // Restore shard
          if (state) {
            placementData.shards.push(state);
          }
          if (placementData.deleted_shards) {
            placementData.deleted_shards = placementData.deleted_shards.filter(k => k !== shardKey);
          }
        } else {
          // Re-delete shard
          if (!placementData.deleted_shards) placementData.deleted_shards = [];
          if (!placementData.deleted_shards.includes(shardKey)) {
            placementData.deleted_shards.push(shardKey);
          }
          placementData.shards = placementData.shards.filter(s => s.key !== shardKey);
        }
      }
      else if (type === 'delete_with_connections') {
        if (isUndo) {
          // Restore shard, peer sockets, and connections
          if (state.shard) {
            placementData.shards.push(state.shard);
          }
          if (placementData.deleted_shards) {
            placementData.deleted_shards = placementData.deleted_shards.filter(k => k !== shardKey);
          }

          if (state.peerSockets) {
            state.peerSockets.forEach(ps => {
              const peerShard = placementData.shards.find(s => s.key === ps.shardKey);
              if (peerShard && peerShard.sockets) {
                if (!peerShard.sockets.some(s => s.name === ps.socket.name)) {
                  peerShard.sockets.push(ps.socket);
                }
              }
              if (placementData.deleted_sockets) {
                const psk = `${ps.shardKey}.${ps.socket.name}`;
                placementData.deleted_sockets = placementData.deleted_sockets.filter(k => k !== psk);
              }
            });
          }

          if (state.connections) {
            state.connections.forEach(conn => {
              placementData.connections.push(conn);
              if (placementData.deleted_connections) {
                const connKey = `${conn.from}.${conn.from_socket} -> ${conn.to}.${conn.to_socket}`;
                placementData.deleted_connections = placementData.deleted_connections.filter(k => k !== connKey);
              }
            });
          }
        } else {
          // Re-delete shard, peer sockets, and connections
          if (!placementData.deleted_shards) placementData.deleted_shards = [];
          if (!placementData.deleted_shards.includes(shardKey)) {
            placementData.deleted_shards.push(shardKey);
          }
          placementData.shards = placementData.shards.filter(s => s.key !== shardKey);

          if (action.undoState && action.undoState.peerSockets) {
            if (!placementData.deleted_sockets) placementData.deleted_sockets = [];
            action.undoState.peerSockets.forEach(ps => {
              const psk = `${ps.shardKey}.${ps.socket.name}`;
              if (!placementData.deleted_sockets.includes(psk)) {
                placementData.deleted_sockets.push(psk);
              }
              const peerShard = placementData.shards.find(s => s.key === ps.shardKey);
              if (peerShard && peerShard.sockets) {
                peerShard.sockets = peerShard.sockets.filter(s => s.name !== ps.socket.name);
              }
            });
          }

          if (action.undoState && action.undoState.connections) {
            if (!placementData.deleted_connections) placementData.deleted_connections = [];
            action.undoState.connections.forEach(conn => {
              const connKey = `${conn.from}.${conn.from_socket} -> ${conn.to}.${conn.to_socket}`;
              if (!placementData.deleted_connections.includes(connKey)) {
                placementData.deleted_connections.push(connKey);
              }
              placementData.connections = placementData.connections.filter(c => 
                !(c.from === conn.from && c.from_socket === conn.from_socket && c.to === conn.to && c.to_socket === conn.to_socket)
              );
            });
          }
        }
      }
      else { // 'move' or 'resize'
        const shard = placementData.shards.find(s => s.key === shardKey);
        if (shard && state) {
          if (state.position) shard.position = JSON.parse(JSON.stringify(state.position));
          if (state.size) shard.size = JSON.parse(JSON.stringify(state.size));
        }
      }
    } 
    else if (action.targetType === 'socket') {
      const lastDot = targetKey.lastIndexOf('.');
      const shardKey = targetKey.substring(0, lastDot);
      const socketName = targetKey.substring(lastDot + 1);

      if (type === 'create') {
        const shard = placementData.shards.find(s => s.key === shardKey);
        if (isUndo) {
          if (shard && shard.sockets) {
            shard.sockets = shard.sockets.filter(s => s.name !== socketName);
          }
        } else {
          if (shard && shard.sockets && state) {
            if (!shard.sockets.some(s => s.name === socketName)) {
              shard.sockets.push(state);
            }
          }
        }
      }
      else if (type === 'delete') {
        const shard = placementData.shards.find(s => s.key === shardKey);
        if (isUndo) {
          if (shard && shard.sockets && state) {
            shard.sockets.push(state);
          }
          if (placementData.deleted_sockets) {
            placementData.deleted_sockets = placementData.deleted_sockets.filter(k => k !== targetKey);
          }
        } else {
          if (shard && shard.sockets) {
            shard.sockets = shard.sockets.filter(s => s.name !== socketName);
          }
          if (!placementData.deleted_sockets) placementData.deleted_sockets = [];
          if (!placementData.deleted_sockets.includes(targetKey)) {
            placementData.deleted_sockets.push(targetKey);
          }
        }
      }
      else if (type === 'delete_pair') {
        const shardA = placementData.shards.find(s => s.key === shardKey);
        
        if (isUndo) {
          if (state.socketA && shardA) {
            shardA.sockets.push(state.socketA);
          }
          if (placementData.deleted_sockets) {
            placementData.deleted_sockets = placementData.deleted_sockets.filter(k => k !== targetKey);
          }

          if (state.socketB) {
            const shardB = placementData.shards.find(s => s.key === state.socketB.shardKey);
            if (shardB && shardB.sockets) {
              shardB.sockets.push(state.socketB.socket);
            }
            if (placementData.deleted_sockets) {
              const psk = `${state.socketB.shardKey}.${state.socketB.socket.name}`;
              placementData.deleted_sockets = placementData.deleted_sockets.filter(k => k !== psk);
            }
          }

          if (state.connection) {
            placementData.connections.push(state.connection);
            if (placementData.deleted_connections) {
              const connKey = `${state.connection.from}.${state.connection.from_socket} -> ${state.connection.to}.${state.connection.to_socket}`;
              placementData.deleted_connections = placementData.deleted_connections.filter(k => k !== connKey);
            }
          }
        } else {
          if (shardA && shardA.sockets) {
            shardA.sockets = shardA.sockets.filter(s => s.name !== socketName);
          }
          if (!placementData.deleted_sockets) placementData.deleted_sockets = [];
          if (!placementData.deleted_sockets.includes(targetKey)) {
            placementData.deleted_sockets.push(targetKey);
          }

          if (action.undoState && action.undoState.socketB) {
            const sb = action.undoState.socketB;
            const psk = `${sb.shardKey}.${sb.socket.name}`;
            if (!placementData.deleted_sockets.includes(psk)) {
              placementData.deleted_sockets.push(psk);
            }
            const shardB = placementData.shards.find(s => s.key === sb.shardKey);
            if (shardB && shardB.sockets) {
              shardB.sockets = shardB.sockets.filter(s => s.name !== sb.socket.name);
            }
          }

          if (action.undoState && action.undoState.connection) {
            const conn = action.undoState.connection;
            const connKey = `${conn.from}.${conn.from_socket} -> ${conn.to}.${conn.to_socket}`;
            if (!placementData.deleted_connections) placementData.deleted_connections = [];
            if (!placementData.deleted_connections.includes(connKey)) {
              placementData.deleted_connections.push(connKey);
            }
            placementData.connections = placementData.connections.filter(c => 
              !(c.from === conn.from && c.from_socket === conn.from_socket && c.to === conn.to && c.to_socket === conn.to_socket)
            );
          }
        }
      }
      else if (type === 'resize' || type === 'move') {
        const shard = placementData.shards.find(s => s.key === shardKey);
        if (shard && shard.sockets && state) {
          const socket = shard.sockets.find(s => s.name === socketName);
          if (socket) {
            if (state.width !== undefined) socket.width = state.width;
            if (state.height !== undefined) socket.height = state.height;
            if (state.pitch !== undefined) socket.pitch = state.pitch;
            if (state.rotation !== undefined) socket.rotation = state.rotation;
            if (state.faceSign !== undefined) socket.faceSign = state.faceSign;
            
            const offset = state.offset || state.originalOffset;
            if (offset) {
              socket.offset = {
                x: offset.x,
                y: offset.y
              };
              if (offset.z !== undefined) {
                socket.offset.z = offset.z;
              }
            } else {
              delete socket.offset;
            }
            if (state.entry_z !== undefined) {
              socket.entry_z = state.entry_z;
            } else {
              delete socket.entry_z;
            }
            if (state.name) socket.name = state.name;
          }
        }
      }
    } 
    else if (action.targetType === 'connection') {
      if (type === 'move' || type === 'resize') {
        const matchingConn = placementData.connections.find(c => 
          `${c.from}.${c.from_socket}→${c.to}.${c.to_socket}` === targetKey ||
          `${c.to}.${c.to_socket}→${c.from}.${c.from_socket}` === targetKey
        );
        if (matchingConn && state) {
          matchingConn.control_points = JSON.parse(JSON.stringify(state.control_points));
          matchingConn.manual = state.manual;
        }
      }
      else {
        const conns = state || action.undoState;
        const connArray = Array.isArray(conns) ? conns : [conns];

        connArray.forEach(conn => {
          if (!conn) return;
          const connKey = `${conn.from}.${conn.from_socket} -> ${conn.to}.${conn.to_socket}`;

          if (type === 'disconnect') {
            if (isUndo) {
              placementData.connections.push(conn);
              if (placementData.deleted_connections) {
                placementData.deleted_connections = placementData.deleted_connections.filter(k => k !== connKey);
              }
            } else {
              if (!placementData.deleted_connections) placementData.deleted_connections = [];
              if (!placementData.deleted_connections.includes(connKey)) {
                placementData.deleted_connections.push(connKey);
              }
              placementData.connections = placementData.connections.filter(c => 
                !(c.from === conn.from && c.from_socket === conn.from_socket && c.to === conn.to && c.to_socket === conn.to_socket)
              );
            }
          }
        });
      }
    }
  }

  /**
   * Triggers delta events to incrementally update the scene for an action.
   * @param {any} action 
   * @param {boolean} isUndo 
   */
  triggerDeltaUpdate(action, isUndo) {
    const state = isUndo ? action.undoState : action.redoState;
    const type = action.type;
    const targetKey = action.targetKey;

    if (action.targetType === 'shard') {
      if (type === 'create') {
        if (isUndo) {
          emit(EVENTS.SHARD_DELETED, targetKey);
        } else {
          if (state) emit(EVENTS.SHARD_ADDED, state);
        }
      } 
      else if (type === 'delete') {
        if (isUndo) {
          if (state) emit(EVENTS.SHARD_ADDED, state);
        } else {
          emit(EVENTS.SHARD_DELETED, targetKey);
        }
      }
      else if (type === 'delete_with_connections') {
        if (isUndo) {
          if (state && state.shard) {
            emit(EVENTS.SHARD_ADDED, state.shard);
          }
        } else {
          emit(EVENTS.SHARD_DELETED, targetKey);
        }
      }
      else { // 'move' or 'resize'
        if (state && state.position && state.size) {
          emit(EVENTS.SHARD_TRANSFORMED, {
            key: targetKey,
            position: state.position,
            size: state.size
          });
        }
      }
    }
  }

  /**
   * Triggers linear Undo globally.
   */
  undoGlobal() {
    if (this.globalIndex >= 0) {
      deselectAll();
      const placementData = this.clonePlacement();
      const action = this.globalStack[this.globalIndex];
      this.applyActionState(placementData, action, true);

      this.globalIndex--;
      store.set('placementData', placementData);
      this.triggerDeltaUpdate(action, true);
      const routes = store.get('routesData');
      if (routes) drawRoutes(routes);

      store.set('historyUpdated', Date.now());
    }
  }

  /**
   * Triggers linear Redo globally.
   */
  redoGlobal() {
    if (this.globalIndex < this.globalStack.length - 1) {
      deselectAll();
      const placementData = this.clonePlacement();
      this.globalIndex++;
      const action = this.globalStack[this.globalIndex];
      this.applyActionState(placementData, action, false);

      store.set('placementData', placementData);
      this.triggerDeltaUpdate(action, false);
      const routes = store.get('routesData');
      if (routes) drawRoutes(routes);

      store.set('historyUpdated', Date.now());
    }
  }

  /**
   * Previews a state in history temporarily.
   * @param {string} actionId
   */
  previewState(actionId) {
    const stack = this.getActiveStack();
    const actionIndex = stack.findIndex(a => a.id === actionId);
    if (actionIndex === -1) return;

    if (!this.previewActive) {
      this.previewActive = true;
      this.originalPlacement = JSON.parse(JSON.stringify(store.get('placementData')));
      this.originalRoutes = JSON.parse(JSON.stringify(store.get('routesData')));
    }

    this.previewIndex = actionIndex;

    // Build the preview state: start from original state and apply actions
    const placementData = JSON.parse(JSON.stringify(this.originalPlacement));
    
    if (this.isFocusedHistory()) {
      // Revert all future local actions from the current end of the local stack
      // (which is stack.length - 1 since local history is always viewed at its latest state)
      for (let i = stack.length - 1; i > actionIndex; i--) {
        this.applyActionState(placementData, stack[i], true);
      }
    } else {
      // Global history preview:
      if (actionIndex < this.globalIndex) {
        // Revert all steps down to the selected action index
        for (let i = this.globalIndex; i > actionIndex; i--) {
          this.applyActionState(placementData, this.globalStack[i], true);
        }
      } else if (actionIndex > this.globalIndex) {
        // Redo all steps up to the selected action index
        for (let i = this.globalIndex + 1; i <= actionIndex; i++) {
          this.applyActionState(placementData, this.globalStack[i], false);
        }
      }
    }

    // Temporarily set and rebuild
    store.set('placementData', placementData);
    buildSceneData(placementData, true);
    if (this.originalRoutes) drawRoutes(this.originalRoutes);
  }

  /**
   * Commits the active preview, writing a new action (Git Revert Style).
   */
  applyPreview() {
    if (!this.previewActive || this.previewIndex === -1) return;

    const isFocused = this.isFocusedHistory();
    const stack = this.getActiveStack();
    const actionIndex = this.previewIndex;
    const action = stack[actionIndex];
    const previewPlacement = JSON.parse(JSON.stringify(store.get('placementData')));

    this.previewActive = false;
    this.originalPlacement = null;
    this.originalRoutes = null;
    this.previewIndex = -1;

    if (!isFocused) {
      // Global history: roll back and truncate future global actions
      if (actionIndex < this.globalIndex) {
        const actionsToInvalidate = this.globalStack.slice(actionIndex + 1);
        this.globalStack = this.globalStack.slice(0, actionIndex + 1);
        this.globalIndex = actionIndex;

        // Clean up these actions from local histories of the target objects
        actionsToInvalidate.forEach(act => {
          if (act.targetKey && this.objectHistory[act.targetKey]) {
            this.objectHistory[act.targetKey] = this.objectHistory[act.targetKey].filter(a => a.id !== act.id);
          }
        });
      } else if (actionIndex > this.globalIndex) {
        // Redo up to actionIndex: just update the globalIndex
        this.globalIndex = actionIndex;
      }
    } else {
      // Local history: roll back and truncate future local actions of the selected object
      const key = store.get('selectedShardKey') || store.get('selectedSocketKey');
      if (key && this.objectHistory[key]) {
        const localActions = this.objectHistory[key];
        const localActionIndex = localActions.findIndex(a => a.id === action.id);
        if (localActionIndex !== -1) {
          const localToInvalidate = localActions.slice(localActionIndex + 1);
          this.objectHistory[key] = localActions.slice(0, localActionIndex + 1);

          // Selectively delete these actions from the global stack (since they are scattered)
          const idsToRemove = localToInvalidate.map(a => a.id);
          
          let newGlobalIndex = this.globalIndex;
          const newGlobalStack = [];
          
          for (let i = 0; i < this.globalStack.length; i++) {
            const act = this.globalStack[i];
            if (idsToRemove.includes(act.id)) {
              if (i <= this.globalIndex) {
                newGlobalIndex--;
              }
            } else {
              newGlobalStack.push(act);
            }
          }
          
          this.globalStack = newGlobalStack;
          this.globalIndex = Math.max(-1, newGlobalIndex);
        }
      }
    }

    // Apply the previewed placement state and rebuild
    store.set('placementData', previewPlacement);
    buildSceneData(previewPlacement, true);
    const routes = store.get('routesData');
    if (routes) drawRoutes(routes);

    deselectAll();
    store.set('historyUpdated', Date.now());
  }

  /**
   * Resets active preview, returning to the current state.
   */
  resetPreview() {
    if (!this.previewActive) return;

    const origPlacement = this.originalPlacement;
    const origRoutes = this.originalRoutes;

    this.previewActive = false;
    this.originalPlacement = null;
    this.originalRoutes = null;
    this.previewIndex = -1;

    store.set('placementData', origPlacement);
    buildSceneData(origPlacement, true);
    if (origRoutes) drawRoutes(origRoutes);

    store.set('historyUpdated', Date.now());
  }

  /**
   * Deserializes and restores history stacks.
   * @param {any} data
   */
  deserializeHistory(data) {
    if (!data) {
      this.globalStack = [];
      this.globalIndex = -1;
      this.objectHistory = {};
    } else {
      const threeHistory = historyToThree(data);
      this.globalStack = threeHistory.globalStack || [];
      this.globalIndex = typeof threeHistory.globalIndex === 'number' ? threeHistory.globalIndex : -1;
      this.objectHistory = threeHistory.objectHistory || {};
    }
    // Discard active previews
    this.previewActive = false;
    this.previewIndex = -1;
    this.originalPlacement = null;
    this.originalRoutes = null;
  }

  /**
   * Checks if the history panel should display individual focused history.
   * @returns {boolean}
   */
  isFocusedHistory() {
    const editorSettings = store.get('editorSettings') || {};
    if (editorSettings.history_mode === 'global') {
      return false;
    }
    const selShardKey = store.get('selectedShardKey');
    const selSocketKey = store.get('selectedSocketKey');
    return !!(selShardKey || selSocketKey);
  }

  /**
   * Returns active history stack based on focus state.
   * @returns {Array<any>}
   */
  getActiveStack() {
    if (this.isFocusedHistory()) {
      const selShardKey = store.get('selectedShardKey');
      const selSocketKey = store.get('selectedSocketKey');
      const key = selShardKey || selSocketKey;
      return this.objectHistory[key] || [];
    }
    // Filter global stack down to active items matching globalIndex linear boundary
    return this.globalStack;
  }
}

export const historyManager = new HistoryManager();
