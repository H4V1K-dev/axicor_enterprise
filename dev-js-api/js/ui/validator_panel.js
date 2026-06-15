import { shardMeshes, socketMeshes, shardDataMap, VIS_SCALE, drawRoutes, rebuildSocket, updateAllSocketVisuals } from '../scene_builder.js';
import { runValidation } from '../validator.js';
import { saveAllLayoutChanges } from './sidebar.js';
import { store } from '../store/store.js';
import { on, emit, EVENTS } from '../store/event_bus.js';
import { lintScene } from '../algorithms/toml/index.js';

/**
 * Constructs a live model placement data structure using the active 3D mesh states.
 * @returns {Object|null}
 */
function getLivePlacementData() {
  const baseData = store.get('placementData');
  if (!baseData) return null;

  // Deep clone base placement data
  const liveData = JSON.parse(JSON.stringify(baseData));

  // Overwrite coordinates, sockets, and layers from the live 3D scene elements
  liveData.shards.forEach(shard => {
    const mesh = shardMeshes[shard.key];
    if (mesh) {
      shard.position.x = Math.round(mesh.position.x / VIS_SCALE);
      shard.position.y = Math.round(mesh.position.y / VIS_SCALE);
      shard.position.z = Math.round(mesh.position.z / VIS_SCALE);

      const rawData = shardDataMap[mesh.uuid];
      if (rawData && rawData.layers) {
        shard.layers = JSON.parse(JSON.stringify(rawData.layers));
      }
    }

    (shard.sockets || []).forEach(socket => {
      const socketKey = `${shard.key}.${socket.name}`;
      const group = socketMeshes[socketKey];
      if (group) {
        socket.width = group.userData.width;
        socket.height = group.userData.height;
        socket.pitch = group.userData.pitch;
        socket.offset = group.userData.originalOffset;
        socket.faceSign = group.userData.faceSign;
        socket.rotation = group.userData.rotation;
      }
    });
  });

  return liveData;
}

/**
 * Initializes the validator drawer panel and hooks it to the validation request events.
 * @param {HTMLButtonElement} validatorBtn 
 */
