/**
 * @fileoverview composition_workspace.js — Workspace for layout compilation and shard operations.
 */

import { Workspace } from './workspace.js';

export class CompositionWorkspace extends Workspace {
  constructor() {
    super('model-composition');
  }

  getRequiredTools() {
    return ['tool-inspect', 'tool-translate', 'tool-resize', 'add-shard-btn'];
  }

  getBottomPanelsConfig() {
    return {
      modeSwitch: false,
      hierarchy: true,
      validator: true,
      snapSettings: true
    };
  }
}
