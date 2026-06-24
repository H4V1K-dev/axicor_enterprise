/**
 * @fileoverview toolbar.js — Bottom panel toolbar, mode switching, and layout saving coordination.
 */

import { saveAllLayoutChanges } from './sidebar.js';
import { updateFocusVisuals } from '../editor.js';
import { drawRoutes, buildSceneData } from '../scene_builder.js';
import { store } from '../store/store.js';
import { on, EVENTS } from '../store/event_bus.js';
import { deselectAll } from '../editor/selection.js';

/**
 * Initializes and builds the bottom panel toolbar.
 * @returns {{ physicsBtn: HTMLButtonElement, validatorBtn: HTMLButtonElement }} Buttons for sub-drawers to bind to.
 */
export function initToolbar() {
  // Container for left dock objects
  const leftContainer = document.createElement('div');
  leftContainer.id = 'bottom-left-container';
 
  // 0. Settings button (сама является интерактивной плашкой перед Граф, Пины, Pix)
  const settingsBtn = document.createElement('button');
  settingsBtn.id = 'settings-trigger-btn';
  settingsBtn.className = 'ax-panel bottom-floating-panel interactive-panel ax-btn--icon';
  settingsBtn.title = 'Настройки';
  settingsBtn.style.display = 'flex';
  settingsBtn.style.alignItems = 'center';
  settingsBtn.style.justifyContent = 'center';
  settingsBtn.style.padding = '8px';
  settingsBtn.style.cursor = 'pointer';
  settingsBtn.innerHTML = `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-settings"><path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.1a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"/><circle cx="12" cy="12" r="3"/></svg>`;
  leftContainer.appendChild(settingsBtn);

  // 1. Switch modes panel (отдельный объект)
  const modePanel = document.createElement('div');
  modePanel.id = 'mode-switch-panel';
  modePanel.className = 'ax-panel bottom-floating-panel';
  
  const mode1 = document.createElement('button');
  mode1.id = 'mode-btn-1';
  mode1.className = 'ax-btn active';
  mode1.textContent = 'Граф';
 
  const mode2 = document.createElement('button');
  mode2.id = 'mode-btn-2';
  mode2.className = 'ax-btn';
  mode2.textContent = 'Пины';
 
  const mode3 = document.createElement('button');
  mode3.id = 'mode-btn-3';
  mode3.className = 'ax-btn';
  mode3.textContent = 'Pix';
  
  modePanel.appendChild(mode1);
  modePanel.appendChild(mode2);
  modePanel.appendChild(mode3);
  leftContainer.appendChild(modePanel);
 
  // 2. Hierarchy button (сама является интерактивной плашкой)
  const hierarchyBtn = document.createElement('button');
  hierarchyBtn.id = 'hierarchy-toggle-btn';
  hierarchyBtn.className = 'ax-panel bottom-floating-panel interactive-panel';
  hierarchyBtn.textContent = 'Иерархия';
  leftContainer.appendChild(hierarchyBtn);

  // 3. Validator button (сама является интерактивной плашкой)
  const validatorBtn = document.createElement('button');
  validatorBtn.id = 'validator-toggle-btn';
  validatorBtn.className = 'ax-panel bottom-floating-panel interactive-panel';
  validatorBtn.textContent = 'Стек ошибок';
  leftContainer.appendChild(validatorBtn);
 
  document.body.appendChild(leftContainer);
 
  // 4. Save & Delete buttons (сама является интерактивной плашкой справа внизу)
  const saveContainer = document.createElement('div');
  saveContainer.id = 'bottom-right-container';
  saveContainer.style.gap = '8px';
  
  const deleteBtn = document.createElement('button');
  deleteBtn.id = 'delete-selected-btn';
  deleteBtn.className = 'ax-panel bottom-floating-panel interactive-panel';
  deleteBtn.textContent = 'Удалить';
  deleteBtn.style.display = 'none'; // Hidden by default
  deleteBtn.style.backgroundColor = 'rgba(239, 68, 68, 0.15)';
  deleteBtn.style.borderColor = 'rgba(239, 68, 68, 0.45)';
  deleteBtn.style.color = '#fca5a5';

  deleteBtn.addEventListener('mouseenter', () => {
    deleteBtn.style.backgroundColor = 'rgba(239, 68, 68, 0.3)';
    deleteBtn.style.borderColor = 'rgba(239, 68, 68, 0.7)';
  });
  deleteBtn.addEventListener('mouseleave', () => {
    deleteBtn.style.backgroundColor = 'rgba(239, 68, 68, 0.15)';
    deleteBtn.style.borderColor = 'rgba(239, 68, 68, 0.45)';
  });

  deleteBtn.addEventListener('click', () => {
    const selShardKey = store.get('selectedShardKey');
    const selSocketKey = store.get('selectedSocketKey');
    const context = getDeletionContext();
    if (!context) return;

    if (context.scenario === 1 || context.scenario === 2) {
      // Single step simple confirmation
      showCustomConfirmModal(context, {
        onConfirm: () => { executeDeletion(context, 'delete'); }
      });
    } else if (context.scenario === 3) {
      // Step 1: simple confirmation
      const step1Context = {
        type: 'shard',
        key: selShardKey,
        scenario: 1,
        message: `Вы уверены, что хотите удалить шард <strong>${selShardKey}</strong>?`
      };
      showCustomConfirmModal(step1Context, {
        onConfirm: () => {
          // Step 2: secondary confirmation with context
          showCustomConfirmModal(context, {
            onConfirm: () => { executeDeletion(context, 'delete'); }
          });
        }
      });
    } else if (context.scenario === 4) {
      // Step 1: simple confirmation
      const step1Context = {
        type: 'shard',
        key: selShardKey,
        scenario: 1,
        message: `Вы уверены, что хотите удалить шард <strong>${selShardKey}</strong>?`
      };
      showCustomConfirmModal(step1Context, {
        onConfirm: () => {
          // Step 2: secondary confirmation with connection details
          showCustomConfirmModal(context, {
            onConfirm: () => { executeDeletion(context, 'delete'); },
            onDisconnect: () => { executeDeletion(context, 'disconnect'); },
            onDeleteWithConnections: () => { executeDeletion(context, 'delete_with_connections'); }
          });
        }
      });
    } else if (context.scenario === 5) {
      // Socket with connections: single step with disconnect and delete pair options
      showCustomConfirmModal(context, {
        onConfirm: () => { executeDeletion(context, 'delete'); },
        onDisconnect: () => { executeDeletion(context, 'disconnect'); },
        onDeletePair: () => { executeDeletion(context, 'delete_pair'); }
      });
    }
  });
  
  const historyContainer = document.createElement('div');
  historyContainer.id = 'history-panel-container';
  historyContainer.className = 'ax-panel bottom-floating-panel';
  historyContainer.style.display = 'flex';
  historyContainer.style.alignItems = 'center';
  historyContainer.style.gap = '4px';
  historyContainer.style.padding = '4px 8px';
  historyContainer.style.height = '36px';
  historyContainer.style.boxSizing = 'border-box';

  const undoBtn = document.createElement('button');
  undoBtn.id = 'history-undo-btn';
  undoBtn.className = 'ax-btn';
  undoBtn.title = 'Undo (Ctrl+Z)';
  undoBtn.innerHTML = `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-undo-2"><path d="M9 14 4 9l5-5"/><path d="M4 9h10.5a5.5 5.5 0 0 1 5.5 5.5v0a5.5 5.5 0 0 1-5.5 5.5H11"/></svg>`;

  const redoBtn = document.createElement('button');
  redoBtn.id = 'history-redo-btn';
  redoBtn.className = 'ax-btn';
  redoBtn.title = 'Redo (Ctrl+Y)';
  redoBtn.innerHTML = `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-redo-2"><path d="M15 14l5-5-5-5"/><path d="M20 9H9.5A5.5 5.5 0 0 0 4 14.5v0A5.5 5.5 0 0 0 9.5 20H13"/></svg>`;

  historyContainer.appendChild(undoBtn);
  historyContainer.appendChild(redoBtn);

  const saveAllBtn = document.createElement('button');
  saveAllBtn.id = 'save-all-btn';
  saveAllBtn.className = 'ax-panel bottom-floating-panel interactive-panel';
  saveAllBtn.textContent = 'Сохранить изменения';
  saveAllBtn.addEventListener('click', saveAllLayoutChanges);
  
  saveContainer.appendChild(deleteBtn);
  saveContainer.appendChild(historyContainer);
  saveContainer.appendChild(saveAllBtn);
  
  document.body.appendChild(saveContainer);

  // Show/hide delete button based on selection state
  on(EVENTS.SELECTION_CHANGED, ({ type }) => {
    if (type === 'shard' || type === 'socket') {
      deleteBtn.style.display = '';
    } else {
      deleteBtn.style.display = 'none';
    }
  });

  // Watch for unsaved changes to make the button pulse green
  store.on('hasUnsavedChanges', (val) => {
    if (val) {
      saveAllBtn.classList.add('save-all-btn-unsaved');
    } else {
      saveAllBtn.classList.remove('save-all-btn-unsaved');
    }
  });

  if (store.get('hasUnsavedChanges')) {
    saveAllBtn.classList.add('save-all-btn-unsaved');
  }

  // Reactive store connection mode update
  store.on('connectionMode', (m) => {
    [1, 2, 3].forEach(modeVal => {
      const btn = document.getElementById(`mode-btn-${modeVal}`);
      if (btn) {
        if (modeVal === m) {
          btn.classList.add('active');
        } else {
          btn.classList.remove('active');
        }
      }
    });
    const routes = store.get('routesData');
    if (routes) drawRoutes(routes);
  });

  // Automatically switch connectionMode when activeMode changes:
  // if inspect -> Graph (1), otherwise Pins (2)
  store.on('activeMode', (modeName) => {
    if (modeName === 'inspect') {
      store.set('connectionMode', 1);
    } else {
      store.set('connectionMode', 2);
    }
  });

  // Wire up mode button event listeners
  const modes = [1, 2, 3];
  modes.forEach(m => {
    const btn = document.getElementById(`mode-btn-${m}`);
    if (btn) {
      btn.addEventListener('click', () => {
        store.set('connectionMode', m);
        updateFocusVisuals();
      });
    }
  });

  return { hierarchyBtn, validatorBtn };
}

