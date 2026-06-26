/**
 * @fileoverview api.js — Centralized API service for handling all HTTP requests
 * to the backend REST endpoints and static file assets.
 */

async function handleResponse(response) {
  if (!response.ok) {
    let errMsg = `HTTP Error: ${response.status}`;
    try {
      const errData = await response.json();
      if (errData && errData.message) errMsg = errData.message;
    } catch (_) {}
    throw new Error(errMsg);
  }
  return response.json();
}

export const api = {
  /**
   * Load static placement data for a project.
   * @param {string} projectName
   * @returns {Promise<Object>}
   */
  async loadPlacement(projectName) {
    const response = await fetch(`./projects/local/${projectName}/placement.json`);
    return handleResponse(response);
  },

  /**
   * Load static routes data for a project.
   * @param {string} projectName
   * @returns {Promise<Array>}
   */
  async loadRoutes(projectName) {
    const response = await fetch(`./projects/local/${projectName}/routes.json`);
    return handleResponse(response);
  },

  /**
   * Load static history cache for a project.
   * @param {string} projectName
   * @returns {Promise<Object|null>}
   */
  async loadHistoryCache(projectName) {
    const response = await fetch(`./projects/local/${projectName}/history_cache.json`);
    if (response.status === 404) return null;
    return handleResponse(response);
  },

  /**
   * Fetch list of all scripts, models, and local projects.
   * @returns {Promise<Object>}
   */
  async listProjects() {
    const response = await fetch('/api/projects');
    return handleResponse(response);
  },

  /**
   * Create a new project.
   * @param {string} name
   * @returns {Promise<Object>}
   */
  async createProject(name) {
    const response = await fetch('/api/projects/create', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ name })
    });
    return handleResponse(response);
  },

  /**
   * Import a script project content.
   * @param {string} filename
   * @param {string} content
   * @returns {Promise<Object>}
   */
  async importProject(filename, content) {
    const response = await fetch('/api/projects/import', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ filename, content })
    });
    return handleResponse(response);
  },

  /**
   * Rename a local project.
   * @param {string} oldName
   * @param {string} newName
   * @returns {Promise<Object>}
   */
  async renameProject(oldName, newName) {
    const response = await fetch('/api/projects/rename', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ oldName, newName })
    });
    return handleResponse(response);
  },

  /**
   * Delete a local project.
   * @param {string} name
   * @returns {Promise<Object>}
   */
  async deleteProject(name) {
    const response = await fetch('/api/projects/delete', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ name })
    });
    return handleResponse(response);
  },

  /**
   * Load project recipe or workspace coordinates.
   * @param {string} type - 'script' or 'local'
   * @param {string} name
   * @returns {Promise<Object>}
   */
  async loadProject(type, name) {
    const response = await fetch('/api/projects/load', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ type, name })
    });
    return handleResponse(response);
  },

  /**
   * Save layout and connections overrides payload to backend.
   * @param {Object} payload
   * @returns {Promise<Object>}
   */
  async saveLayout(payload) {
    const response = await fetch('/api/save', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload)
    });
    return handleResponse(response);
  }
};
