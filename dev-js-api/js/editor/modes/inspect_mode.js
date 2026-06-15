import { selectShard, selectSocket, deselectAll } from '../selection.js';
import { updateFocusVisuals } from '../focus.js';
import { modeManager } from '../../editor.js';
import { resolveRaycastHit } from '../collision_manager.js';

export class InspectMode {
  enter() {
    document.body.style.cursor = 'default';
    deselectAll();
    updateFocusVisuals();
  }

  exit() {}

  onPointerDown(event, raycaster) {
    if (event.button !== 0) return false;
    // Double click on an object in InspectMode selects it and transitions to TranslateMode
    if (event.detail >= 2) {
      const bestHit = resolveRaycastHit(raycaster);

      if (bestHit) {
        if (bestHit.type === 'socket') {
          selectSocket(bestHit.key);
          modeManager.setMode('translate');
          return true;
        } else if (bestHit.type === 'shard') {
          selectShard(bestHit.key);
          modeManager.setMode('translate');
          return true;
        }
      }
    }

    return false;
  }

  onPointerMove(event, raycaster) {}

  onPointerUp(event, raycaster) {}

  onKeyDown(event) {
    return false;
  }
}
