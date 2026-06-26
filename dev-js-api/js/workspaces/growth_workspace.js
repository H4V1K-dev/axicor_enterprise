/**
 * @fileoverview growth_workspace.js — Workspace for growth morphogenesis simulation.
 */

import { Workspace } from './workspace.js';

export class GrowthWorkspace extends Workspace {
  constructor() {
    super('growth-simulator');
    this.growthTimer = null;
    this.growthStep = 0;
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
    const growthPanel = document.getElementById('growth-playback-panel');
    if (growthPanel) {
      growthPanel.style.display = 'flex';
    }

    // Set up local controls and reset state
    this.setupPlayback();
  }

  exit() {
    // Hide playback panel
    const growthPanel = document.getElementById('growth-playback-panel');
    if (growthPanel) {
      growthPanel.style.display = 'none';
    }

    // Stop timer and clean up button actions to prevent memory leaks and background ticks
    this.stopSimulation();
    this.cleanupPlayback();

    super.exit();
  }

  setupPlayback() {
    const gPlayBtn = document.getElementById('growth-play-btn');
    const gStepBtn = document.getElementById('growth-step-btn');
    const gResetBtn = document.getElementById('growth-reset-btn');
    const gLabel = document.getElementById('growth-status-label');

    if (gPlayBtn) {
      gPlayBtn.onclick = () => {
        if (this.growthTimer) {
          this.pauseSimulation(gPlayBtn);
        } else {
          this.startSimulation(gPlayBtn, gLabel);
        }
      };
    }

    if (gStepBtn) {
      gStepBtn.onclick = () => {
        this.stepSimulation(gLabel);
      };
    }

    if (gResetBtn) {
      gResetBtn.onclick = () => {
        this.resetSimulation(gPlayBtn, gLabel);
      };
    }

    // Reset status label on enter
    if (gLabel) {
      gLabel.textContent = `Рост: Шаг ${this.growthStep} / 100`;
    }
  }

  cleanupPlayback() {
    const gPlayBtn = document.getElementById('growth-play-btn');
    const gStepBtn = document.getElementById('growth-step-btn');
    const gResetBtn = document.getElementById('growth-reset-btn');

    if (gPlayBtn) gPlayBtn.onclick = null;
    if (gStepBtn) gStepBtn.onclick = null;
    if (gResetBtn) gResetBtn.onclick = null;
  }

  startSimulation(playBtn, label) {
    if (playBtn) {
      playBtn.classList.add('active');
      playBtn.innerHTML = `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="currentColor" stroke="none"><rect x="14" y="4" width="4" height="16" rx="1"/><rect x="6" y="4" width="4" height="16" rx="1"/></svg>`;
    }

    this.growthTimer = setInterval(() => {
      this.stepSimulation(label);
    }, 150);
  }

  pauseSimulation(playBtn) {
    if (this.growthTimer) {
      clearInterval(this.growthTimer);
      this.growthTimer = null;
    }
    if (playBtn) {
      playBtn.innerHTML = `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="currentColor" stroke="none"><polygon points="6 3 20 12 6 21 6 3"/></svg>`;
      playBtn.classList.remove('active');
    }
  }

  stopSimulation() {
    if (this.growthTimer) {
      clearInterval(this.growthTimer);
      this.growthTimer = null;
    }
  }

  stepSimulation(label) {
    this.growthStep++;
    if (this.growthStep > 100) {
      this.growthStep = 0;
    }
    if (label) {
      label.textContent = `Рост: Шаг ${this.growthStep} / 100`;
    }
  }

  resetSimulation(playBtn, label) {
    this.pauseSimulation(playBtn);
    this.growthStep = 0;
    if (label) {
      label.textContent = `Рост: Шаг ${this.growthStep} / 100`;
    }
  }
}
