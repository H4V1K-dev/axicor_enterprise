import { store } from '../store/store.js';
import { api } from '../services/api.js';

// Inline SVGs for beautiful Lucide-style visual layout icons
const SVG_SUN = `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-sun"><circle cx="12" cy="12" r="4"/><path d="M12 2v2"/><path d="M12 20v2"/><path d="M4.93 4.93l1.41 1.41"/><path d="M17.66 17.66l1.41 1.41"/><path d="M2 12h2"/><path d="M20 12h2"/><path d="M6.34 17.66l-1.41 1.41"/><path d="M19.07 4.93l-1.41 1.41"/></svg>`;
const SVG_MOON = `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-moon"><path d="M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9Z"/></svg>`;
const SVG_TRASH2 = `<svg xmlns="http://www.w3.org/2000/svg" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-trash-2"><path d="M3 6h18"/><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"/><path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/><line x1="10" x2="10" y1="11" y2="17"/><line x1="14" x2="14" y1="11" y2="17"/></svg>`;
const SVG_EDIT2 = `<svg xmlns="http://www.w3.org/2000/svg" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-edit-2"><path d="M12 20h9"/><path d="M16.5 3.5a2.12 2.12 0 0 1 3 3L7 19l-4 1 1-4Z"/></svg>`;
const SVG_ACTIVITY = `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-activity"><path d="M22 12h-4l-3 9L9 3l-3 9H2"/></svg>`;
const SVG_SETTINGS = `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-settings"><path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.1a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"/><circle cx="12" cy="12" r="3"/></svg>`;
const SVG_FILECODE2 = `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-file-code-2"><path d="M4 22h14a2 2 0 0 0 2-2V7.5L14.5 2H6a2 2 0 0 0-2 2v4"/><path d="M14 2v6h6"/><path d="m5 12-3 3 3 3"/><path d="m9 18 3-3-3-3"/></svg>`;
const SVG_FOLDERDOT = `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-folder-dot"><path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.93a2 2 0 0 1-1.66-.9l-.82-1.2A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z"/><circle cx="12" cy="13" r="1"/></svg>`;
const SVG_FOLDEROPEN = `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-folder-open"><path d="m6 14 1.45-2.9A2 2 0 0 1 9.24 10H20a2 2 0 0 1 1.94 2.5l-1.55 6a2 2 0 0 1-1.94 1.5H4a2 2 0 0 1-2-2V5c0-1.1.9-2 2-2h3.93a2 2 0 0 1 1.66.9l.82 1.2a2 2 0 0 0 1.66.9H18a2 2 0 0 1 2 2v2"/></svg>`;
const SVG_DOWNLOAD = `<svg xmlns="http://www.w3.org/2000/svg" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-download"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" x2="12" y1="15" y2="3"/></svg>`;
const SVG_PLUS = `<svg xmlns="http://www.w3.org/2000/svg" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-plus"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>`;


