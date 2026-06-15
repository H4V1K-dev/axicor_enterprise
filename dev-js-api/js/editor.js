import * as THREE from 'three';
import { camera, renderer, scene, controls } from './viewer.js';
import { shardMeshes, socketMeshes, VIS_SCALE, shardDataMap } from './scene_builder.js';
import { initTransformControls, transformControls } from './editor/transform.js';
import { selectShard, selectSocket, deselectAll } from './editor/selection.js';
import { checkShardCollision } from './editor/collision_adapter.js';
import { updateFocusVisuals } from './editor/focus.js';
import {
  onPointerDown as onHandlePointerDown,
  onPointerMove as onHandlePointerMove,
  onPointerUp as onHandlePointerUp,
  isDragging as isDraggingHandle,
  updateSelectedSocket,
  updateHandlesScale
} from './editor/handle_drag.js';
import {
  onPointerDown as onDividerPointerDown,
  onPointerMove as onDividerPointerMove,
  onPointerUp as onDividerPointerUp,
  isDragging as isDraggingDivider,
  updateLayersFromDividers,
  updateLayersOrderIn3D
} from './editor/divider_drag.js';
import { spawnSomasForShard, clearSomas } from './rendering/soma_renderer.js';
import { ModeManager } from './editor/mode_manager.js';
import { store } from './store/store.js';
import { SelectMode } from './editor/modes/select_mode.js';
import { TranslateMode } from './editor/modes/translate_mode.js';
import { InspectMode } from './editor/modes/inspect_mode.js';
import { ResizeMode } from './editor/modes/resize_mode.js';

import { AddShardMode } from './editor/modes/add_shard_mode.js';
import { AddSocketMode } from './editor/modes/add_socket_mode.js';
import { AddRouteMode } from './editor/modes/add_route_mode.js';

// Re-exports for other modules (like ui.js, main.js)
export {
  transformControls,
  deselectAll,
  selectSocket,
  selectShard,
  checkShardCollision,
  updateFocusVisuals,
  updateSelectedSocket,
  updateHandlesScale,
  updateLayersOrderIn3D,
  updateLayersFromDividers,
  spawnSomasForShard,
  clearSomas
};

export let modeManager = null;

export function initEditor() {
  initTransformControls();

  modeManager = new ModeManager();
  modeManager.register('inspect', new InspectMode());
  modeManager.register('select', new SelectMode());
  modeManager.register('translate', new TranslateMode());
  modeManager.register('resize', new ResizeMode());
  modeManager.register('add_shard', new AddShardMode());
  modeManager.register('add_socket', new AddSocketMode());
  modeManager.register('add_route', new AddRouteMode());

  // Default to inspect mode
  modeManager.setMode('inspect');
  modeManager.init();
}

// Global window registration for backward compatibility
window.updateLayersOrderIn3D = updateLayersOrderIn3D;
window.spawnSomasForShard = spawnSomasForShard;

