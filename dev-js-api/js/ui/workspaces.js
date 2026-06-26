/**
 * @fileoverview workspaces.js — Workspace Layout manager.
 * Manages the lifecycle of core AxiCAD workspaces using a polymorphic design.
 */

import { store } from '../store/store.js';
import { emit, EVENTS } from '../store/event_bus.js';
import { CompositionWorkspace } from '../workspaces/composition_workspace.js';
import { NeuronLabWorkspace } from '../workspaces/neuron_lab_workspace.js';
import { ConnectomWorkspace } from '../workspaces/connectom_workspace.js';
import { GrowthWorkspace } from '../workspaces/growth_workspace.js';
import { InferenceWorkspace } from '../workspaces/inference_workspace.js';

class WorkspaceManager {
  constructor() {
    this.workspaces = new Map();
    this.activeWorkspace = null;
  }

  /**
   * Registers a workspace instance.
   * @param {import('../workspaces/workspace.js').Workspace} workspace
   */
  register(workspace) {
    this.workspaces.set(workspace.name, workspace);
  }

  /**
   * Switches the active workspace.
   * @param {string} name Workspace unique name.
   */
  switchWorkspace(name) {
    const nextWS = this.workspaces.get(name);
    if (!nextWS) {
      console.warn(`Workspace "${name}" is not registered.`);
      return;
    }

    if (this.activeWorkspace) {
      this.activeWorkspace.exit();
    }

    this.activeWorkspace = nextWS;
    store.set('activeWorkspace', name);
    this.activeWorkspace.enter();

    // Update active class on tab elements
    const tabs = document.querySelectorAll('.workspace-tab');
    tabs.forEach(tab => {
      if (tab.dataset.workspace === name) {
        tab.classList.add('active');
      } else {
        tab.classList.remove('active');
      }
    });

    emit(EVENTS.WORKSPACE_CHANGED, { workspace: name });
  }
}

export function initWorkspaces() {
  const tabsContainer = document.getElementById('workspace-tabs');
  const tabs = document.querySelectorAll('.workspace-tab');

  if (!tabsContainer || tabs.length === 0) {
    console.warn('Workspace tabs markup not found.');
    return;
  }

  const manager = new WorkspaceManager();
  manager.register(new CompositionWorkspace());
  manager.register(new NeuronLabWorkspace());
  manager.register(new ConnectomWorkspace());
  manager.register(new GrowthWorkspace());
  manager.register(new InferenceWorkspace());

  // Hook up tab click listeners
  tabs.forEach(tab => {
    tab.addEventListener('click', () => {
      const ws = tab.dataset.workspace;
      manager.switchWorkspace(ws);
    });
  });

  // Watch store project loading to display the workspace tabs bar
  store.on('projectName', (name) => {
    if (name) {
      tabsContainer.style.display = 'flex';
      // Default back to first tab on new project load
      manager.switchWorkspace('model-composition');
    } else {
      tabsContainer.style.display = 'none';
      if (manager.activeWorkspace) {
        manager.activeWorkspace.exit();
        manager.activeWorkspace = null;
      }
    }
  });
}