function getDeletionContext() {
  const selShardKey = store.get('selectedShardKey');
  const selSocketKey = store.get('selectedSocketKey');
  const placementData = store.get('placementData') || { shards: [], connections: [] };

  if (selShardKey) {
    const shard = placementData.shards.find(s => s.key === selShardKey);
    const sockets = shard ? shard.sockets || [] : [];
    
    const shardConns = (placementData.connections || []).filter(c => 
      c.from === selShardKey || c.to === selShardKey
    );

    if (sockets.length === 0) {
      return {
        type: 'shard',
        key: selShardKey,
        scenario: 1,
        message: `Вы уверены, что хотите удалить шард <strong>${selShardKey}</strong>?`,
        connectionsCount: 0
      };
    } else if (shardConns.length === 0) {
      return {
        type: 'shard',
        key: selShardKey,
        scenario: 3,
        message: `У шарда <strong>${selShardKey}</strong> есть размещенные сокеты, но ни один из них не подключен. Действительно удалить шард?`,
        connectionsCount: 0
      };
    } else {
      return {
        type: 'shard',
        key: selShardKey,
        scenario: 4,
        message: `У шарда <strong>${selShardKey}</strong> есть размещенные сокеты и присутствуют активные связи (${shardConns.length}) с другими шардами. При удалении шарда все его связи будут разорваны.`,
        connectionsCount: shardConns.length,
        connections: shardConns
      };
    }
  } else if (selSocketKey) {
    const lastDot = selSocketKey.lastIndexOf('.');
    if (lastDot === -1) return null;
    const shardKey = selSocketKey.substring(0, lastDot);
    const socketName = selSocketKey.substring(lastDot + 1);

    const socketConn = (placementData.connections || []).find(c => 
      (c.from === shardKey && c.from_socket === socketName) ||
      (c.to === shardKey && c.to_socket === socketName)
    );

    if (!socketConn) {
      return {
        type: 'socket',
        key: selSocketKey,
        scenario: 2,
        message: `Вы уверены, что хотите удалить сокет <strong>${socketName}</strong>?`,
        shardKey,
        socketName
      };
    } else {
      const peerShardKey = socketConn.from === shardKey ? socketConn.to : socketConn.from;
      const peerSocketName = socketConn.from === shardKey ? socketConn.to_socket : socketConn.from_socket;
      const peerSocketKey = `${peerShardKey}.${peerSocketName}`;

      return {
        type: 'socket',
        key: selSocketKey,
        scenario: 5,
        message: `У сокета <strong>${socketName}</strong> есть активная связь. Вы можете удалить сокет (связь будет разорвана), разорвать связь или удалить оба сокета.`,
        shardKey,
        socketName,
        connection: socketConn,
        peerShardKey,
        peerSocketName,
        peerSocketKey
      };
    }
  }
  return null;
}

