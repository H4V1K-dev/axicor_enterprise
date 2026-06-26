/**
 * @fileoverview inference_workspace.js — Workspace for SNN inference / step-by-step debugger.
 */

import { Workspace } from './workspace.js';

export class InferenceWorkspace extends Workspace {
  constructor() {
    super('inference-mode');
    this.infTimer = null;
    this.infTick = 0;
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

  enter() {
    super.enter();

    // Show playback panel
    const inferencePanel = document.getElementById('inference-playback-panel');
    if (inferencePanel) {
      inferencePanel.style.display = 'flex';
    }

    // Set up local controls and reset state
    this.setupPlayback();
  }

  exit() {
    // Hide playback panel
    const inferencePanel = document.getElementById('inference-playback-panel');
    if (inferencePanel) {
      inferencePanel.style.display = 'none';
    }

    // Stop timer and clean up button actions
    this.stopSimulation();
    this.cleanupPlayback();

    super.exit();
  }

  setupPlayback() {
    const iPlayBtn = document.getElementById('inference-play-btn');
    const iStepBtn = document.getElementById('inference-step-btn');
    const iResetBtn = document.getElementById('inference-reset-btn');
    const iLabel = document.getElementById('inference-status-label');

    if (iPlayBtn) {
      iPlayBtn.onclick = () => {
        if (this.infTimer) {
          this.pauseSimulation(iPlayBtn);
        } else {
          this.startSimulation(iPlayBtn, iLabel);
        }
      };
    }

    if (iStepBtn) {
      iStepBtn.onclick = () => {
        this.stepSimulation(iLabel);
      };
    }

    if (iResetBtn) {
      iResetBtn.onclick = () => {
        this.resetSimulation(iPlayBtn, iLabel);
      };
    }

    // Reset status label on enter
    if (iLabel) {
      iLabel.textContent = `Инференс: Такт ${this.infTick}`;
    }
  }

  cleanupPlayback() {
    const iPlayBtn = document.getElementById('inference-play-btn');
    const iStepBtn = document.getElementById('inference-step-btn');
    const iResetBtn = document.getElementById('inference-reset-btn');

    if (iPlayBtn) iPlayBtn.onclick = null;
    if (iStepBtn) iStepBtn.onclick = null;
    if (iResetBtn) iResetBtn.onclick = null;
  }

  startSimulation(playBtn, label) {
    if (playBtn) {
      playBtn.classList.add('active');
      playBtn.innerHTML = `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="currentColor" stroke="none"><rect x="14" y="4" width="4" height="16" rx="1"/><rect x="6" y="4" width="4" height="16" rx="1"/></svg>`;
    }

    this.infTimer = setInterval(() => {
      this.stepSimulation(label);
    }, 100);
  }

  pauseSimulation(playBtn) {
    if (this.infTimer) {
      clearInterval(this.infTimer);
      this.infTimer = null;
    }
    if (playBtn) {
      playBtn.innerHTML = `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="currentColor" stroke="none"><polygon points="6 3 20 12 6 21 6 3"/></svg>`;
      playBtn.classList.remove('active');
    }
  }

  stopSimulation() {
    if (this.infTimer) {
      clearInterval(this.infTimer);
      this.infTimer = null;
    }
  }

  stepSimulation(label) {
    this.infTick++;
    if (label) {
      label.textContent = `Инференс: Такт ${this.infTick}`;
    }
  }

  resetSimulation(playBtn, label) {
    this.pauseSimulation(playBtn);
    this.infTick = 0;
    if (label) {
      label.textContent = `Инференс: Такт ${this.infTick}`;
    }
  }
}
