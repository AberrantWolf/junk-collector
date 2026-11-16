export type Appearance = 'system' | 'light' | 'dark';

const storageKey = 'collector.appearance';

export function appearancePreference(): Appearance {
 const value = document.documentElement.dataset.appearance;
 return value === 'light' || value === 'dark' ? value : 'system';
}

function applyAppearance(value: string | null) {
 if (value === 'light' || value === 'dark') {
  document.documentElement.dataset.appearance = value;
 } else {
  delete document.documentElement.dataset.appearance;
 }
}

export function setAppearance(value: Appearance) {
 applyAppearance(value);
 try {
  if (value === 'system') localStorage.removeItem(storageKey);
  else localStorage.setItem(storageKey, value);
 } catch {
  // The current tab can still switch themes if browser storage is unavailable.
 }
}

export function initializeAppearance() {
 try {
  const storage = localStorage;
  applyAppearance(storage.getItem(storageKey));
  window.addEventListener('storage', event => {
   if (event.storageArea === storage && (event.key === storageKey || event.key === null)) {
    applyAppearance(event.newValue);
   }
  });
 } catch {
  // The stylesheet follows the system setting by default.
 }
}
