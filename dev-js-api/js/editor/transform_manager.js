/**
 * @fileoverview transform_manager.js — Isolated reactive controller for attaching/detaching TransformControls.
 */

import { store } from '../store/store.js';
import { transformControls } from './transform.js';
import { shardMeshes, socketMeshes, VIS_SCALE } from '../scene_builder.js';

/**
 * Reactively evaluates and updates the snap setting on TransformControls.
 */
function updateTransformSnap() {
  if (!transformControls) return;
  const gridSnapStep = store.get('gridSnapStep');
  if (gridSnapStep > 0) {
    transformControls.translationSnap = gridSnapStep * VIS_SCALE;
  } else {
    transformControls.translationSnap = null;
  }
}

/**
 * Reactively evaluates and updates the attachment state of TransformControls.
 */
function updateTransformControlsAttachment() {
  if (!transformControls) return;

  const selShardKey = store.get('selectedShardKey');
  const selSocketKey = store.get('selectedSocketKey');
  const activeMode = store.get('activeMode');

  // We only attach gizmo axes in translate mode
  if (activeMode === 'translate') {
    if (selShardKey) {
      const mesh = shardMeshes[selShardKey];
      if (mesh) {
        if (transformControls.object !== mesh) {
          transformControls.detach();
          transformControls.attach(mesh);
          transformControls.space = 'world';
          transformControls.showX = true;
          transformControls.showY = false; // Floor Lock
          transformControls.showZ = true;
          updateTransformSnap();
        }
        return;
      }
    } else if (selSocketKey) {
      const group = socketMeshes[selSocketKey];
      if (group) {
        if (transformControls.object !== group) {
          transformControls.detach();
          transformControls.attach(group);
          transformControls.space = 'local';
          transformControls.showX = true;
          transformControls.showY = true;
          transformControls.showZ = true;
          updateTransformSnap();
        }
        return;
      }
    }
  }

  // Detach if conditions are not met
  if (transformControls.object) {
    transformControls.detach();
  }
}

/**
 * Initializes listeners to track selection and mode changes.
 */
export function initTransformManager() {
  store.on('selectedShardKey', updateTransformControlsAttachment);
  store.on('selectedSocketKey', updateTransformControlsAttachment);
  store.on('activeMode', updateTransformControlsAttachment);
  store.on('gridSnapStep', updateTransformSnap);
}