export async function showProjectSelector() {
  return new Promise((resolve) => {
    const modal = document.getElementById('project-selector-modal');
    modal.style.display = 'flex';

    const container = modal.querySelector('.project-selector-container');
    const themeBtn = document.getElementById('theme-switch-btn');
    const importBtn = document.getElementById('import-project-btn');
    const createBtn = document.getElementById('create-project-btn');
    const fileInput = document.getElementById('import-file-input');

    // Populate buttons dynamically with SVGs
    importBtn.innerHTML = `${SVG_FOLDEROPEN}<span>Импортировать</span>`;
    importBtn.onclick = () => fileInput.click();

    createBtn.innerHTML = `${SVG_PLUS}<span>Создать проект</span>`;
    createBtn.onclick = async () => {
      try {
        const data = await api.listProjects();
        const existingLocalNames = (data.local || []).map(p => p.name);

        let index = 0;
        let newProjName = '';
        while (true) {
          const suffix = String(index).padStart(3, '0');
          newProjName = `New Model ${suffix}`;
          if (!existingLocalNames.includes(newProjName)) {
            break;
          }
          index++;
        }

        const res = await api.createProject(newProjName);
        if (res.status === 'success') {
          window.newlyCreatedProject = newProjName;
          alert(`Проект "${newProjName}" успешно создан! ✓`);
          await renderLists();
        } else {
          alert('Ошибка при создании проекта: ' + res.message);
        }
      } catch (err) {
        alert('Ошибка связи с сервером при создании проекта: ' + err.message);
      }
    };

    // Theme Switcher Logic with persistence
    let currentTheme = localStorage.getItem('axicor-hub-theme') || 'dark';

    function updateThemeUI() {
      container.dataset.theme = currentTheme;
      if (currentTheme === 'dark') {
        themeBtn.innerHTML = `${SVG_MOON}<span>Dark</span>`;
        themeBtn.title = "Текущая тема: Темная. Нажмите для переключения на Светлую.";
      } else {
        themeBtn.innerHTML = `${SVG_SUN}<span>Light</span>`;
        themeBtn.title = "Текущая тема: Светлая. Нажмите для переключения на Темную.";
      }
    }

    updateThemeUI();

    themeBtn.onclick = (e) => {
      e.stopPropagation();
      currentTheme = currentTheme === 'dark' ? 'light' : 'dark';
      localStorage.setItem('axicor-hub-theme', currentTheme);
      updateThemeUI();
    };

    let activeCanvas = null;
    let canvasAnimationId = null;

    function initNeuralCanvas(canvas) {
      const ctx = canvas.getContext('2d');
      if (!ctx) return;

      let nodes = [];

      function resize() {
        const rect = canvas.getBoundingClientRect();
        const dpr = window.devicePixelRatio || 1;
        canvas.width = rect.width * dpr;
        canvas.height = rect.height * dpr;
        ctx.scale(dpr, dpr);
        initNodes(rect.width, rect.height);
      }

      function initNodes(width, height) {
        const numNodes = 28;
        nodes = [];
        for (let i = 0; i < numNodes; i++) {
          nodes.push({
            x: Math.random() * (width - 60) + 30,
            y: Math.random() * (height - 40) + 20,
            radius: Math.random() * 2.5 + 1.2,
            color: i % 4 === 0 ? 'rgba(239, 68, 68, 0.95)' : 'rgba(239, 68, 68, 0.55)',
            vx: (Math.random() - 0.5) * 0.3,
            vy: (Math.random() - 0.5) * 0.3,
            pulseSpeed: Math.random() * 0.03 + 0.01,
            pulsePhase: Math.random() * Math.PI
          });
        }
      }

      resize();

      const resizeObserver = new ResizeObserver(() => resize());
      resizeObserver.observe(canvas.parentElement || canvas);

      function draw() {
        const width = canvas.width / (window.devicePixelRatio || 1);
        const height = canvas.height / (window.devicePixelRatio || 1);
        ctx.clearRect(0, 0, width, height);

        nodes.forEach(node => {
          node.x += node.vx;
          node.y += node.vy;
          node.pulsePhase += node.pulseSpeed;

          if (node.x < 10 || node.x > width - 10) node.vx *= -1;
          if (node.y < 10 || node.y > height - 10) node.vy *= -1;
        });

        ctx.lineWidth = 0.5;
        for (let i = 0; i < nodes.length; i++) {
          for (let j = i + 1; j < nodes.length; j++) {
            const dx = nodes[i].x - nodes[j].x;
            const dy = nodes[i].y - nodes[j].y;
            const dist = Math.sqrt(dx * dx + dy * dy);

            if (dist < 85) {
              const alpha = (1 - dist / 85) * 0.45;
              ctx.strokeStyle = `rgba(239, 68, 68, ${alpha})`;
              ctx.beginPath();
              ctx.moveTo(nodes[i].x, nodes[i].y);
              ctx.lineTo(nodes[j].x, nodes[j].y);
              ctx.stroke();
            }
          }
        }

        nodes.forEach((node, index) => {
          const currentRadius = node.radius + Math.sin(node.pulsePhase) * 0.7;
          ctx.fillStyle = node.color;
          ctx.beginPath();
          ctx.arc(node.x, node.y, currentRadius, 0, Math.PI * 2);
          ctx.fill();

          if (index % 4 === 0) {
            ctx.fillStyle = 'rgba(239, 68, 68, 0.15)';
            ctx.beginPath();
            ctx.arc(node.x, node.y, currentRadius * 3, 0, Math.PI * 2);
            ctx.fill();
          }
        });

        canvasAnimationId = requestAnimationFrame(draw);
      }

      draw();

      canvas.cleanup = () => {
        if (canvasAnimationId) cancelAnimationFrame(canvasAnimationId);
        resizeObserver.disconnect();
      };

      activeCanvas = canvas;
    }

    function closeModal(result) {
      if (activeCanvas && activeCanvas.cleanup) {
        activeCanvas.cleanup();
      }
      modal.style.display = 'none';
      resolve(result);
    }

    fileInput.onchange = async (e) => {
      const file = e.target.files[0];
      if (!file) return;

      const reader = new FileReader();
      reader.onload = async (event) => {
        try {
          const content = event.target.result;
          const res = await api.importProject(file.name, content);
          if (res.status === 'success') {
            alert('Скрипт импортирован успешно! ✓');
            await renderLists();
          } else {
            alert('Ошибка импорта: ' + res.message);
          }
        } catch (err) {
          alert('Ошибка чтения/загрузки файла: ' + err.message);
        }
      };
      reader.readAsText(file);
    };

    // Shared rename flow
    async function renameProjectFlow(oldName) {
      const newName = prompt('Введите новое имя проекта:', oldName);
      if (!newName || newName === oldName) return;

      const nameRegex = /^[a-zA-Z0-9_\-]+$/;
      if (!nameRegex.test(newName)) {
        alert('Имя проекта может содержать только латинские буквы, цифры, дефис и подчеркивание.');
        return;
      }

      try {
        const res = await api.renameProject(oldName, newName);
        if (res.status === 'success') {
          alert('Проект переименован! ✓');
          await renderLists();
        } else {
          alert('Ошибка переименования: ' + res.message);
        }
      } catch (err) {
        alert('Ошибка связи с сервером: ' + err.message);
      }
    }

    // Shared delete flow
    async function deleteProjectFlow(name) {
      if (!confirm(`Вы действительно хотите удалить проект "${name}"?
Это действие нельзя отменить.`)) {
        return;
      }

      try {
        const res = await api.deleteProject(name);
        if (res.status === 'success') {
          alert('Проект успешно удален! ✓');
          await renderLists();
        } else {
          alert('Ошибка удаления: ' + res.message);
        }
      } catch (err) {
        alert('Ошибка связи с сервером: ' + err.message);
      }
    }

    async function renderLists() {
      const data = await api.listProjects();

      const scriptsList = document.getElementById('scripts-list');
      const modelsList = document.getElementById('models-list');
      const localList = document.getElementById('local-list');
      const featuredContainer = document.getElementById('featured-project-container');

      // Update Counts
      const scriptsCount = document.getElementById('scripts-count');
      if (scriptsCount) {
        scriptsCount.textContent = data.scripts ? data.scripts.length : 0;
      }

      const modelsCount = document.getElementById('models-count');
      if (modelsCount) {
        modelsCount.textContent = data.models ? data.models.length : 0;
      }

      const currentProject = store.get('projectName');

      // 1. Python scripts list
      scriptsList.innerHTML = '';
      if (!data.scripts || data.scripts.length === 0) {
        scriptsList.innerHTML = `
          <div class="ax-list-item tree-empty-node" title="Скрипты в projects/scripts/ не найдены" style="pointer-events: none; color: var(--ax-text-faint); padding: 3px 6px; font-size: 11.5px; display: flex; align-items: center; gap: 4px;">
            <span class="tree-node-prefix">└─</span>
            <span style="font-style: italic;">Пусто</span>
          </div>
        `;
      } else {
        data.scripts.forEach((script, idx) => {
          const item = document.createElement('div');

          const scriptProjName = script.replace(/\.(py|toml)$/, '');
          const isScriptActive = scriptProjName === currentProject;

          item.className = `ax-list-item ${isScriptActive ? 'active' : ''}`;
          const prefix = (idx === data.scripts.length - 1) ? '└─' : '├─';

          item.innerHTML = `
            <span class="tree-node-prefix">${prefix}</span>
            <span style="display: flex; align-items: center; gap: 4px;">
              ${SVG_FILECODE2}
              <b>${script}</b>
            </span>
          `;

          item.onclick = async () => {
            closeModal(scriptProjName);
            document.getElementById('loading').style.display = 'block';
            document.getElementById('loading').textContent = 'Компиляция проекта...';
            try {
              const loadRes = await api.loadProject('script', script);
              if (loadRes.status === 'success') {
                closeModal(loadRes.project);
              } else {
                alert('Ошибка компиляции проекта: ' + loadRes.message);
                modal.style.display = 'flex';
                document.getElementById('loading').style.display = 'none';
              }
            } catch (err) {
              alert('Ошибка связи с сервером: ' + err.message);
              modal.style.display = 'flex';
              document.getElementById('loading').style.display = 'none';
            }
          };
          scriptsList.appendChild(item);
        });
      }

      // 2. TOML models list
      modelsList.innerHTML = '';
      if (!data.models || data.models.length === 0) {
        modelsList.innerHTML = `
          <div class="ax-list-item tree-empty-node" title="Модели в projects/models/ не найдены" style="pointer-events: none; color: var(--ax-text-faint); padding: 3px 6px; font-size: 11.5px; display: flex; align-items: center; gap: 4px;">
            <span class="tree-node-prefix">└─</span>
            <span style="font-style: italic;">Пусто</span>
          </div>
        `;
      } else {
        data.models.forEach((modelName, idx) => {
          const item = document.createElement('div');
          item.className = 'ax-list-item';
          const prefix = (idx === data.models.length - 1) ? '└─' : '├─';
          item.innerHTML = `
            <span class="tree-node-prefix">${prefix}</span>
            <span style="display: flex; align-items: center; gap: 4px;">
              ${SVG_SETTINGS}
              <b>${modelName}</b>
            </span>
          `;
          item.onclick = () => {
            alert('Запуск TOML моделей напрямую пока в разработке. Выберите Python скрипт.');
          };
          modelsList.appendChild(item);
        });
      }

      // 3. Local projects list & Featured Card
      localList.innerHTML = '';
      featuredContainer.innerHTML = '';

      if (!data.local || data.local.length === 0) {
        localList.innerHTML = `<div class="project-empty">Локальные проекты не найдены</div>`;
        featuredContainer.innerHTML = `<div class="project-empty">Нет недавних проектов</div>`;
      } else {
        // First project is the featured "Last Work" (sorted by mtime descending from backend)
        const lastWork = data.local[0];

        // Clean up previous canvas if any
        if (activeCanvas && activeCanvas.cleanup) {
          activeCanvas.cleanup();
        }

        // Render Featured Card
        const featuredCard = document.createElement('div');
        featuredCard.className = 'featured-project-card ax-card ax-card--featured';

        // Canvas Graph background wrapper
        const canvasWrapper = document.createElement('div');
        canvasWrapper.className = 'preview-canvas-wrapper';
        const canvas = document.createElement('canvas');
        canvasWrapper.appendChild(canvas);
        featuredCard.appendChild(canvasWrapper);

        // Add blur overlay
        const blurOverlay = document.createElement('div');
        blurOverlay.className = 'card-blur-overlay';
        featuredCard.appendChild(blurOverlay);

        // Content layout
        const featuredInfo = document.createElement('div');
        featuredInfo.className = 'featured-card-info';

        const isNewProject = window.newlyCreatedProject === lastWork.name;
        const sourceText = lastWork.name.startsWith('New Model ') ? 'AxiCAD' : `${lastWork.name}.py`;

        featuredInfo.innerHTML = `
          <div class="featured-card-top">
            <div class="card-tag" style="font-family: var(--ax-font-mono); font-size: 10px; font-weight: 700; letter-spacing: 1.5px; color: var(--ax-text-faint); margin-bottom: 4px; text-transform: uppercase;">LAST WORK</div>
            <h4 class="card-project-name" style="margin-left: 12px; display: flex; align-items: center;">
              <span class="slash-decor ${isNewProject ? 'new-project-active' : ''}"><span class="slash-green">/</span><span class="slash-red">/</span></span>
              <span>${lastWork.name}</span>
            </h4>
          </div>
          <div class="featured-card-bottom">
            <div class="card-meta" style="margin-bottom: 10px;">
              <span>
                ${SVG_FOLDERDOT}
                ${lastWork.formatted_time}
              </span>
              <span>
                ${SVG_FILECODE2}
                Источник: ${sourceText}
              </span>
            </div>
            <div class="card-actions">
              <button class="ax-btn ax-btn--primary ${isNewProject ? 'new-project-active' : ''}" id="btn-load-featured">${SVG_ACTIVITY}Продолжить работу</button>
              <button class="ax-btn ax-btn--secondary btn-rename-featured">${SVG_EDIT2}Переименовать</button>
              <button class="ax-btn ax-btn--danger btn-delete-featured">${SVG_TRASH2}Удалить</button>
            </div>
          </div>
        `;

        featuredInfo.querySelector('#btn-load-featured').onclick = () => {
          closeModal(lastWork.name);
        };

        featuredInfo.querySelector('.btn-rename-featured').onclick = async (e) => {
          e.stopPropagation();
          await renameProjectFlow(lastWork.name);
        };

        featuredInfo.querySelector('.btn-delete-featured').onclick = async (e) => {
          e.stopPropagation();
          await deleteProjectFlow(lastWork.name);
        };

        featuredCard.appendChild(featuredInfo);
        featuredContainer.appendChild(featuredCard);

        // Initialize animation on canvas
        initNeuralCanvas(canvas);

        // Render grid of remaining local projects
        const gridProjects = data.local.slice(1);
        if (gridProjects.length === 0) {
          localList.innerHTML = `<div class="project-empty border border-dashed border-white/5 rounded-xl py-8">Дополнительные проекты не найдены.</div>`;
        } else {
          gridProjects.forEach(localProj => {
            const item = document.createElement('div');
              item.className = 'local-grid-card ax-card';

              // Preview background or fallback grid matrix
              if (localProj.has_preview) {
                const previewWrapper = document.createElement('div');
                previewWrapper.className = 'grid-card-preview-wrapper';
                previewWrapper.innerHTML = `<img src="./projects/local/${localProj.name}/preview.png" alt="Preview">`;
                item.appendChild(previewWrapper);

                const blurOverlay = document.createElement('div');
                blurOverlay.className = 'grid-card-blur-overlay';
                item.appendChild(blurOverlay);

                const innerShadow = document.createElement('div');
                innerShadow.className = 'grid-card-inner-shadow';
                item.appendChild(innerShadow);
              } else {
                const gridBg = document.createElement('div');
                gridBg.className = 'grid-card-bg placeholder-bg';
                item.appendChild(gridBg);
              }

              // Content layer
              const content = document.createElement('div');
              content.className = 'grid-card-content';
              content.innerHTML = `
          <div class="grid-card-main-click" style="display: flex; flex-direction: column; justify-content: space-between; height: 100%;">
            <h4 title="${localProj.name}">${localProj.name}</h4>
            <div class="grid-project-bottom-meta" style="display: flex; flex-direction: column; gap: 2px;">
              <div class="grid-project-meta flex items-center gap-1">
                <span style="width: 6px; height: 6px; border-radius: 50%; background: #10b981; display: inline-block;"></span>
                ${localProj.formatted_time}
              </div>
              <div class="grid-project-path-wrapper" title="projects/local/${localProj.name}" style="font-size: 9px; color: var(--ax-text-faint); font-family: var(--ax-font-mono); display: flex; align-items: center; gap: 4px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 140px;">
                <span>Местоположение:</span>
                <a href="./projects/local/${localProj.name}/" target="_blank" class="project-path-link" style="color: var(--ax-text-muted); text-decoration: underline; pointer-events: auto;">projects/local/${localProj.name}</a>
              </div>
            </div>
          </div>
          <div class="grid-card-actions">
            <button class="ax-btn ax-btn--secondary ax-btn--icon btn-rename-grid" title="Переименовать">${SVG_EDIT2}</button>
            <button class="ax-btn ax-btn--secondary ax-btn--icon btn-export-grid" title="Экспорт">${SVG_DOWNLOAD}</button>
            <button class="ax-btn ax-btn--danger ax-btn--icon btn-delete-grid" title="Удалить">${SVG_TRASH2}</button>
          </div>
        `;

              content.querySelector('.grid-card-main-click').onclick = () => {
                closeModal(localProj.name);
              };

              content.querySelector('.project-path-link').onclick = (e) => {
                e.stopPropagation();
              };

              content.querySelector('.btn-rename-grid').onclick = async (e) => {
                e.stopPropagation();
                await renameProjectFlow(localProj.name);
              };

              content.querySelector('.btn-export-grid').onclick = (e) => {
                e.stopPropagation();
                alert(`Экспорт проекта "${localProj.name}" пока не реализован.`);
              };

              content.querySelector('.btn-delete-grid').onclick = async (e) => {
                e.stopPropagation();
                await deleteProjectFlow(localProj.name);
              };

              item.appendChild(content);
              localList.appendChild(item);
            });
          }
      }
      }

      renderLists().catch(err => {
        console.error('Failed to load projects list:', err);
        closeModal('octopus');
      });
    });
}
