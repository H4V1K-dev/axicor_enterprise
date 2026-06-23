/**
 * @fileoverview settings_panel.js — Settings modal panel for configuring editor preferences and simulation physics.
 */

import { store } from '../store/store.js';
import { emit } from '../store/event_bus.js';
import { showToast } from './toast.js';

/**
 * Initializes the settings trigger button to show the modal panel.
 * @param {HTMLButtonElement} settingsBtn 
 */
export function initSettingsPanel(settingsBtn) {
  if (!settingsBtn) return;
  settingsBtn.addEventListener('click', (e) => {
    e.stopPropagation();
    showSettingsModal();
  });
}

/**
 * Creates and displays the settings modal panel overlay.
 */
export function showSettingsModal() {
  const existingModal = document.getElementById('ax-settings-modal');
  if (existingModal) return;

  // Set modalActive state in store to block other inputs
  store.set('modalActive', true);

  const modalOverlay = document.createElement('div');
  modalOverlay.id = 'ax-settings-modal';
  modalOverlay.className = 'settings-modal-overlay';

  // Read current configuration
  const editorSettings = store.get('editorSettings') || {};
  const placementData = store.get('placementData') || {};
  const simulation = placementData.simulation || {};
  const world = placementData.world || {};
  const layersVis = store.get('layersVisibility') || { 0: true, 1: true, 2: true, 3: true };

  const modalBox = document.createElement('div');
  modalBox.className = 'settings-modal-box';

  let orbitsHtml = '';

  // Calculate dynamic bounding box of all shards in voxels
  const shards = placementData.shards || [];
  let minX = Infinity, maxX = -Infinity;
  let minY = Infinity, maxY = -Infinity;
  let minZ = Infinity, maxZ = -Infinity;

  shards.forEach(s => {
    const w = s.size.w;
    const d = s.size.d;
    const h = s.size.h;
    
    const sx = s.position.x;
    const sy = s.position.y;
    const sz = s.position.z;

    minX = Math.min(minX, sx - w / 2);
    maxX = Math.max(maxX, sx + w / 2);
    minY = Math.min(minY, sy - d / 2);
    maxY = Math.max(maxY, sy + d / 2);
    minZ = Math.min(minZ, sz - h / 2);
    maxZ = Math.max(maxZ, sz + h / 2);
  });

  const width_voxels = shards.length > 0 ? (maxX - minX) : 0;
  const depth_voxels = shards.length > 0 ? (maxY - minY) : 0;
  const height_voxels = shards.length > 0 ? (maxZ - minZ) : 0;

  modalBox.innerHTML = `
    <div class="settings-modal-header">
      <h3>НАСТРОЙКИ</h3>
      <button id="settings-close-x" class="ax-btn ax-btn--ghost ax-btn--icon" style="padding:4px; font-size:16px; min-width:unset; width:24px; height:24px; color:var(--ax-text-faint);" title="Закрыть">✕</button>
    </div>
    
    <div class="settings-modal-body">
      <div class="settings-modal-sidebar">
        <button class="settings-modal-tab-btn" data-tab="tab-interface">Интерфейс</button>
        <button class="settings-modal-tab-btn active" data-tab="tab-editor">Редактор</button>
        <button class="settings-modal-tab-btn" data-tab="tab-physics">Симуляция</button>
        <button class="settings-modal-tab-btn" data-tab="tab-cloud">Облако</button>
      </div>
      
      <div class="settings-modal-content">
        <!-- TAB 1: INTERFACE -->
        <div id="tab-interface" class="settings-modal-tab-content">
          <div class="settings-modal-section">
            <div class="settings-modal-section-title">Отображение</div>
            
            <div class="settings-modal-field">
              <label>Тема оформления:</label>
              <select id="set-theme" class="settings-select">
                <option value="dark" ${editorSettings.theme === 'dark' || !editorSettings.theme ? 'selected' : ''}>Тёмная (Glassmorphism)</option>
                <option value="light" disabled>Светлая (🔒 Блокировано)</option>
                <option value="contrast" disabled>Высококонтрастная (🔒 Блокировано)</option>
              </select>
            </div>
            
            <div class="settings-modal-field">
              <label title="Показывать всплывающие подсказки при наведении на шарды">Всплывающие подсказки (Tooltips):</label>
              <input type="checkbox" id="set-show-tooltips" ${editorSettings.show_tooltips !== false ? 'checked' : ''} style="accent-color: #10b981; width: 16px; height: 16px; cursor: pointer;">
            </div>
            
            <div class="settings-modal-field">
              <label title="Показывать связывающие кабели на сцене">Отображать связи (Connections):</label>
              <input type="checkbox" id="set-show-connections" ${editorSettings.show_connections !== false ? 'checked' : ''} style="accent-color: #10b981; width: 16px; height: 16px; cursor: pointer;">
            </div>
          </div>
          

        </div>

        <!-- TAB 2: EDITOR -->
        <div id="tab-editor" class="settings-modal-tab-content active">
          <div class="settings-modal-section">
            <div class="settings-modal-section-title">Привязки и Сетка</div>
            
            <div class="settings-modal-field">
              <label title="Шаг отрисовки линий сетки на сцене в вокселях">Шаг сетки:</label>
              <div class="settings-stepper">
                <button class="settings-stepper-btn minus" data-input-id="set-grid-step" data-step="1">−</button>
                <input type="number" id="set-grid-step" class="settings-stepper-input" value="${editorSettings.grid_step || 100}" min="10" max="1000" step="1">
                <button class="settings-stepper-btn plus" data-input-id="set-grid-step" data-step="1">+</button>
              </div>
            </div>
            
            <div class="settings-modal-field">
              <label title="Шаг округления координат при перемещении объектов">Привязка перемещения (Snap):</label>
              <div class="settings-stepper">
                <button class="settings-stepper-btn minus" data-input-id="set-snap-step" data-step="1">−</button>
                <input type="number" id="set-snap-step" class="settings-stepper-input" value="${editorSettings.snap_step || 1}" min="1" max="50" step="1">
                <button class="settings-stepper-btn plus" data-input-id="set-snap-step" data-step="1">+</button>
              </div>
            </div>
            
            <div class="settings-modal-field">
              <label title="Шаг изменения размера шардов в вокселях">Привязка изменения размера:</label>
              <div class="settings-stepper">
                <button class="settings-stepper-btn minus" data-input-id="set-resize-step" data-step="1">−</button>
                <input type="number" id="set-resize-step" class="settings-stepper-input" value="${editorSettings.resize_step || 10}" min="1" max="100" step="1">
                <button class="settings-stepper-btn plus" data-input-id="set-resize-step" data-step="1">+</button>
              </div>
            </div>
            
            <div class="settings-modal-field">
              <label title="Шаг разбиения кабелей при ручной трассировке в вокселях">Шаг трассировки кабеля:</label>
              <div class="settings-stepper">
                <button class="settings-stepper-btn minus" data-input-id="set-cable-subdivision" data-step="1">−</button>
                <input type="number" id="set-cable-subdivision" class="settings-stepper-input" value="${editorSettings.cable_subdivision_step || 30}" min="10" max="100" step="1">
                <button class="settings-stepper-btn plus" data-input-id="set-cable-subdivision" data-step="1">+</button>
              </div>
            </div>
          </div>
          
          <div class="settings-modal-section">
            <div class="settings-modal-section-title">Объекты по умолчанию</div>
            
            <div class="settings-modal-field">
              <label>Шард (W, D, H):</label>
              <div class="settings-inline-inputs">
                <input type="number" id="set-def-shard-w" value="${editorSettings.default_shard_w || 32}" min="10" max="1024">
                <span class="separator">X</span>
                <input type="number" id="set-def-shard-d" value="${editorSettings.default_shard_d || 32}" min="10" max="1024">
                <span class="separator">|</span>
                <input type="number" id="set-def-shard-h" value="${editorSettings.default_shard_h || 16}" min="10" max="256">
              </div>
            </div>
            
            <div class="settings-modal-field">
              <label>Сокет (W, H, P):</label>
              <div class="settings-inline-inputs">
                <input type="number" id="set-def-socket-w" value="${editorSettings.default_socket_w || 4}" min="1" max="64">
                <span class="separator">|</span>
                <input type="number" id="set-def-socket-h" value="${editorSettings.default_socket_h || 4}" min="1" max="64">
                <span class="separator">|</span>
                <input type="number" id="set-def-socket-pitch" value="${editorSettings.default_socket_pitch || 2}" min="1" max="4">
              </div>
            </div>
          </div>
          
          <div class="settings-modal-section">
            <div class="settings-modal-section-title">Камера</div>
            
            <div class="settings-modal-field">
              <label title="Чувствительность вращения камеры при перетаскивании ViewCube">Чувствительность ViewCube:</label>
              <div class="settings-slider-group">
                <input type="range" id="set-viewcube-sens" class="settings-slider" value="${editorSettings.viewcube_sensitivity || 0.0075}" min="0.001" max="0.03" step="0.0005">
                <span id="viewcube-sens-val" class="settings-slider-value">${(editorSettings.viewcube_sensitivity || 0.0075).toFixed(4)}</span>
              </div>
            </div>
          </div>
          
          <div class="settings-modal-section">
            <div class="settings-modal-section-title">Остальное</div>
            
            <div class="settings-modal-field">
              <label title="Тип отката изменений в истории">Режим Undo/Redo:</label>
              <select id="set-history-mode" class="settings-select">
                <option value="global" ${editorSettings.history_mode === 'global' ? 'selected' : ''}>Глобальный стек</option>
                <option value="per-object" ${editorSettings.history_mode === 'per-object' ? 'selected' : ''}>Локальный стек</option>
              </select>
            </div>
            
            <div class="settings-action-btn-row">
              <button class="ax-btn ax-btn--secondary" id="settings-export-btn" style="flex: 1; padding: 6px 12px; font-size: 11px;" title="Экспорт настроек в JSON файл">Экспорт конфига</button>
              <button class="ax-btn ax-btn--secondary" id="settings-import-btn" style="flex: 1; padding: 6px 12px; font-size: 11px;" title="Импорт настроек из JSON файла">Импорт конфига</button>
              <input type="file" id="settings-import-file" style="display: none;" accept=".json">
            </div>
          </div>
        </div>

        <!-- TAB 3: PHYSICS / SIMULATION -->
        <div id="tab-physics" class="settings-modal-tab-content">
          <div class="settings-modal-section">
            <div class="settings-modal-section-title">Размеры мира</div>
            
            <div class="settings-modal-field">
              <label title="Размеры мира (вычисляются динамически как произведение размера вокселя на габариты занятых вокселей)">Размеры мира:</label>
              <div style="font-family: var(--ax-font-mono); font-size: 12px; font-weight: 600; color: var(--ax-text-secondary);">
                <span id="display-world-w">0</span> X <span id="display-world-d">0</span> | <span id="display-world-h">0</span>
              </div>
              <input type="hidden" id="set-world-w" value="${world.width_um || 25000.0}">
              <input type="hidden" id="set-world-d" value="${world.depth_um || 25000.0}">
              <input type="hidden" id="set-world-h" value="${world.height_um || 6375.0}">
            </div>
          </div>
          
          <div class="settings-modal-section">
            <div class="settings-modal-section-title">Параметры среды</div>
            
            <div class="settings-modal-field">
              <label title="Размер одного вокселя в микрометрах">Размер вокселя (um):</label>
              <div class="settings-stepper">
                <button class="settings-stepper-btn minus" data-input-id="set-voxel-size" data-step="1">−</button>
                <input type="number" id="set-voxel-size" class="settings-stepper-input" value="${Math.round(simulation.voxel_size_um || 25)}" min="1" max="100" step="1">
                <button class="settings-stepper-btn plus" data-input-id="set-voxel-size" data-step="1">+</button>
              </div>
            </div>
            
            <div class="settings-modal-field">
              <label title="Длительность одного тика симуляции в микросекундах">Длительность тика (us):</label>
              <div class="settings-stepper">
                <button class="settings-stepper-btn minus" data-input-id="set-tick-duration" data-step="1">−</button>
                <input type="number" id="set-tick-duration" class="settings-stepper-input" value="${simulation.tick_duration_us || 100}" min="1" max="1000" step="1">
                <button class="settings-stepper-btn plus" data-input-id="set-tick-duration" data-step="1">+</button>
              </div>
            </div>
            
            <div class="settings-modal-field">
              <label title="Скорость распространения сигнала в метрах в секунду">Скорость сигнала (m/s):</label>
              <div class="settings-stepper">
                <button class="settings-stepper-btn minus" data-input-id="set-signal-speed" data-step="0.1">−</button>
                <input type="number" id="set-signal-speed" class="settings-stepper-input" value="${simulation.signal_speed_m_s || 0.5}" min="0.1" max="10" step="0.1">
                <button class="settings-stepper-btn plus" data-input-id="set-signal-speed" data-step="0.1">+</button>
              </div>
            </div>
          </div>
          
          <div class="settings-modal-section">
            <div class="settings-modal-section-title">Параметры роста</div>
            
            <div class="settings-modal-field">
              <label title="Длина сегмента аксона в вокселях">Длина сегмента аксона:</label>
              <div class="settings-stepper">
                <button class="settings-stepper-btn minus" data-input-id="set-segment-len" data-step="1">−</button>
                <input type="number" id="set-segment-len" class="settings-stepper-input" value="${simulation.segment_length_voxels || 2}" min="1" max="50" step="1">
                <button class="settings-stepper-btn plus" data-input-id="set-segment-len" data-step="1">+</button>
              </div>
            </div>
            
            <div class="settings-modal-field">
              <label title="Максимальное число шагов роста аксона">Макс шагов роста аксона:</label>
              <div class="settings-stepper">
                <button class="settings-stepper-btn minus" data-input-id="set-axon-growth" data-step="1">−</button>
                <input type="number" id="set-axon-growth" class="settings-stepper-input" value="${simulation.axon_growth_max_steps || 250}" min="1" max="256" step="1">
                <button class="settings-stepper-btn plus" data-input-id="set-axon-growth" data-step="1">+</button>
              </div>
            </div>
          </div>
        </div>

        <!-- TAB 4: CLOUD -->
        <div id="tab-cloud" class="settings-modal-tab-content">
          <div class="settings-modal-section">
            <div class="settings-modal-section-title">Синхронизация</div>
            
            <div class="settings-modal-field">
              <label>Адрес API Сервера:</label>
              <input type="text" id="set-api-url" class="ax-input" style="width: 180px; text-align: left; padding: 6px 10px; height: 32px; font-size: 12px; opacity: 0.5;" value="${editorSettings.api_url || 'http://localhost:8080'}" disabled>
            </div>
            
            <div class="settings-modal-field">
              <label>Автосохранение в облако:</label>
              <select id="set-auto-save" class="settings-select" style="opacity: 0.5;" disabled>
                <option value="false" selected>Выключено</option>
              </select>
            </div>
            
            <div style="font-size: 11px; color: var(--ax-text-faint); margin-top: 12px; text-align: center; border: 1px dashed rgba(255,255,255,0.08); padding: 10px; border-radius: 8px; line-height: 1.4;">
              🔒 Авторизация и облачное хранилище в данный момент недоступны. Функционал будет развернут в следующем обновлении.
            </div>
          </div>
        </div>
      </div>
    </div>
    
    <div class="settings-modal-footer">
      <button class="ax-btn ax-btn--secondary" id="settings-cancel-btn" style="padding: 8px 20px; font-size:12px; font-weight: 700;">Отмена</button>
      <button class="ax-btn ax-btn--primary" id="settings-apply-btn" style="padding: 8px 24px; font-size:12px; font-weight: 700;">Применить</button>
    </div>
  `;

  modalOverlay.appendChild(modalBox);
  document.body.appendChild(modalOverlay);

  // Dynamic world size calculation logic
  const updateWorldSizeDisplay = () => {
    const voxelSizeEl = modalBox.querySelector('#set-voxel-size');
    const voxel_size_um = parseFloat(voxelSizeEl.value) || 25.0;
    
    const w_um = Math.round(width_voxels * voxel_size_um);
    const d_um = Math.round(depth_voxels * voxel_size_um);
    const h_um = Math.round(height_voxels * voxel_size_um);
    
    modalBox.querySelector('#display-world-w').textContent = `${w_um} um`;
    modalBox.querySelector('#display-world-d').textContent = `${d_um} um`;
    modalBox.querySelector('#display-world-h').textContent = `${h_um} um`;
    
    modalBox.querySelector('#set-world-w').value = w_um;
    modalBox.querySelector('#set-world-d').value = d_um;
    modalBox.querySelector('#set-world-h').value = h_um;
  };

  // Sanitizers for Shard/Socket/World inputs
  const sanitizeIntInput = (selector, minVal, maxVal) => {
    const el = modalBox.querySelector(selector);
    if (!el) return;
    const handler = () => {
      let val = parseInt(el.value);
      if (isNaN(val)) {
        val = minVal;
      }
      val = Math.max(minVal, Math.round(val));
      if (maxVal !== undefined) {
        val = Math.min(maxVal, val);
      }
      el.value = val;
      el.dispatchEvent(new Event('input', { bubbles: true }));
    };
    el.addEventListener('blur', handler);
    el.addEventListener('change', handler);
  };

  const sanitizeFloatInput = (selector, minVal, maxVal) => {
    const el = modalBox.querySelector(selector);
    if (!el) return;
    const handler = () => {
      let val = parseFloat(el.value);
      if (isNaN(val)) {
        val = minVal;
      }
      val = Math.max(minVal, val);
      if (maxVal !== undefined) {
        val = Math.min(maxVal, val);
      }
      el.value = val.toFixed(1);
      el.dispatchEvent(new Event('input', { bubbles: true }));
    };
    el.addEventListener('blur', handler);
    el.addEventListener('change', handler);
  };

  // Wire input limits
  sanitizeIntInput('#set-grid-step', 10, 1000);
  sanitizeIntInput('#set-snap-step', 1, 50);
  sanitizeIntInput('#set-resize-step', 1, 100);
  sanitizeIntInput('#set-cable-subdivision', 10, 100);

  sanitizeIntInput('#set-def-shard-w', 10, 1024);
  sanitizeIntInput('#set-def-shard-d', 10, 1024);
  sanitizeIntInput('#set-def-shard-h', 10, 256);

  sanitizeIntInput('#set-def-socket-w', 1, 64);
  sanitizeIntInput('#set-def-socket-h', 1, 64);
  sanitizeIntInput('#set-def-socket-pitch', 1, 4);

  // Simulation parameters sanitization
  sanitizeIntInput('#set-voxel-size', 1, 100);
  sanitizeIntInput('#set-tick-duration', 1, 1000);
  sanitizeFloatInput('#set-signal-speed', 0.1, 10.0);
  
  // Growth parameters sanitization
  sanitizeIntInput('#set-segment-len', 1, 50);
  sanitizeIntInput('#set-axon-growth', 1, 256);

  // Connect voxel size changes to dynamic world calculations
  const voxelSizeInput = modalBox.querySelector('#set-voxel-size');
  if (voxelSizeInput) {
    voxelSizeInput.addEventListener('input', updateWorldSizeDisplay);
  }
  updateWorldSizeDisplay(); // Initial display render

  // Tab switching logic
  const tabButtons = modalBox.querySelectorAll('.settings-modal-tab-btn');
  const tabContents = modalBox.querySelectorAll('.settings-modal-tab-content');

  tabButtons.forEach(btn => {
    btn.addEventListener('click', () => {
      tabButtons.forEach(b => b.classList.remove('active'));
      tabContents.forEach(c => c.classList.remove('active'));

      btn.classList.add('active');
      const tabId = btn.dataset.tab;
      modalBox.querySelector(`#${tabId}`).classList.add('active');
    });
  });

  // Stepper input event delegation
  modalBox.addEventListener('click', (e) => {
    const stepperBtn = e.target.closest('.settings-stepper-btn');
    if (!stepperBtn) return;
    const inputId = stepperBtn.dataset.inputId;
    const step = parseFloat(stepperBtn.dataset.step || 1);
    const input = modalBox.querySelector(`#${inputId}`);
    if (input) {
      let val = parseFloat(input.value);
      if (isNaN(val)) val = 0;
      
      const min = parseFloat(input.getAttribute('min'));
      const max = parseFloat(input.getAttribute('max'));
      
      if (stepperBtn.classList.contains('plus')) {
        val += step;
      } else {
        val -= step;
      }
      
      if (!isNaN(min)) val = Math.max(min, val);
      if (!isNaN(max)) val = Math.min(max, val);
      
      if (step % 1 !== 0) {
        const decimals = (step.toString().split('.')[1] || '').length;
        input.value = val.toFixed(decimals);
      } else {
        input.value = Math.round(val); // Always keep integers rounded
      }
      
      input.dispatchEvent(new Event('input', { bubbles: true }));
    }
  });

  // Slider interactive feedback
  const viewcubeSensSlider = modalBox.querySelector('#set-viewcube-sens');
  const viewcubeSensVal = modalBox.querySelector('#viewcube-sens-val');
  if (viewcubeSensSlider && viewcubeSensVal) {
    viewcubeSensSlider.addEventListener('input', (e) => {
      viewcubeSensVal.textContent = parseFloat(e.target.value).toFixed(4);
    });
  }

  // Export settings logic
  const exportBtn = modalBox.querySelector('#settings-export-btn');
  if (exportBtn) {
    exportBtn.addEventListener('click', () => {
      const config = {
        grid_step: parseInt(modalBox.querySelector('#set-grid-step').value),
        snap_step: parseInt(modalBox.querySelector('#set-snap-step').value),
        resize_step: parseInt(modalBox.querySelector('#set-resize-step').value),
        cable_subdivision_step: parseInt(modalBox.querySelector('#set-cable-subdivision').value),
        history_mode: modalBox.querySelector('#set-history-mode').value,
        viewcube_sensitivity: parseFloat(modalBox.querySelector('#set-viewcube-sens').value),
        default_shard_w: parseInt(modalBox.querySelector('#set-def-shard-w').value),
        default_shard_d: parseInt(modalBox.querySelector('#set-def-shard-d').value),
        default_shard_h: parseInt(modalBox.querySelector('#set-def-shard-h').value),
        default_socket_w: parseInt(modalBox.querySelector('#set-def-socket-w').value),
        default_socket_h: parseInt(modalBox.querySelector('#set-def-socket-h').value),
        default_socket_pitch: parseInt(modalBox.querySelector('#set-def-socket-pitch').value),
        
        world_width_um: parseFloat(modalBox.querySelector('#set-world-w').value),
        world_depth_um: parseFloat(modalBox.querySelector('#set-world-d').value),
        world_height_um: parseFloat(modalBox.querySelector('#set-world-h').value),
        voxel_size_um: parseFloat(modalBox.querySelector('#set-voxel-size').value),
        tick_duration_us: parseInt(modalBox.querySelector('#set-tick-duration').value),
        signal_speed_m_s: parseFloat(modalBox.querySelector('#set-signal-speed').value),
        segment_length_voxels: parseInt(modalBox.querySelector('#set-segment-len').value),
        axon_growth_max_steps: parseInt(modalBox.querySelector('#set-axon-growth').value),
        
        api_url: modalBox.querySelector('#set-api-url').value,
        auto_save: modalBox.querySelector('#set-auto-save').value === 'true',
        
        show_tooltips: modalBox.querySelector('#set-show-tooltips').checked,
        show_connections: modalBox.querySelector('#set-show-connections').checked,
        theme: modalBox.querySelector('#set-theme').value
      };
      
      const dataStr = "data:text/json;charset=utf-8," + encodeURIComponent(JSON.stringify(config, null, 2));
      const downloadAnchor = document.createElement('a');
      downloadAnchor.setAttribute("href", dataStr);
      downloadAnchor.setAttribute("download", "axicad_settings.json");
      document.body.appendChild(downloadAnchor);
      downloadAnchor.click();
      downloadAnchor.remove();
      showToast("Конфиг успешно экспортирован!", "success");
    });
  }

  // Import settings logic
  const importBtn = modalBox.querySelector('#settings-import-btn');
  const importFileInput = modalBox.querySelector('#settings-import-file');
  if (importBtn && importFileInput) {
    importBtn.addEventListener('click', () => {
      importFileInput.click();
    });
    importFileInput.addEventListener('change', (e) => {
      const file = e.target.files[0];
      if (!file) return;
      const reader = new FileReader();
      reader.onload = (event) => {
        try {
          const config = JSON.parse(event.target.result);
          
          if (config.grid_step !== undefined) modalBox.querySelector('#set-grid-step').value = config.grid_step;
          if (config.snap_step !== undefined) modalBox.querySelector('#set-snap-step').value = config.snap_step;
          if (config.resize_step !== undefined) modalBox.querySelector('#set-resize-step').value = config.resize_step;
          if (config.cable_subdivision_step !== undefined) modalBox.querySelector('#set-cable-subdivision').value = config.cable_subdivision_step;
          if (config.history_mode !== undefined) modalBox.querySelector('#set-history-mode').value = config.history_mode;
          if (config.viewcube_sensitivity !== undefined) {
            modalBox.querySelector('#set-viewcube-sens').value = config.viewcube_sensitivity;
            modalBox.querySelector('#viewcube-sens-val').textContent = config.viewcube_sensitivity.toFixed(4);
          }
          if (config.default_shard_w !== undefined) modalBox.querySelector('#set-def-shard-w').value = config.default_shard_w;
          if (config.default_shard_d !== undefined) modalBox.querySelector('#set-def-shard-d').value = config.default_shard_d;
          if (config.default_shard_h !== undefined) modalBox.querySelector('#set-def-shard-h').value = config.default_shard_h;
          if (config.default_socket_w !== undefined) modalBox.querySelector('#set-def-socket-w').value = config.default_socket_w;
          if (config.default_socket_h !== undefined) modalBox.querySelector('#set-def-socket-h').value = config.default_socket_h;
          if (config.default_socket_pitch !== undefined) modalBox.querySelector('#set-def-socket-pitch').value = config.default_socket_pitch;
          
          if (config.world_width_um !== undefined) modalBox.querySelector('#set-world-w').value = config.world_width_um;
          if (config.world_depth_um !== undefined) modalBox.querySelector('#set-world-d').value = config.world_depth_um;
          if (config.world_height_um !== undefined) modalBox.querySelector('#set-world-h').value = config.world_height_um;
          if (config.voxel_size_um !== undefined) modalBox.querySelector('#set-voxel-size').value = config.voxel_size_um;
          if (config.tick_duration_us !== undefined) modalBox.querySelector('#set-tick-duration').value = config.tick_duration_us;
          if (config.signal_speed_m_s !== undefined) modalBox.querySelector('#set-signal-speed').value = config.signal_speed_m_s;
          if (config.segment_length_voxels !== undefined) modalBox.querySelector('#set-segment-len').value = config.segment_length_voxels;
          if (config.axon_growth_max_steps !== undefined) modalBox.querySelector('#set-axon-growth').value = config.axon_growth_max_steps;
          
          if (config.api_url !== undefined) modalBox.querySelector('#set-api-url').value = config.api_url;
          if (config.auto_save !== undefined) modalBox.querySelector('#set-auto-save').value = config.auto_save ? 'true' : 'false';
          
          if (config.show_tooltips !== undefined) modalBox.querySelector('#set-show-tooltips').checked = config.show_tooltips;
          if (config.show_connections !== undefined) modalBox.querySelector('#set-show-connections').checked = config.show_connections;
          if (config.theme !== undefined) modalBox.querySelector('#set-theme').value = config.theme;
          
          updateWorldSizeDisplay();
          showToast("Конфиг импортирован! Нажмите 'Применить', чтобы сохранить.", "success");
        } catch (err) {
          showToast("Не удалось прочитать файл настроек. Некорректный формат.", "error");
        }
      };
      reader.readAsText(file);
    });
  }

  const closeModal = () => {
    window.removeEventListener('keydown', handleKeyDown);
    store.set('modalActive', false);
    modalOverlay.remove();
  };

  const handleKeyDown = (e) => {
    if (e.key === 'Escape') {
      closeModal();
    }
  };
  window.addEventListener('keydown', handleKeyDown);

  modalOverlay.addEventListener('click', (e) => {
    if (e.target === modalOverlay) {
      closeModal();
    }
  });

  modalBox.querySelector('#settings-close-x').addEventListener('click', closeModal);
  modalBox.querySelector('#settings-cancel-btn').addEventListener('click', closeModal);

  // Apply button handler
  modalBox.querySelector('#settings-apply-btn').addEventListener('click', () => {
    const grid_step = parseInt(modalBox.querySelector('#set-grid-step').value);
    const snap_step = parseInt(modalBox.querySelector('#set-snap-step').value);
    const resize_step = parseInt(modalBox.querySelector('#set-resize-step').value);
    const cable_subdivision_step = parseInt(modalBox.querySelector('#set-cable-subdivision').value);
    const history_mode = modalBox.querySelector('#set-history-mode').value;
    const viewcube_sensitivity = parseFloat(modalBox.querySelector('#set-viewcube-sens').value);

    const default_shard_w = parseInt(modalBox.querySelector('#set-def-shard-w').value);
    const default_shard_d = parseInt(modalBox.querySelector('#set-def-shard-d').value);
    const default_shard_h = parseInt(modalBox.querySelector('#set-def-shard-h').value);
    const default_socket_w = parseInt(modalBox.querySelector('#set-def-socket-w').value);
    const default_socket_h = parseInt(modalBox.querySelector('#set-def-socket-h').value);
    const default_socket_pitch = parseInt(modalBox.querySelector('#set-def-socket-pitch').value);

    const api_url = modalBox.querySelector('#set-api-url').value;
    const auto_save = modalBox.querySelector('#set-auto-save').value === 'true';
    const show_tooltips = modalBox.querySelector('#set-show-tooltips').checked;
    const show_connections = modalBox.querySelector('#set-show-connections').checked;
    const theme = modalBox.querySelector('#set-theme').value;

    const newEditorSettings = {
      grid_step,
      snap_step,
      resize_step,
      cable_subdivision_step,
      history_mode,
      viewcube_sensitivity,
      default_shard_w,
      default_shard_d,
      default_shard_h,
      default_socket_w,
      default_socket_h,
      default_socket_pitch,
      api_url,
      auto_save,
      show_tooltips,
      show_connections,
      theme
    };

    // Save editorSettings in store and localStorage
    const oldSettings = store.get('editorSettings') || {};
    store.set('editorSettings', newEditorSettings);
    localStorage.setItem('axicor_editor_settings', JSON.stringify(newEditorSettings));



    // Emit event if grid step changed
    if (oldSettings.grid_step !== grid_step) {
      emit('GRID_CONFIG_CHANGED', grid_step);
    }

    // Dynamic routes redraw if connections visibility changed
    if (oldSettings.show_connections !== show_connections) {
      import('../scene_builder.js').then(({ drawRoutes }) => {
        const routesData = store.get('routesData') || [];
        drawRoutes(routesData);
      });
    }

    // Save simulation and world parameters in placementData
    const pData = store.get('placementData');
    if (pData) {
      pData.world = pData.world || {};
      pData.world.width_um = parseFloat(modalBox.querySelector('#set-world-w').value);
      pData.world.depth_um = parseFloat(modalBox.querySelector('#set-world-d').value);
      pData.world.height_um = parseFloat(modalBox.querySelector('#set-world-h').value);

      pData.simulation = pData.simulation || {};
      pData.simulation.voxel_size_um = parseFloat(modalBox.querySelector('#set-voxel-size').value);
      pData.simulation.tick_duration_us = parseInt(modalBox.querySelector('#set-tick-duration').value);
      pData.simulation.signal_speed_m_s = parseFloat(modalBox.querySelector('#set-signal-speed').value);
      pData.simulation.segment_length_voxels = parseInt(modalBox.querySelector('#set-segment-len').value);
      pData.simulation.axon_growth_max_steps = parseInt(modalBox.querySelector('#set-axon-growth').value);

      store.set('placementData', pData);
      store.set('hasUnsavedChanges', true);
    }

    showToast('Настройки применены!', 'success');
    closeModal();
  });
}
