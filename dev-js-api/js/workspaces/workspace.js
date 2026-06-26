/**
 * @fileoverview workspace.js — Base class for all workspaces in AxiCAD.
 */

import { deselectAll } from '../editor.js';

export class Workspace {
  /**
   * @param {string} name Unique identifier of the workspace.
   */
  constructor(name) {
    this.name = name;
  }

  /**
   * Returns the list of DOM element IDs for tools allowed in this workspace.
   * @returns {string[]}
   */
  getRequiredTools() {
    return ['tool-inspect'];
  }

  /**
   * Returns configuration for bottom panels visibility.
   * @returns {{ modeSwitch: boolean, hierarchy: boolean, validator: boolean, snapSettings: boolean }}
   */
  getBottomPanelsConfig() {
    return {
      modeSwitch: false,
      hierarchy: false,
      validator: false,
      snapSettings: false
    };
  }

  /**
   * Called when entering the workspace.
   * Overridden by subclasses to mount custom UI and start background timers/simulations.
   */
  enter() {
    // Force deselect all to avoid weird editing states across workspaces
    deselectAll();
    this.applyCommonUI();
  }

  /**
   * Called when exiting the workspace.
   * Overridden by subclasses to teardown UI and clear resources/timers.
   */
  exit() {
    // Base exit does nothing
  }

  /**
   * Updates common UI elements based on the workspace's tools and panels configuration.
   */
  applyCommonUI() {
    const requiredTools = this.getRequiredTools();
    const allToolIds = [
      'tool-inspect',
      'tool-translate',
      'tool-resize',
      'tool-add-socket-btn',
      'tool-add-route-btn',
      'add-shard-btn'
    ];

    // Show/hide tools in sidebar
    allToolIds.forEach(id => {
      const el = document.getElementById(id);
      if (el) {
        el.style.display = requiredTools.includes(id) ? '' : 'none';
      }
    });

    // Check active mode fallback
    import('../editor.js').then(({ modeManager }) => {
      if (modeManager && modeManager.activeModeName) {
        const allowedModeNames = [];
        if (requiredTools.includes('tool-inspect')) allowedModeNames.push('inspect');
        if (requiredTools.includes('tool-translate')) allowedModeNames.push('translate');
        if (requiredTools.includes('tool-resize')) allowedModeNames.push('resize');
        if (requiredTools.includes('tool-add-socket-btn')) allowedModeNames.push('add_socket');
        if (requiredTools.includes('tool-add-route-btn')) allowedModeNames.push('add_route');
        if (requiredTools.includes('add-shard-btn')) allowedModeNames.push('add_shard');

        if (!allowedModeNames.includes(modeManager.activeModeName)) {
          modeManager.setMode('inspect');
        }
      }
    });

    // Show/hide bottom panels
    const panelsConfig = this.getBottomPanelsConfig();
    const modeSwitchPanel = document.getElementById('mode-switch-panel');
    const hierarchyToggle = document.getElementById('hierarchy-toggle-btn');
    const validatorToggle = document.getElementById('validator-toggle-btn');
    const snapSettingsPanel = document.getElementById('snap-settings-panel');

    if (modeSwitchPanel) modeSwitchPanel.style.display = panelsConfig.modeSwitch ? '' : 'none';
    if (hierarchyToggle) hierarchyToggle.style.display = panelsConfig.hierarchy ? '' : 'none';
    if (validatorToggle) validatorToggle.style.display = panelsConfig.validator ? '' : 'none';
    if (snapSettingsPanel) snapSettingsPanel.style.display = panelsConfig.snapSettings ? 'flex' : 'none';

    // Close drawers if they shouldn't be visible in the current workspace
    if (!panelsConfig.hierarchy) {
      const drawer = document.getElementById('hierarchy-drawer');
      if (drawer && drawer.classList.contains('open')) {
        drawer.classList.remove('open');
        if (hierarchyToggle) hierarchyToggle.classList.remove('active');
      }
    }

    if (!panelsConfig.validator) {
      const drawer = document.getElementById('validator-drawer');
      if (drawer && drawer.classList.contains('open')) {
        drawer.classList.remove('open');
        if (validatorToggle) validatorToggle.classList.remove('active');
      }
    }
  }
}