function showCustomConfirmModal(context, actionsMap) {
  const existingModal = document.getElementById('ax-confirm-modal');
  if (existingModal) {
    if (existingModal._cleanup) {
      existingModal._cleanup();
    }
    existingModal.remove();
  }

  // Set modalActive state in store to block other inputs
  store.set('modalActive', true);

  const modalOverlay = document.createElement('div');
  modalOverlay.id = 'ax-confirm-modal';
  modalOverlay.style.position = 'fixed';
  modalOverlay.style.top = '0';
  modalOverlay.style.left = '0';
  modalOverlay.style.width = '100vw';
  modalOverlay.style.height = '100vh';
  modalOverlay.style.backgroundColor = 'rgba(0, 0, 0, 0.6)';
  modalOverlay.style.backdropFilter = 'blur(6px)';
  modalOverlay.style.display = 'flex';
  modalOverlay.style.alignItems = 'center';
  modalOverlay.style.justifyContent = 'center';
  modalOverlay.style.zIndex = '9999';

  const modalBox = document.createElement('div');
  modalBox.className = 'ax-panel';
  modalBox.style.width = '420px';
  modalBox.style.padding = '24px';
  modalBox.style.display = 'flex';
  modalBox.style.flexDirection = 'column';
  modalBox.style.gap = '16px';
  modalBox.style.boxShadow = 'var(--ax-shadow-lg, 0 10px 25px rgba(0, 0, 0, 0.3))';

  const header = document.createElement('div');
  header.style.fontSize = '16px';
  header.style.fontWeight = '600';
  header.style.color = '#f87171'; // Red-ish warning header color
  header.innerHTML = `⚠️ Предупреждение`;
  modalBox.appendChild(header);

  const body = document.createElement('div');
  body.style.fontSize = '13px';
  body.style.lineHeight = '1.6';
  body.innerHTML = context.message;
  modalBox.appendChild(body);

  const actions = document.createElement('div');
  actions.style.display = 'flex';
  actions.style.flexDirection = 'column';
  actions.style.gap = '10px';
  actions.style.marginTop = '12px';

  const closeModal = () => {
    if (modalOverlay._cleanup) {
      modalOverlay._cleanup();
    }
    store.set('modalActive', false);
    modalOverlay.remove();
  };

  const handleKeyDown = (e) => {
    if (e.key === 'Escape') {
      closeModal();
    }
  };
  window.addEventListener('keydown', handleKeyDown);
  
  modalOverlay._cleanup = () => {
    window.removeEventListener('keydown', handleKeyDown);
  };

  modalOverlay.addEventListener('click', (e) => {
    if (e.target === modalOverlay) {
      closeModal();
    }
  });

  if (actionsMap.onConfirm) {
    const confirmBtn = document.createElement('button');
    confirmBtn.className = 'ax-btn ax-btn--danger';
    confirmBtn.style.width = '100%';
    confirmBtn.style.padding = '10px';
    confirmBtn.style.cursor = 'pointer';
    confirmBtn.style.fontWeight = '600';
    confirmBtn.textContent = 'Удалить выбраный';
    confirmBtn.addEventListener('click', () => {
      actionsMap.onConfirm();
      closeModal();
    });
    actions.appendChild(confirmBtn);
  }

  if (actionsMap.onDisconnect) {
    const disconnectBtn = document.createElement('button');
    disconnectBtn.className = 'ax-btn ax-btn--warning';
    disconnectBtn.style.width = '100%';
    disconnectBtn.style.padding = '10px';
    disconnectBtn.style.cursor = 'pointer';
    disconnectBtn.style.fontWeight = '600';
    disconnectBtn.textContent = 'Разорвать связь';
    disconnectBtn.addEventListener('click', () => {
      actionsMap.onDisconnect();
      closeModal();
    });
    actions.appendChild(disconnectBtn);
  }

  if (actionsMap.onDeletePair) {
    const deletePairBtn = document.createElement('button');
    deletePairBtn.className = 'ax-btn ax-btn--danger';
    deletePairBtn.style.width = '100%';
    deletePairBtn.style.padding = '10px';
    deletePairBtn.style.cursor = 'pointer';
    deletePairBtn.style.fontWeight = '600';
    deletePairBtn.textContent = 'Удалить пару';
    deletePairBtn.addEventListener('click', () => {
      actionsMap.onDeletePair();
      closeModal();
    });
    actions.appendChild(deletePairBtn);
  }

  if (actionsMap.onDeleteWithConnections) {
    const deleteWithConnsBtn = document.createElement('button');
    deleteWithConnsBtn.className = 'ax-btn ax-btn--danger';
    deleteWithConnsBtn.style.width = '100%';
    deleteWithConnsBtn.style.padding = '10px';
    deleteWithConnsBtn.style.cursor = 'pointer';
    deleteWithConnsBtn.style.fontWeight = '600';
    deleteWithConnsBtn.textContent = 'Удалить с связями';
    deleteWithConnsBtn.addEventListener('click', () => {
      actionsMap.onDeleteWithConnections();
      closeModal();
    });
    actions.appendChild(deleteWithConnsBtn);
  }

  const cancelBtn = document.createElement('button');
  cancelBtn.className = 'ax-btn ax-btn--secondary';
  cancelBtn.style.width = '100%';
  cancelBtn.style.padding = '10px';
  cancelBtn.style.cursor = 'pointer';
  cancelBtn.textContent = 'Отмена';
  cancelBtn.addEventListener('click', () => {
    closeModal();
  });
  actions.appendChild(cancelBtn);

  modalBox.appendChild(actions);
  modalOverlay.appendChild(modalBox);
  document.body.appendChild(modalOverlay);
}

