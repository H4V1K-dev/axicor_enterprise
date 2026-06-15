/**
 * @fileoverview layers_panel.js — Layers control drawer panel and toggles.
 */

import { ORBIT_COLORS, ORBIT_LABELS, setLayerPlaneVisibility } from '../scene_builder.js';
import { store } from '../store/store.js';
import { on, emit, EVENTS } from '../store/event_bus.js';
import * as THREE from 'three';

const SVG_PENCIL = `<svg xmlns="http://www.w3.org/2000/svg" width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-pencil"><path d="M12 20h9"/><path d="M16.5 3.5a2.12 2.12 0 0 1 3 3L7 19l-4 1 1-4Z"/></svg>`;

// Load custom labels from localStorage on startup
try {
  const savedLabels = localStorage.getItem('axicor_orbit_labels');
  if (savedLabels) {
    const parsed = JSON.parse(savedLabels);
    if (Array.isArray(parsed)) {
      parsed.forEach((label, idx) => {
        if (label) ORBIT_LABELS[idx] = label;
      });
    }
  }
} catch (e) {
  console.warn('Failed to load custom orbit labels:', e);
}

// Load custom colors from localStorage on startup
try {
  const savedColors = localStorage.getItem('axicor_orbit_colors');
  if (savedColors) {
    const parsed = JSON.parse(savedColors);
    if (Array.isArray(parsed)) {
      parsed.forEach((colorHex, idx) => {
        if (colorHex) {
          ORBIT_COLORS[idx] = parseInt(colorHex.replace('#', '0x'), 16);
        }
      });
    }
  }
} catch (e) {
  console.warn('Failed to load custom orbit colors:', e);
}

/**
 * Initializes the layers config drawer panel and hooks it to the toggle button.
 * @param {HTMLButtonElement} layersBtn 
 */
