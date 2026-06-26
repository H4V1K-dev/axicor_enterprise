/**
 * @fileoverview neuron_lab_workspace.js — Placeholder workspace for neuron-level sandbox.
 */

import { Workspace } from './workspace.js';

export class NeuronLabWorkspace extends Workspace {
  constructor() {
    super('neuron-lab');
  }

  getRequiredTools() {
    return ['tool-inspect'];
  }

  getBottomPanelsConfig() {
    return {
      modeSwitch: false,
      hierarchy: false,
      validator: false,
      snapSettings: false
    };
  }
}