function executeDeletion(context, action) {
  const placementData = store.get('placementData');
  if (!placementData) return;

  if (context.type === 'shard') {
    const shardKey = context.key;
    const shard = placementData.shards.find(s => s.key === shardKey);
    if (!shard) return;
    
    if (action === 'delete') {
      const undoState = JSON.parse(JSON.stringify(shard));
      const description = `Удаление шарда ${shardKey}`;
      import('../store/history_manager.js').then(({ historyManager }) => {
        historyManager.pushAction('delete', 'shard', shardKey, description, undoState, null);
      });

      if (!placementData.deleted_shards) {
        placementData.deleted_shards = [];
      }
      if (!placementData.deleted_shards.includes(shardKey)) {
        placementData.deleted_shards.push(shardKey);
      }
      
      placementData.shards = placementData.shards.filter(s => s.key !== shardKey);
      
      if (placementData.connections) {
        placementData.connections = placementData.connections.filter(c => 
          c.from !== shardKey && c.to !== shardKey
        );
      }
    } else if (action === 'delete_with_connections') {
      const peerSockets = [];
      const connections = [];
      if (context.connections) {
        context.connections.forEach(c => {
          const isFrom = c.from === shardKey;
          const peerShardKey = isFrom ? c.to : c.from;
          const peerSocketName = isFrom ? c.to_socket : c.from_socket;
          const peerShard = placementData.shards.find(s => s.key === peerShardKey);
          if (peerShard && peerShard.sockets) {
            const peerSock = peerShard.sockets.find(s => s.name === peerSocketName);
            if (peerSock) {
              peerSockets.push({
                shardKey: peerShardKey,
                socket: JSON.parse(JSON.stringify(peerSock))
              });
            }
          }
          connections.push(JSON.parse(JSON.stringify(c)));
        });
      }
      const undoState = {
        shard: JSON.parse(JSON.stringify(shard)),
        peerSockets: peerSockets,
        connections: connections
      };
      const description = `Удаление шарда ${shardKey} со связями`;
      import('../store/history_manager.js').then(({ historyManager }) => {
        historyManager.pushAction('delete_with_connections', 'shard', shardKey, description, undoState, null);
      });

      if (!placementData.deleted_shards) {
        placementData.deleted_shards = [];
      }
      if (!placementData.deleted_shards.includes(shardKey)) {
        placementData.deleted_shards.push(shardKey);
      }
      placementData.shards = placementData.shards.filter(s => s.key !== shardKey);

      if (!placementData.deleted_sockets) {
        placementData.deleted_sockets = [];
      }

      if (context.connections) {
        context.connections.forEach(c => {
          const isFrom = c.from === shardKey;
          const peerShardKey = isFrom ? c.to : c.from;
          const peerSocketName = isFrom ? c.to_socket : c.from_socket;
          const peerSocketKey = `${peerShardKey}.${peerSocketName}`;

          if (!placementData.deleted_sockets.includes(peerSocketKey)) {
            placementData.deleted_sockets.push(peerSocketKey);
          }

          const peerShard = placementData.shards.find(s => s.key === peerShardKey);
          if (peerShard && peerShard.sockets) {
            peerShard.sockets = peerShard.sockets.filter(s => s.name !== peerSocketName);
          }
        });
      }

      if (placementData.connections) {
        placementData.connections = placementData.connections.filter(c => 
          c.from !== shardKey && c.to !== shardKey
        );
      }
    } else if (action === 'disconnect') {
      const connections = [];
      if (context.connections) {
        context.connections.forEach(c => {
          connections.push(JSON.parse(JSON.stringify(c)));
        });
      }
      const description = `Отключение связей шарда ${shardKey}`;
      import('../store/history_manager.js').then(({ historyManager }) => {
        historyManager.pushAction('disconnect', 'connection', shardKey, description, connections, null);
      });

      if (placementData.connections && context.connections) {
        if (!placementData.deleted_connections) {
          placementData.deleted_connections = [];
        }
        
        context.connections.forEach(c => {
          const connKey = `${c.from}.${c.from_socket} -> ${c.to}.${c.to_socket}`;
          if (!placementData.deleted_connections.includes(connKey)) {
            placementData.deleted_connections.push(connKey);
          }
        });

        placementData.connections = placementData.connections.filter(c => 
          c.from !== shardKey && c.to !== shardKey
        );
      }
    }
  } else if (context.type === 'socket') {
    const shardKey = context.shardKey;
    const socketName = context.socketName;
    const socketKey = context.key;

    if (action === 'delete') {
      const shard = placementData.shards.find(s => s.key === shardKey);
      const socket = shard && shard.sockets ? shard.sockets.find(s => s.name === socketName) : null;
      const undoState = socket ? JSON.parse(JSON.stringify(socket)) : null;
      const description = `Удаление сокета ${socketName}`;
      import('../store/history_manager.js').then(({ historyManager }) => {
        historyManager.pushAction('delete', 'socket', socketKey, description, undoState, null);
      });

      if (!placementData.deleted_sockets) {
        placementData.deleted_sockets = [];
      }
      if (!placementData.deleted_sockets.includes(socketKey)) {
        placementData.deleted_sockets.push(socketKey);
      }

      if (shard && shard.sockets) {
        shard.sockets = shard.sockets.filter(s => s.name !== socketName);
      }

      if (placementData.connections) {
        placementData.connections = placementData.connections.filter(c => 
          !(c.from === shardKey && c.from_socket === socketName) &&
          !(c.to === shardKey && c.to_socket === socketName)
        );
      }
    } else if (action === 'delete_pair') {
      const shard = placementData.shards.find(s => s.key === shardKey);
      const socket = shard && shard.sockets ? shard.sockets.find(s => s.name === socketName) : null;
      const peerShard = context.peerShardKey ? placementData.shards.find(s => s.key === context.peerShardKey) : null;
      const peerSocket = peerShard && peerShard.sockets ? peerShard.sockets.find(s => s.name === context.peerSocketName) : null;
      
      const undoState = {
        socketA: socket ? JSON.parse(JSON.stringify(socket)) : null,
        socketB: peerSocket ? { shardKey: context.peerShardKey, socket: JSON.parse(JSON.stringify(peerSocket)) } : null,
        connection: context.connection ? JSON.parse(JSON.stringify(context.connection)) : null
      };
      const description = `Удаление пары сокетов ${socketName} и ${context.peerSocketName}`;
      import('../store/history_manager.js').then(({ historyManager }) => {
        historyManager.pushAction('delete_pair', 'socket', socketKey, description, undoState, null);
      });

      if (!placementData.deleted_sockets) {
        placementData.deleted_sockets = [];
      }
      if (!placementData.deleted_sockets.includes(socketKey)) {
        placementData.deleted_sockets.push(socketKey);
      }
      if (context.peerSocketKey && !placementData.deleted_sockets.includes(context.peerSocketKey)) {
        placementData.deleted_sockets.push(context.peerSocketKey);
      }

      if (shard && shard.sockets) {
        shard.sockets = shard.sockets.filter(s => s.name !== socketName);
      }

      if (context.peerShardKey && context.peerSocketName) {
        if (peerShard && peerShard.sockets) {
          peerShard.sockets = peerShard.sockets.filter(s => s.name !== context.peerSocketName);
        }
      }

      if (placementData.connections) {
        placementData.connections = placementData.connections.filter(c => 
          !(c.from === shardKey && c.from_socket === socketName) &&
          !(c.to === shardKey && c.to_socket === socketName)
        );
      }
    } else if (action === 'disconnect') {
      const undoState = context.connection ? JSON.parse(JSON.stringify(context.connection)) : null;
      const description = `Отключение связи сокета ${socketName}`;
      import('../store/history_manager.js').then(({ historyManager }) => {
        historyManager.pushAction('disconnect', 'connection', socketKey, description, undoState, null);
      });

      if (placementData.connections && context.connection) {
        const c = context.connection;
        const connKey = `${c.from}.${c.from_socket} -> ${c.to}.${c.to_socket}`;
        
        if (!placementData.deleted_connections) {
          placementData.deleted_connections = [];
        }
        if (!placementData.deleted_connections.includes(connKey)) {
          placementData.deleted_connections.push(connKey);
        }

        placementData.connections = placementData.connections.filter(conn => 
          !(conn.from === shardKey && conn.from_socket === socketName) &&
          !(conn.to === shardKey && conn.to_socket === socketName)
        );
      }
    }
  }

  store.set('placementData', placementData);
  store.set('hasUnsavedChanges', true);
  
  deselectAll();
  buildSceneData(placementData, true);

  const routes = store.get('routesData');
  if (routes) drawRoutes(routes);
}
