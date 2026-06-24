/**
 * @fileoverview hierarchy_panel.js — Unified Hierarchy control drawer panel (Levels, Departments, Shards) with Tab switcher.
 */

import * as THREE from 'three';
import { store } from '../store/store.js';
import { on, emit, EVENTS } from '../store/event_bus.js';
import { buildSceneData, VIS_SCALE } from '../scene_builder.js';
import { scene } from '../viewer.js';
import { saveAllLayoutChanges } from './sidebar.js';
import { layoutLevelsAndShards } from '../algorithms/placement/levels.js';
import { selectShard } from '../editor/selection.js';

let activeGridHelper = null;
let activeTab = 'layers'; // 'layers', 'depts', 'shards'
let selectedDeptName = null;

/**
 * Initializes the unified hierarchy drawer panel and hooks it to the toggle button.
 * @param {HTMLButtonElement} hierarchyBtn 
 */
export function initHierarchyPanel(hierarchyBtn) {
  const hierarchyDrawer = document.createElement('div');
  hierarchyDrawer.id = 'hierarchy-drawer';
  hierarchyDrawer.className = 'ax-drawer';
  hierarchyDrawer.innerHTML = `
    <div class="drawer-header" style="flex-direction: column; align-items: stretch; gap: 8px; border-bottom: none; margin-bottom: 8px; padding-bottom: 0;">
      <div class="drawer-tabs">
        <button class="drawer-tab active" data-tab="layers">Уровни</button>
        <button class="drawer-tab" data-tab="depts">Департаменты</button>
        <button class="drawer-tab" data-tab="shards">Шарды</button>
      </div>
    </div>
    <div id="hierarchy-drawer-list" class="physics-slider-group" style="flex: 1; display: flex; flex-direction: column; gap: 6px; padding: 0 8px; overflow-y: auto;">
      <!-- Populated dynamically -->
    </div>
    <div id="hierarchy-drawer-actions" style="padding: 8px 8px 12px 8px; border-top: 1px solid var(--ax-border-subtle); display: block;">
      <button class="ax-btn ax-btn--secondary" id="hierarchy-add-btn" style="width: 100%; justify-content: center; height: 32px; font-size: 12px;">
        <span style="font-weight: bold; margin-right: 4px;">+</span> Добавить уровень
      </button>
    </div>
  `;
  document.body.appendChild(hierarchyDrawer);

  const listContainer = hierarchyDrawer.querySelector('#hierarchy-drawer-list');
  const actionsContainer = hierarchyDrawer.querySelector('#hierarchy-drawer-actions');
  const addBtn = hierarchyDrawer.querySelector('#hierarchy-add-btn');
  const tabs = hierarchyDrawer.querySelectorAll('.drawer-tab');

  // Handle Tab Switching
  tabs.forEach(tab => {
    tab.addEventListener('click', () => {
      tabs.forEach(t => t.classList.remove('active'));
      tab.classList.add('active');
      activeTab = tab.dataset.tab;

      // Show/Hide Add button action container (only for layers tab)
      if (activeTab === 'layers') {
        actionsContainer.style.display = 'block';
      } else {
        actionsContainer.style.display = 'none';
      }

      renderHierarchyList();
    });
  });

  // Reactive GridHelper handling based on focused orbit
  store.on('focusedLevelId', (lvlId) => {
    if (activeGridHelper) {
      scene.remove(activeGridHelper);
      if (activeGridHelper.geometry) activeGridHelper.geometry.dispose();
      if (activeGridHelper.material) activeGridHelper.material.dispose();
      activeGridHelper = null;
    }

    const cards = listContainer.querySelectorAll('.hierarchy-card, .panel-card');
    cards.forEach(c => {
      c.classList.remove('active');
    });

    if (lvlId !== null) {
      const data = store.get('placementData');
      const lvl = data?.levels.find(l => l.id === lvlId);
      if (lvl) {
        const activeCard = listContainer.querySelector(`.hierarchy-card[data-id="${lvlId}"], .panel-card[data-id="${lvlId}"]`);
        if (activeCard) {
          activeCard.classList.add('active');
        }

        const size = 6000;
        const divisions = 200;
        const colorCenterLine = new THREE.Color(lvl.color || '#10b981');
        const colorGrid = new THREE.Color(0x3f3f3f);
        
        activeGridHelper = new THREE.GridHelper(size, divisions, colorCenterLine, colorGrid);
        activeGridHelper.position.y = (lvl.z_start || 0) * VIS_SCALE;
        activeGridHelper.material.opacity = 0.25;
        activeGridHelper.material.transparent = true;
        
        scene.add(activeGridHelper);
      }
    }
  });

  // Helper for levels layout reordering
  const updateLevelOrder = (newLevelsOrder) => {
    const data = store.get('placementData');
    if (!data) return;

    const oldZStarts = {};
    data.levels.forEach(l => {
      oldZStarts[l.id] = l.z_start;
    });

    const layout = layoutLevelsAndShards(newLevelsOrder, data.shards, oldZStarts);
    data.levels = layout.levels;
    data.shards = layout.shards;

    store.set('placementData', data);
    store.set('hasUnsavedChanges', true);
    buildSceneData(data, true);

    if (activeGridHelper) {
      const focusedId = store.get('focusedLevelId');
      const activeLvl = data.levels.find(l => l.id === focusedId);
      if (activeLvl) {
        activeGridHelper.position.y = (activeLvl.z_start || 0) * VIS_SCALE;
      }
    }

    renderHierarchyList();
  };

  // ----------------------------------------------------
  // RENDER LAYERS (LEVELS) TAB
  // ----------------------------------------------------
  const updateLayersList = (data) => {
    if (!data || !data.levels || data.levels.length === 0) {
      listContainer.innerHTML = '<div class="project-empty" style="padding:16px; color:var(--ax-text-faint); text-align:center;">Нет доступных уровней</div>';
      return;
    }

    const hiddenLevelIds = store.get('hiddenLevelIds') || new Set();
    listContainer.innerHTML = '';

    const reversedLevels = [...data.levels].reverse();

    reversedLevels.forEach((lvl, index) => {
      const card = document.createElement('div');
      card.className = 'hierarchy-card';
      card.draggable = true;
      card.dataset.id = lvl.id;
      card.dataset.index = index;

      const shardCount = data.shards ? data.shards.filter(s => s.orbit === lvl.id).length : 0;
      const canDelete = shardCount === 0;
      const isHidden = hiddenLevelIds.has(lvl.id);

      const eyeSvg = isHidden 
        ? `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24"></path><line x1="1" y1="1" x2="23" y2="23"></line></svg>`
        : `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"></path><circle cx="12" cy="12" r="3"></circle></svg>`;

      const binSvg = `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"></polyline><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path><line x1="10" y1="11" x2="10" y2="17"></line><line x1="14" y1="11" x2="14" y2="17"></line></svg>`;

      const dragSvg = `
      <svg class="drag-handle" width="10" height="16" viewBox="0 0 10 16" fill="none">
        <circle cx="3" cy="2" r="1.2" fill="currentColor"/>
        <circle cx="7" cy="2" r="1.2" fill="currentColor"/>
        <circle cx="3" cy="6" r="1.2" fill="currentColor"/>
        <circle cx="7" cy="6" r="1.2" fill="currentColor"/>
        <circle cx="3" cy="10" r="1.2" fill="currentColor"/>
        <circle cx="7" cy="10" r="1.2" fill="currentColor"/>
        <circle cx="3" cy="14" r="1.2" fill="currentColor"/>
        <circle cx="7" cy="14" r="1.2" fill="currentColor"/>
      </svg>`;

      card.innerHTML = `
        <div class="hierarchy-card__grip">
          ${dragSvg}
        </div>
        <div class="hierarchy-card__content">
          <div class="hierarchy-card__row">
            <div class="hierarchy-card__name-container">
              <span class="hierarchy-card__name" title="Двойной клик для переименования">${lvl.name}</span>
              <button class="edit-name-btn hierarchy-card__edit-btn" title="Переименовать">
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 20h9"></path><path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4L16.5 3.5z"></path></svg>
              </button>
            </div>
            <div class="hierarchy-card__actions">
              <button class="visibility-lvl-btn hierarchy-card__btn hierarchy-card__btn--visibility ${isHidden ? 'muted' : ''}" title="${isHidden ? 'Показать уровень' : 'Скрыть уровень'}">
                ${eyeSvg}
              </button>
              <button class="delete-lvl-btn hierarchy-card__btn hierarchy-card__btn--delete" ${!canDelete ? 'disabled' : ''} title="${canDelete ? 'Удалить уровень' : 'Нельзя удалить уровень, пока на нем есть шарды'}">
                ${binSvg}
              </button>
            </div>
          </div>
          <div class="hierarchy-card__meta-row">
            <span>H: ${Math.round(lvl.height)} vx (${shardCount} шт)</span>
            <div class="hierarchy-card__padding-control">
              <span>Отступ:</span>
              <button class="pad-minus-btn hierarchy-card__padding-btn">-</button>
              <input type="text" class="lvl-padding-input hierarchy-card__padding-input" value="${lvl.padding || 0}">
              <button class="pad-plus-btn hierarchy-card__padding-btn">+</button>
            </div>
          </div>
        </div>
      `;

      card.addEventListener('dragstart', (e) => {
        card.classList.add('dragging');
        card.style.opacity = '0.45';
        e.dataTransfer.effectAllowed = 'move';
        e.dataTransfer.setData('text/plain', lvl.id);
      });

      card.addEventListener('dragend', () => {
        card.classList.remove('dragging');
        card.style.opacity = '';
        renderHierarchyList();
      });

      // Name container inline editing
      const nameContainer = card.querySelector('.hierarchy-card__name-container');
      const editNameBtn = card.querySelector('.edit-name-btn');
      const nameLabel = card.querySelector('.hierarchy-card__name');

      const startEditing = () => {
        nameContainer.innerHTML = `<input type="text" class="ax-input lvl-name-input" value="${lvl.name}" style="font-weight:600; font-size:13px; width:100%; background:var(--ax-bg-input); border:1px solid var(--ax-border-active); padding:2px 6px; color:var(--ax-text); outline:none; border-radius:var(--ax-radius-sm);">`;
        const input = nameContainer.querySelector('.lvl-name-input');
        input.focus();
        input.select();

        const finishEditing = () => {
          const newVal = input.value.trim();
          if (newVal && newVal !== lvl.name) {
            lvl.name = newVal;
            store.set('placementData', data);
            store.set('hasUnsavedChanges', true);
            buildSceneData(data, true);
          }
          renderHierarchyList();
        };

        input.addEventListener('click', (e) => e.stopPropagation());
        input.addEventListener('blur', finishEditing);
        input.addEventListener('keydown', (e) => {
          if (e.key === 'Enter') {
            input.blur();
          } else if (e.key === 'Escape') {
            input.value = lvl.name;
            input.blur();
          }
        });
      };

      editNameBtn.addEventListener('click', (e) => {
        e.stopPropagation();
        startEditing();
      });
      nameLabel.addEventListener('dblclick', (e) => {
        e.stopPropagation();
        startEditing();
      });

      // Padding adjustments
      const paddingInput = card.querySelector('.lvl-padding-input');
      paddingInput.addEventListener('change', (e) => {
        lvl.padding = Math.max(0, parseInt(e.target.value) || 0);
        updateLevelOrder(data.levels);
      });
      paddingInput.addEventListener('click', (e) => e.stopPropagation());

      const minusBtn = card.querySelector('.pad-minus-btn');
      minusBtn.addEventListener('click', (e) => {
        e.stopPropagation();
        const step = e.shiftKey ? 10 : 1;
        lvl.padding = Math.max(0, (lvl.padding || 0) - step);
        updateLevelOrder(data.levels);
      });

      const plusBtn = card.querySelector('.pad-plus-btn');
      plusBtn.addEventListener('click', (e) => {
        e.stopPropagation();
        const step = e.shiftKey ? 10 : 1;
        lvl.padding = (lvl.padding || 0) + step;
        updateLevelOrder(data.levels);
      });

      // Visibility Toggle
      const visibilityBtn = card.querySelector('.visibility-lvl-btn');
      visibilityBtn.addEventListener('click', (e) => {
        e.stopPropagation();
        const hidden = store.get('hiddenLevelIds') || new Set();
        const newHidden = new Set(hidden);
        if (newHidden.has(lvl.id)) {
          newHidden.delete(lvl.id);
        } else {
          newHidden.add(lvl.id);
        }
        store.set('hiddenLevelIds', newHidden);
        renderHierarchyList();
      });

      // Delete Level
      const deleteBtn = card.querySelector('.delete-lvl-btn');
      if (canDelete) {
        deleteBtn.addEventListener('click', (e) => {
          e.stopPropagation();
          const filteredLevels = data.levels.filter(l => l.id !== lvl.id);
          updateLevelOrder(filteredLevels);
        });
      }

      // Card Focus click (selects orbit level in store)
      card.addEventListener('click', (e) => {
        if (e.target.tagName === 'INPUT' || e.target.closest('button') || e.target.classList.contains('drag-handle') || e.target.classList.contains('lvl-padding-input')) {
          return;
        }
        const currentFocus = store.get('focusedLevelId');
        if (currentFocus === lvl.id) {
          store.set('focusedLevelId', null);
          selectedDeptName = null; // Сброс выбранного департамента
        } else {
          store.set('focusedLevelId', lvl.id);
          // Сброс выбранного департамента, если он на другом уровне
          if (selectedDeptName && data && data.departments) {
            const deptObj = data.departments.find(d => d.name === selectedDeptName);
            if (deptObj && deptObj.orbit !== lvl.id) {
              selectedDeptName = null;
            }
          }
        }
      });

      listContainer.appendChild(card);
    });

    // Color active focus outline
    const focusedId = store.get('focusedLevelId');
    if (focusedId !== null) {
      const activeCard = listContainer.querySelector(`.hierarchy-card[data-id="${focusedId}"]`);
      if (activeCard) {
        activeCard.classList.add('active');
      }
    }

    // Drag and Drop sort binders
    listContainer.addEventListener('dragover', (e) => {
      e.preventDefault();
      const draggingCard = listContainer.querySelector('.hierarchy-card.dragging');
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
      const cards = Array.from(listContainer.querySelectorAll('.hierarchy-card'));
      const newOrderIds = cards.map(c => parseInt(c.dataset.id));
      newOrderIds.reverse();
      const newLevelsOrder = newOrderIds.map(id => data.levels.find(l => l.id === id));
      updateLevelOrder(newLevelsOrder);
    });
  };

  function getDragAfterElement(container, y) {
    const draggableElements = [...container.querySelectorAll('.hierarchy-card:not(.dragging)')];
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

  // ----------------------------------------------------
  // RENDER DEPARTMENTS TAB
  // ----------------------------------------------------
  const updateDeptsList = (data) => {
    if (!data || !data.departments || data.departments.length === 0) {
      listContainer.innerHTML = '<div style="padding:20px; color:var(--ax-text-faint); text-align:center; font-size:12px;">Нет департаментов</div>';
      return;
    }

    const focusedLevelId = store.get('focusedLevelId');
    listContainer.innerHTML = '';

    // Фильтруем департаменты, если выбран уровень
    let deptsToShow = [...data.departments];
    if (focusedLevelId !== null) {
      deptsToShow = deptsToShow.filter(d => d.orbit === focusedLevelId);
    }

    deptsToShow.forEach((dept) => {
      const shardCount = data.shards ? data.shards.filter(s => s.dept === dept.name).length : 0;
      const isActive = selectedDeptName === dept.name;
      const initials = dept.name.replace(/([A-Z])/g, ' $1').trim().split(' ').slice(0, 2).map(w => w[0]).join('').toUpperCase() || dept.name.slice(0, 2).toUpperCase();

      const card = document.createElement('div');
      card.className = 'panel-card' + (isActive ? ' active' : '');

      card.innerHTML = `
        <div style="display:flex; align-items:center; gap:10px;">
          <div class="panel-card__icon">${initials}</div>
          <div style="flex:1; min-width:0;">
            <div class="panel-card__title">${dept.name}</div>
            <div class="panel-card__meta">
              <span>Уровень l${dept.orbit}</span>
              <span class="panel-card__badge">${shardCount} шд</span>
            </div>
          </div>
        </div>
      `;

      card.addEventListener('click', () => {
        if (selectedDeptName === dept.name) {
          selectedDeptName = null;
        } else {
          selectedDeptName = dept.name;
        }
        renderHierarchyList();
      });

      listContainer.appendChild(card);
    });
  };

  // ----------------------------------------------------
  // RENDER SHARDS TAB
  // ----------------------------------------------------
  const updateShardsList = (data) => {
    if (!data || !data.shards || data.shards.length === 0) {
      listContainer.innerHTML = '<div style="padding:20px; color:var(--ax-text-faint); text-align:center; font-size:12px;">Нет шардов</div>';
      return;
    }

    const focusedLevelId = store.get('focusedLevelId');
    listContainer.innerHTML = '';

    // 1. Фильтруем шарды, если выбран уровень
    let shardsToShow = [...data.shards];
    if (focusedLevelId !== null) {
      shardsToShow = shardsToShow.filter(s => s.orbit === focusedLevelId);
    }

    // 2. Сортируем шарды, если выбран департамент (шарды этого департамента идут первыми)
    if (selectedDeptName) {
      shardsToShow.sort((a, b) => {
        const aIsSelected = (a.dept === selectedDeptName);
        const bIsSelected = (b.dept === selectedDeptName);
        if (aIsSelected && !bIsSelected) return -1;
        if (!aIsSelected && bIsSelected) return 1;
        return 0;
      });
    }

    shardsToShow.forEach(shard => {
      const shortName = shard.key.split('.').pop() || shard.key;
      const initials = shortName.replace(/([A-Z])/g, ' $1').trim().split(' ').slice(0, 2).map(w => w[0]).join('').toUpperCase() || shortName.slice(0, 2).toUpperCase();

      const card = document.createElement('div');
      card.className = 'panel-card';

      card.innerHTML = `
        <div style="display:flex; align-items:center; gap:10px;">
          <div class="panel-card__icon">${initials}</div>
          <div style="flex:1; min-width:0;">
            <div class="panel-card__title" style="white-space:normal; line-height:1.3;">${shard.key}</div>
            <div class="panel-card__meta">
              <span>${shard.dept}</span>
              <span class="panel-card__badge">l${shard.orbit}</span>
            </div>
          </div>
        </div>
      `;

      card.addEventListener('click', () => {
        selectShard(shard.key);
      });

      listContainer.appendChild(card);
    });
  };

  // ----------------------------------------------------
  // MAIN ROUTING RENDERER
  // ----------------------------------------------------
  const renderHierarchyList = () => {
    const data = store.get('placementData');
    if (activeTab === 'layers') {
      updateLayersList(data);
    } else if (activeTab === 'depts') {
      updateDeptsList(data);
    } else if (activeTab === 'shards') {
      updateShardsList(data);
    }
  };

  // Add Level Event
  addBtn.addEventListener('click', () => {
    if (activeTab !== 'layers') return;

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
      padding: 0,
      color: newColor
    };

    const newLevels = [...data.levels, newLevel];
    updateLevelOrder(newLevels);
  });

  // Calculate drawer viewport layout positions dynamically
  const updatePosition = () => {
    const btnRect = hierarchyBtn.getBoundingClientRect();
    const drawerWidth = 380;
    const targetLeft = btnRect.left + btnRect.width / 2 - drawerWidth / 2;
    const safeLeft = Math.max(16, Math.min(window.innerWidth - drawerWidth - 16, targetLeft));
    hierarchyDrawer.style.left = safeLeft + 'px';
  };

  const openDrawer = () => {
    renderHierarchyList();
    updatePosition();
    hierarchyDrawer.classList.add('open');
    hierarchyBtn.classList.add('active');
  };

  const closeDrawer = () => {
    hierarchyDrawer.classList.remove('open');
    hierarchyBtn.classList.remove('active');
  };

  hierarchyBtn.addEventListener('click', (e) => {
    e.stopPropagation();
    if (hierarchyDrawer.classList.contains('open')) {
      closeDrawer();
    } else {
      // Close other drawers
      const openDrawers = document.querySelectorAll('.ax-drawer.open');
      openDrawers.forEach(d => {
        if (d !== hierarchyDrawer) {
          d.classList.remove('open');
          const triggerId = d.id.replace('-drawer', '-toggle-btn');
          const trigger = document.getElementById(triggerId);
          if (trigger) trigger.classList.remove('active');
        }
      });
      openDrawer();
    }
  });

  const handleOutsideClick = (e) => {
    if (!hierarchyDrawer.classList.contains('open')) return;
    const clickedInside = hierarchyDrawer.contains(e.target) || !e.target.isConnected;
    const clickedToggle = hierarchyBtn.contains(e.target);
    if (!clickedInside && !clickedToggle) {
      closeDrawer();
    }
  };
  document.addEventListener('click', handleOutsideClick);

  on(EVENTS.DATA_RELOADED, renderHierarchyList);
}
