/**
 * @fileoverview layers_panel.js — Levels control drawer panel with Drag & Drop ordering and automated Z stacking.
 */

import { store } from '../store/store.js';
import { on, emit, EVENTS } from '../store/event_bus.js';
import { buildSceneData } from '../scene_builder.js';
import { saveAllLayoutChanges } from './sidebar.js';
import { layoutLevelsAndShards } from '../algorithms/placement/levels.js';

/**
 * Initializes the levels config drawer panel and hooks it to the toggle button.
 * @param {HTMLButtonElement} layersBtn 
 */
export function initLayersPanel(layersBtn) {
  const layersDrawer = document.createElement('div');
  layersDrawer.id = 'layers-drawer';
  layersDrawer.className = 'ax-drawer';
  layersDrawer.innerHTML = `
    <div style="display:flex; align-items:center; justify-content:space-between; padding: 0 8px 12px 8px; border-bottom: 1px solid var(--ax-border-muted); margin-bottom: 12px;">
      <h3 class="ax-section-title" style="margin:0;">Уровни (Levels)</h3>
      <button class="ax-btn ax-btn--primary" id="add-level-btn" style="padding: 4px 8px; font-size:12px; height:auto; display:flex; align-items:center; gap:4px;">
        <span style="font-size:14px; font-weight:bold;">+</span> Добавить
      </button>
    </div>
    <div id="layers-drawer-list" class="physics-slider-group" style="display:flex; flex-direction:column; gap:8px; padding:0 8px 12px 8px;">
      <!-- Populated dynamically -->
    </div>
  `;
  document.body.appendChild(layersDrawer);

  const listContainer = layersDrawer.querySelector('#layers-drawer-list');
  const addBtn = layersDrawer.querySelector('#add-level-btn');

  // Core level reordering and Z translation math
  const updateLevelOrder = (newLevelsOrder) => {
    const data = store.get('placementData');
    if (!data) return;

    // Preserve old z_starts
    const oldZStarts = {};
    data.levels.forEach(l => {
      oldZStarts[l.id] = l.z_start;
    });

    // Run layout stacking
    const layout = layoutLevelsAndShards(newLevelsOrder, data.shards, oldZStarts);
    data.levels = layout.levels;
    data.shards = layout.shards;

    store.set('placementData', data);
    buildSceneData(data, true);
    saveAllLayoutChanges();
  };

  const updateLayersList = () => {
    const data = store.get('placementData');
    if (!data || !data.levels || data.levels.length === 0) {
      listContainer.innerHTML = '<div class="project-empty" style="padding:16px; color:var(--ax-text-faint);">Нет доступных уровней</div>';
      return;
    }

    listContainer.innerHTML = '';

    const reversedLevels = [...data.levels].reverse();

    reversedLevels.forEach((lvl, index) => {
      const card = document.createElement('div');
      card.className = 'ax-list-item layer-card';
      card.draggable = true;
      card.dataset.id = lvl.id;
      card.dataset.index = index;
      
      card.style.display = 'flex';
      card.style.alignItems = 'center';
      card.style.justifyContent = 'space-between';
      card.style.gap = '12px';
      card.style.padding = '8px 12px';
      card.style.border = '1px solid var(--ax-border-muted)';
      card.style.borderRadius = '6px';
      card.style.background = 'rgba(255, 255, 255, 0.02)';
      card.style.cursor = 'grab';

      // Count shards on this level
      const shardCount = data.shards ? data.shards.filter(s => s.orbit === lvl.id).length : 0;
      const canDelete = shardCount === 0;

      card.innerHTML = `
        <div style="display:flex; align-items:center; gap:10px; flex:1;">
          <span class="drag-handle" style="color: var(--ax-text-faint); font-size:16px; user-select:none;">☰</span>
          <div style="width:12px; height:12px; border-radius:50%; background:${lvl.color || '#fff'}; flex-shrink:0;"></div>
          <input type="text" class="ax-input lvl-name-input" value="${lvl.name}" style="font-weight:600; font-size:13px; width:100%; background:transparent; border:none; padding:2px 4px; color:var(--ax-text); outline:none;">
        </div>
        <div style="display:flex; align-items:center; gap:12px;">
          <span style="color:var(--ax-text-faint); font-size:11px; white-space:nowrap;">H: ${Math.round(lvl.height)} vx (${shardCount} шт)</span>
          <button class="delete-lvl-btn" ${!canDelete ? 'disabled' : ''} style="background:transparent; border:none; color:${canDelete ? 'var(--ax-text-faint)' : 'rgba(255,255,255,0.05)'}; cursor:${canDelete ? 'pointer' : 'not-allowed'}; padding:4px; display:flex; align-items:center; justify-content:center;" title="${canDelete ? 'Удалить уровень' : 'Нельзя удалить уровень, пока на нем есть шарды'}">
            <span style="font-size:14px; font-weight:bold; color:${canDelete ? '#ff5f56' : '#555'};">🗑</span>
          </button>
        </div>
      `;

      card.addEventListener('dragstart', (e) => {
        card.style.opacity = '0.4';
        e.dataTransfer.effectAllowed = 'move';
        e.dataTransfer.setData('text/plain', lvl.id);
      });

      card.addEventListener('dragend', () => {
        card.style.opacity = '1';
        updateLayersList();
      });

      const nameInput = card.querySelector('.lvl-name-input');
      nameInput.addEventListener('change', (e) => {
        lvl.name = e.target.value;
        store.set('placementData', data);
        saveAllLayoutChanges();
      });

      const deleteBtn = card.querySelector('.delete-lvl-btn');
      if (canDelete) {
        deleteBtn.addEventListener('click', (e) => {
          e.stopPropagation();
          const filteredLevels = data.levels.filter(l => l.id !== lvl.id);
          updateLevelOrder(filteredLevels);
        });
      }

      listContainer.appendChild(card);
    });

    listContainer.addEventListener('dragover', (e) => {
      e.preventDefault();
      const draggingCard = listContainer.querySelector('.layer-card[style*="opacity"]');
      if (!draggingCard) return;

      const afterElement = getDragAfterElement(listContainer, e.clientY);
      if (afterElement == null) {
        listContainer.appendChild(draggingCard);
      } else {
        listContainer.insertBefore(draggingCard, afterElement);
      }
    });

    listContainer.addEventListener('drop', (e) => {
      e.preventDefault();
      const cards = Array.from(listContainer.querySelectorAll('.layer-card'));
      const newOrderIds = cards.map(c => parseInt(c.dataset.id));
      newOrderIds.reverse();
      const newLevelsOrder = newOrderIds.map(id => data.levels.find(l => l.id === id));
      updateLevelOrder(newLevelsOrder);
    });
  };

  function getDragAfterElement(container, y) {
    const draggableElements = [...container.querySelectorAll('.layer-card:not([style*="opacity"])')];
    return draggableElements.reduce((closest, child) => {
      const box = child.getBoundingClientRect();
      const offset = y - box.top - box.height / 2;
      if (offset < 0 && offset > closest.offset) {
        return { offset: offset, element: child };
      } else {
        return closest;
      }
    }, { offset: Number.NEGATIVE_INFINITY }).element;
  }

  addBtn.addEventListener('click', () => {
    const data = store.get('placementData');
    if (!data) return;

    const maxId = data.levels.length > 0 ? Math.max(...data.levels.map(l => l.id)) : 0;
    const newId = maxId + 1;
    const colors = ["#34d399", "#38bdf8", "#f472b6", "#a78bfa", "#f472b6"];
    const newColor = colors[data.levels.length % colors.length];

    const newLevel = {
      id: newId,
      name: `Level ${newId}`,
      z_start: 0,
      height: 40,
      color: newColor
    };

    const newLevels = [...data.levels, newLevel];
    updateLevelOrder(newLevels);
  });

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

  on(EVENTS.DATA_RELOADED, updateLayersList);
}
