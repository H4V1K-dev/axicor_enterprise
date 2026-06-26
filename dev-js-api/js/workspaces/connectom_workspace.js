/**
 * @fileoverview connectom_workspace.js — Workspace for routing, socket placement, and connection editing.
 */

import { Workspace } from './workspace.js';

export class ConnectomWorkspace extends Workspace {
  constructor() {
    super('connectom-editor');
  }

  getRequiredTools() {
    return ['tool-inspect', 'tool-translate', 'tool-resize', 'tool-add-socket-btn', 'tool-add-route-btn'];
  }

  getBottomPanelsConfig() {
    return {
      modeSwitch: true,
      hierarchy: true,
      validator: true,
      snapSettings: true
    };
  }
}