export function initValidatorPanel(validatorBtn) {
  const validatorDrawer = document.createElement('div');
  validatorDrawer.id = 'validator-drawer';
  validatorDrawer.className = 'ax-drawer';
  validatorDrawer.innerHTML = `
    <h3 class="ax-section-title">Стек ошибок</h3>
    <div class="validator-list" id="validator-list">
      <div class="validator-empty">Ошибок не обнаружено. Система функционирует нормально.</div>
    </div>
    <div class="validator-actions" style="display: grid; grid-template-columns: repeat(2, 1fr); gap: 8px; margin-top: 12px; border-top: 1px solid var(--ax-border-subtle); padding-top: 12px;">
      <button class="ax-btn ax-btn--secondary ax-btn--sm" id="val-fix-errors-btn">Фикс ошибок</button>
      <button class="ax-btn ax-btn--secondary ax-btn--sm" id="val-fix-warnings-btn">Фикс предупреждений</button>
      <button class="ax-btn ax-btn--secondary ax-btn--sm" id="val-fix-all-btn">Фикс всего</button>
      <button class="ax-btn ax-btn--secondary ax-btn--sm" id="val-revalidate-btn">Перепроверить</button>
    </div>
  `;
  document.body.appendChild(validatorDrawer);

  let currentIssues = [];

  const applyIssueFix = async (issue) => {
    if (!issue.fixable) return;
    const { actionType, socketKey, width, height, pitch, offset, faceSign, rotation } = issue.fixData;
    const group = socketMeshes[socketKey];
    if (!group) return;

    if (actionType === 'resize_socket') {
      rebuildSocket(group.userData.shardKey, group.userData.socketName, width, height, pitch, group.userData.originalOffset, group.userData.faceSign, group.userData.rotation);
    } else if (actionType === 'move_socket') {
      rebuildSocket(group.userData.shardKey, group.userData.socketName, group.userData.width, group.userData.height, group.userData.pitch, offset, group.userData.faceSign, group.userData.rotation);
    } else if (actionType === 'flip_socket') {
      rebuildSocket(group.userData.shardKey, group.userData.socketName, group.userData.width, group.userData.height, group.userData.pitch, group.userData.originalOffset, faceSign, group.userData.rotation);
    }
  };

  const updateValidatorBtnStyle = () => {
    if (validatorDrawer.classList.contains('open')) return;
    const errorsCount = currentIssues.filter(i => i.type === 'error').length;
    const warningsCount = currentIssues.filter(i => i.type === 'warning').length;

    validatorBtn.classList.remove('ax-btn--danger', 'ax-btn--warning', 'ax-btn--success', 'active');

    if (errorsCount > 0) {
      validatorBtn.classList.add('ax-btn--danger');
      validatorBtn.textContent = `Ошибок: ${errorsCount}`;
    } else if (warningsCount > 0) {
      validatorBtn.classList.add('ax-btn--warning');
      validatorBtn.textContent = `Предупреждений: ${warningsCount}`;
    } else {
      validatorBtn.classList.add('ax-btn--success');
      validatorBtn.textContent = 'Ошибок нет';
    }
  };

  // Center align validator drawer center to button center
  const updatePosition = () => {
    const btnRect = validatorBtn.getBoundingClientRect();
    const drawerWidth = 420;
    const targetLeft = btnRect.left + btnRect.width / 2 - drawerWidth / 2;
    const safeLeft = Math.max(16, Math.min(window.innerWidth - drawerWidth - 16, targetLeft));
    validatorDrawer.style.left = safeLeft + 'px';
  };

  let closeTimeout = null;

  const openDrawer = () => {
    if (closeTimeout) clearTimeout(closeTimeout);
    updatePosition();
    validatorDrawer.classList.add('open');
    validatorBtn.classList.remove('ax-btn--danger', 'ax-btn--warning', 'ax-btn--success');
    validatorBtn.classList.add('active');
  };

  const closeDrawer = () => {
    if (closeTimeout) clearTimeout(closeTimeout);
    closeTimeout = setTimeout(() => {
      validatorDrawer.classList.remove('open');
      validatorBtn.classList.remove('active');
      updateValidatorBtnStyle();
    }, 200);
  };

  validatorBtn.addEventListener('click', (e) => {
    e.stopPropagation();
    if (validatorDrawer.classList.contains('open')) {
      closeDrawer();
    } else {
      openDrawer();
    }
  });

  validatorBtn.addEventListener('mouseleave', closeDrawer);

  validatorDrawer.addEventListener('mouseenter', () => {
    if (closeTimeout) clearTimeout(closeTimeout);
  });
  validatorDrawer.addEventListener('mouseleave', closeDrawer);

  // Bind bulk fix actions
  document.getElementById('val-fix-errors-btn').addEventListener('click', async () => {
    const fixable = currentIssues.filter(i => i.type === 'error' && i.fixable);
    if (fixable.length === 0) {
      import('./toast.js').then(({ showToast }) => showToast('Нет автоисправлений для ошибок', 'info'));
      return;
    }
    for (const issue of fixable) {
      await applyIssueFix(issue);
    }
    const routes = store.get('routesData');
    if (routes) drawRoutes(routes);
    emit(EVENTS.VALIDATION_REQ);
    await saveAllLayoutChanges();
    import('./toast.js').then(({ showToast }) => showToast(`Исправлено ошибок: ${fixable.length}`, 'success'));
  });

  document.getElementById('val-fix-warnings-btn').addEventListener('click', async () => {
    const fixable = currentIssues.filter(i => i.type === 'warning' && i.fixable);
    if (fixable.length === 0) {
      import('./toast.js').then(({ showToast }) => showToast('Нет автоисправлений для предупреждений', 'info'));
      return;
    }
    for (const issue of fixable) {
      await applyIssueFix(issue);
    }
    const routes = store.get('routesData');
    if (routes) drawRoutes(routes);
    emit(EVENTS.VALIDATION_REQ);
    await saveAllLayoutChanges();
    import('./toast.js').then(({ showToast }) => showToast(`Исправлено предупреждений: ${fixable.length}`, 'success'));
  });

  document.getElementById('val-fix-all-btn').addEventListener('click', async () => {
    const fixable = currentIssues.filter(i => i.fixable);
    if (fixable.length === 0) {
      import('./toast.js').then(({ showToast }) => showToast('Нет доступных автоисправлений', 'info'));
      return;
    }
    for (const issue of fixable) {
      await applyIssueFix(issue);
    }
    const routes = store.get('routesData');
    if (routes) drawRoutes(routes);
    emit(EVENTS.VALIDATION_REQ);
    await saveAllLayoutChanges();
    import('./toast.js').then(({ showToast }) => showToast(`Исправлено проблем: ${fixable.length}`, 'success'));
  });

  document.getElementById('val-revalidate-btn').addEventListener('click', () => {
    emit(EVENTS.VALIDATION_REQ);
    import('./toast.js').then(({ showToast }) => showToast('Стек ошибок обновлен', 'info'));
  });

  on(EVENTS.VALIDATION_REQ, () => {
    const routes = store.get('routesData');
    if (!routes) return;

    // 1. Run standard geometry validation
    currentIssues = runValidation(routes, shardMeshes, socketMeshes, VIS_SCALE);

    const problematic = new Set();
    currentIssues.forEach(issue => {
      if (issue.affectedSockets) {
        issue.affectedSockets.forEach(k => problematic.add(k));
      }
    });
    store.set('problematicSockets', Array.from(problematic));

    // 2. Run SDK TOML Linter validation on the live placement coordinates representation
    const livePlacement = getLivePlacementData();
    if (livePlacement) {
      const lintIssues = lintScene(livePlacement);
      lintIssues.forEach(issue => {
        currentIssues.push({
          type: issue.severity, // 'error' or 'warning'
          message: `[${issue.file}] ${issue.message}`,
          fixable: false
        });
      });
    }

    // Sort issues: errors first, then warnings
    currentIssues.sort((a, b) => {
      if (a.type === 'error' && b.type !== 'error') return -1;
      if (a.type !== 'error' && b.type === 'error') return 1;
      return 0;
    });

    updateValidatorBtnStyle();

    const listContainer = document.getElementById('validator-list');
    if (!listContainer) return;

    if (currentIssues.length === 0) {
      listContainer.innerHTML = '<div class="validator-empty">Ошибок не обнаружено. Система функционирует нормально.</div>';
      return;
    }

    listContainer.innerHTML = '';
    currentIssues.forEach(issue => {
      const item = document.createElement('div');
      item.className = `validator-item ${issue.type}`;

      const header = document.createElement('div');
      header.className = 'validator-item-header';
      header.innerHTML = `<span>${issue.type === 'error' ? 'Ошибка' : 'Предупреждение'}</span>`;

      const body = document.createElement('div');
      body.className = 'validator-item-body';
      body.textContent = issue.message;

      item.appendChild(header);
      item.appendChild(body);

      if (issue.fixable) {
        const fixBtn = document.createElement('button');
        fixBtn.className = 'ax-btn ax-btn--secondary ax-btn--sm';
        fixBtn.textContent = issue.fixLabel;
        fixBtn.addEventListener('click', async () => {
          await applyIssueFix(issue);
          const routes = store.get('routesData');
          if (routes) drawRoutes(routes);
          emit(EVENTS.VALIDATION_REQ);
          await saveAllLayoutChanges();
        });
        item.appendChild(fixBtn);
      }

      listContainer.appendChild(item);
    });

    // Update socket 3D visual states based on new validation results
    updateAllSocketVisuals();
  });
}
