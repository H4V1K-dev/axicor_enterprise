/**
 * @fileoverview toast.js — Handles toast notifications.
 */

let toastElement = null;

function getToastElement() {
  if (!toastElement) {
    toastElement = document.getElementById('toast');
    if (!toastElement) {
      toastElement = document.createElement('div');
      toastElement.id = 'toast';
      toastElement.className = 'ax-toast ax-ui-overlay';
      document.body.appendChild(toastElement);
    } else {
      toastElement.className = 'ax-toast ax-ui-overlay';
    }
  }
  return toastElement;
}

/**
 * Displays a toast notification.
 * @param {string} message 
 * @param {'success'|'error'|'info'} type 
 * @param {number|null} [duration=3000] - Duration in ms, or null to keep open indefinitely
 */
export function showToast(message, type = 'success', duration = 3000) {
  const el = getToastElement();
  el.textContent = message;
  
  // Reset states
  el.className = 'ax-toast ax-ui-overlay';
  
  if (type === 'success') {
    el.classList.add('ax-toast--success');
  } else if (type === 'error') {
    el.classList.add('ax-toast--error');
  } else {
    el.classList.add('ax-toast--info');
  }
  
  el.style.display = 'block';
  setTimeout(() => { el.style.opacity = '1'; }, 50);

  if (el.timeoutId) clearTimeout(el.timeoutId);
  if (el.fadeTimeoutId) clearTimeout(el.fadeTimeoutId);
  el.timeoutId = null;
  el.fadeTimeoutId = null;

  if (duration !== null && duration > 0) {
    el.timeoutId = setTimeout(() => {
      el.style.opacity = '0';
      el.fadeTimeoutId = setTimeout(() => { el.style.display = 'none'; }, 300);
    }, duration);
  }
}

/**
 * Hides the active toast notification immediately.
 */
export function hideToast() {
  if (!toastElement) return;
  if (toastElement.timeoutId) clearTimeout(toastElement.timeoutId);
  if (toastElement.fadeTimeoutId) clearTimeout(toastElement.fadeTimeoutId);
  toastElement.style.opacity = '0';
  toastElement.fadeTimeoutId = setTimeout(() => { toastElement.style.display = 'none'; }, 300);
}
