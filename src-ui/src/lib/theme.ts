export type Theme = 'light' | 'dark';

const THEME_STORAGE_KEY = 'rocket-ground-station.theme.v1';

function readStoredTheme(): Theme | null {
  try {
    const value = localStorage.getItem(THEME_STORAGE_KEY);
    return value === 'light' || value === 'dark' ? value : null;
  } catch {
    return null;
  }
}

function systemTheme(): Theme {
  return typeof matchMedia === 'function' && matchMedia('(prefers-color-scheme: dark)').matches
    ? 'dark'
    : 'light';
}

export function currentTheme(): Theme {
  return document.documentElement.dataset.theme === 'dark' ? 'dark' : 'light';
}

export function setTheme(theme: Theme) {
  document.documentElement.dataset.theme = theme;
  try {
    localStorage.setItem(THEME_STORAGE_KEY, theme);
  } catch {
    // Theme still applies for this window when storage is unavailable.
  }
}

export function applyStoredTheme() {
  // `?theme=dark` lets a preview or poster capture pick the night chart directly.
  const requested = new URLSearchParams(location.search).get('theme');
  const fromUrl: Theme | null = requested === 'light' || requested === 'dark' ? requested : null;
  document.documentElement.dataset.theme = fromUrl ?? readStoredTheme() ?? systemTheme();
}
