/**
 * @fileoverview physics_panel.js — Physics configuration drawer panel and slider controls.
 */

import { ROUTER_CONFIG } from '../cable_router.js';
import { drawRoutes } from '../scene_builder.js';
import { showToast } from './toast.js';
import { store } from '../store/store.js';

/**
 * Initializes the physics config drawer panel and hooks it to the toggle button.
 * @param {HTMLButtonElement} physicsBtn 
 */
export function initPhysicsPanel(physicsBtn) {
  const physicsDrawer = document.createElement('div');
  physicsDrawer.id = 'physics-drawer';
  physicsDrawer.className = 'ax-drawer';
  physicsDrawer.innerHTML = `
    <h3 class="ax-section-title">Физика соединений</h3>
    
    <div class="physics-slider-group">
      <div class="physics-slider-row">
        <label title="Сила натяжения кабелей (Laplacian smoothing)">Натяжение (springK):</label>
        <input type="range" id="phy-springK" class="ax-range" min="0.05" max="0.8" step="0.01" value="${ROUTER_CONFIG.springK}">
        <span id="val-springK">${ROUTER_CONFIG.springK}</span>
      </div>
      <div class="physics-slider-row">
        <label title="Радиус отталкивания кабелей от шардов">Радиус отталкивания:</label>
        <input type="range" id="phy-repulsionRad" class="ax-range" min="5" max="40" step="1" value="${ROUTER_CONFIG.repulsionRad}">
        <span id="val-repulsionRad">${ROUTER_CONFIG.repulsionRad}</span>
      </div>
      <div class="physics-slider-row">
        <label title="Сила отталкивания кабелей от шардов">Сила отталкивания:</label>
        <input type="range" id="phy-repulsionStrength" class="ax-range" min="0.1" max="3.0" step="0.1" value="${ROUTER_CONFIG.repulsionStrength}">
        <span id="val-repulsionStrength">${ROUTER_CONFIG.repulsionStrength}</span>
      </div>
      <div class="physics-slider-row">
        <label title="Радиус притяжения параллельных кабелей">Радиус жгута:</label>
        <input type="range" id="phy-attractionRad" class="ax-range" min="5" max="40" step="1" value="${ROUTER_CONFIG.attractionRad}">
        <span id="val-attractionRad">${ROUTER_CONFIG.attractionRad}</span>
      </div>
      <div class="physics-slider-row">
        <label title="Сила притяжения кабелей в общие магистрали">Сила жгута:</label>
        <input type="range" id="phy-attractionStrength" class="ax-range" min="0.01" max="0.8" step="0.01" value="${ROUTER_CONFIG.attractionStrength}">
        <span id="val-attractionStrength">${ROUTER_CONFIG.attractionStrength}</span>
      </div>
      <div class="physics-slider-row">
        <label title="Количество шагов симуляции релаксации">Шаги симуляции:</label>
        <input type="range" id="phy-iterations" class="ax-range" min="10" max="120" step="5" value="${ROUTER_CONFIG.iterations}">
        <span id="val-iterations">${ROUTER_CONFIG.iterations}</span>
      </div>
      <div class="physics-slider-row">
        <label title="Шаг расстановки точек кабеля в вокселях">Сегменты (voxelSegmentLen):</label>
        <input type="range" id="phy-voxelSegmentLength" class="ax-range" min="4" max="30" step="1" value="${ROUTER_CONFIG.voxelSegmentLength}">
        <span id="val-voxelSegmentLength">${ROUTER_CONFIG.voxelSegmentLength}</span>
      </div>
    </div>

    <button class="ax-btn ax-btn--primary" id="physics-apply-btn" style="width:100%;">Применить</button>
  `;
  document.body.appendChild(physicsDrawer);

  const slidersKeys = ['springK', 'repulsionRad', 'repulsionStrength', 'attractionRad', 'attractionStrength', 'iterations', 'voxelSegmentLength'];
  
  // Update live values on slider drag
  slidersKeys.forEach(key => {
    const input = document.getElementById(`phy-${key}`);
    const valSpan = document.getElementById(`val-${key}`);
    if (input && valSpan) {
      input.addEventListener('input', () => {
        valSpan.textContent = input.value;
      });
    }
  });

  // Center align physics drawer center to button center
  const updatePosition = () => {
    const btnRect = physicsBtn.getBoundingClientRect();
    const drawerWidth = 380;
    const targetLeft = btnRect.left + btnRect.width / 2 - drawerWidth / 2;
    const safeLeft = Math.max(16, Math.min(window.innerWidth - drawerWidth - 16, targetLeft));
    physicsDrawer.style.left = safeLeft + 'px';
  };

  let closeTimeout = null;

  const openDrawer = () => {
    if (closeTimeout) clearTimeout(closeTimeout);
    updatePosition();
    physicsDrawer.classList.add('open');
    physicsBtn.classList.add('active');
  };

  const closeDrawer = () => {
    if (closeTimeout) clearTimeout(closeTimeout);
    closeTimeout = setTimeout(() => {
      physicsDrawer.classList.remove('open');
      physicsBtn.classList.remove('active');
    }, 200);
  };

  physicsBtn.addEventListener('click', (e) => {
    e.stopPropagation();
    if (physicsDrawer.classList.contains('open')) {
      closeDrawer();
    } else {
      openDrawer();
    }
  });

  physicsBtn.addEventListener('mouseleave', closeDrawer);

  physicsDrawer.addEventListener('mouseenter', () => {
    if (closeTimeout) clearTimeout(closeTimeout);
  });
  physicsDrawer.addEventListener('mouseleave', closeDrawer);

  // Apply configuration button action
  document.getElementById('physics-apply-btn').addEventListener('click', () => {
    slidersKeys.forEach(key => {
      const input = document.getElementById(`phy-${key}`);
      if (input) {
        ROUTER_CONFIG[key] = parseFloat(input.value);
      }
    });

    try {
      localStorage.setItem('axicor_router_config', JSON.stringify(ROUTER_CONFIG));
    } catch (e) {
      console.warn('Failed to save router config:', e);
    }

    showToast('Параметры физики обновлены!', 'success');

    const routes = store.get('routesData');
    if (routes) {
      drawRoutes(routes);
    }
  });
}