export function initLayersPanel(layersBtn) {
  const layersDrawer = document.createElement('div');
  layersDrawer.id = 'layers-drawer';
  layersDrawer.className = 'ax-drawer';
  layersDrawer.innerHTML = `
    <h3 class="ax-section-title">Слои</h3>
    <div id="layers-drawer-list" class="physics-slider-group">
      <!-- Populated dynamically -->
    </div>
  `;
  document.body.appendChild(layersDrawer);

  const listContainer = layersDrawer.querySelector('#layers-drawer-list');

  const updateLayersList = () => {
    const placement = store.get('placementData');
    if (!placement || !placement.orbits) {
      listContainer.innerHTML = '<div class="project-empty">Нет доступных слоев</div>';
      return;
    }

    // Sort orbits descending: highest index first, zero-th (lowest) at the bottom
    const sortedOrbits = [...placement.orbits].sort((a, b) => b.index - a.index);
    const layersVis = store.get('layersVisibility') || {};

    listContainer.innerHTML = sortedOrbits.map(orb => {
      const isChecked = layersVis[orb.index] !== false; // default to true
      const color = ORBIT_COLORS[orb.index] || 0x888888;
      const hexColor = '#' + new THREE.Color(color).getHexString();
      const label = ORBIT_LABELS[orb.index] || `L${orb.index}`;

      return `
        <div class="layer-control-row">
          <div class="layer-control-left">
            <div class="orbit-color-container" title="Изменить цвет слоя">
              <div class="orbit-color-square" data-orbit-index="${orb.index}" style="background: ${hexColor};"></div>
              <input type="color" class="orbit-color-picker" data-orbit-index="${orb.index}" value="${hexColor}">
            </div>
            <span class="layer-control-name">Layer ${orb.index} (${label})</span>
            <button class="layer-rename-btn" data-orbit-index="${orb.index}" title="Переименовать слой">
              ${SVG_PENCIL}
            </button>
          </div>
          <label class="ax-switch-label">
            <input type="checkbox" data-orbit-index="${orb.index}" ${isChecked ? 'checked' : ''}>
            <span class="ax-switch-slider"></span>
          </label>
        </div>
      `;
    }).join('');

    // Bind change events
    listContainer.querySelectorAll('input[type="checkbox"]').forEach(checkbox => {
      checkbox.addEventListener('change', (e) => {
        const orbitIndex = parseInt(e.target.dataset.orbitIndex);
        const visible = e.target.checked;
        setLayerPlaneVisibility(orbitIndex, visible);
      });
    });

    // Bind color picker events
    listContainer.querySelectorAll('.orbit-color-picker').forEach(picker => {
      picker.addEventListener('input', (e) => {
        const orbitIndex = parseInt(e.target.dataset.orbitIndex);
        const hexColor = e.target.value;
        
        // Update local array ORBIT_COLORS
        ORBIT_COLORS[orbitIndex] = parseInt(hexColor.replace('#', '0x'), 16);
        
        // Update the square background instantly
        const square = listContainer.querySelector(`.orbit-color-square[data-orbit-index="${orbitIndex}"]`);
        if (square) {
          square.style.background = hexColor;
        }

        // Save to localStorage
        try {
          const colorsToSave = ORBIT_COLORS.map(c => '#' + new THREE.Color(c).getHexString());
          localStorage.setItem('axicor_orbit_colors', JSON.stringify(colorsToSave));
        } catch (err) {
          console.warn(err);
        }

        // Notify scene and HUD to update
        emit(EVENTS.ORBIT_COLORS_CHANGED);
      });
    });

    // Bind rename events
    listContainer.querySelectorAll('.layer-rename-btn').forEach(btn => {
      btn.addEventListener('click', (e) => {
        e.stopPropagation();
        const orbitIndex = parseInt(btn.dataset.orbitIndex);
        const currentName = ORBIT_LABELS[orbitIndex] || `L${orbitIndex}`;
        const newName = prompt(`Переименовать слой L${orbitIndex}:`, currentName);
        if (newName !== null && newName.trim() !== "") {
          ORBIT_LABELS[orbitIndex] = newName.trim();
          
          try {
            localStorage.setItem('axicor_orbit_labels', JSON.stringify(ORBIT_LABELS));
          } catch (err) {
            console.warn(err);
          }
          
          emit(EVENTS.ORBIT_LABELS_CHANGED);
          updateLayersList();
        }
      });
    });
  };

  // Center align layers drawer center to button center
  const updatePosition = () => {
    const btnRect = layersBtn.getBoundingClientRect();
    const drawerWidth = 380;
    const targetLeft = btnRect.left + btnRect.width / 2 - drawerWidth / 2;
    const safeLeft = Math.max(16, Math.min(window.innerWidth - drawerWidth - 16, targetLeft));
    layersDrawer.style.left = safeLeft + 'px';
  };

  let closeTimeout = null;

  const openDrawer = () => {
    if (closeTimeout) clearTimeout(closeTimeout);
    updateLayersList();
    updatePosition();
    layersDrawer.classList.add('open');
    layersBtn.classList.add('active');
  };

  const closeDrawer = () => {
    if (closeTimeout) clearTimeout(closeTimeout);
    closeTimeout = setTimeout(() => {
      layersDrawer.classList.remove('open');
      layersBtn.classList.remove('active');
    }, 200);
  };

  layersBtn.addEventListener('click', (e) => {
    e.stopPropagation();
    if (layersDrawer.classList.contains('open')) {
      closeDrawer();
    } else {
      openDrawer();
    }
  });

  layersBtn.addEventListener('mouseleave', closeDrawer);

  layersDrawer.addEventListener('mouseenter', () => {
    if (closeTimeout) clearTimeout(closeTimeout);
  });
  layersDrawer.addEventListener('mouseleave', closeDrawer);

  // Re-render list when visualizer data is dynamically updated/reloaded
  on(EVENTS.DATA_RELOADED, updateLayersList);
}
