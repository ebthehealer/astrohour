// AstroHour color themes
// Each theme maps semantic token names to CSS color values.
// Applied as CSS custom properties on :root via applyTheme().

export interface Theme {
  name:        string;
  label:       string;
  // Backgrounds
  bgBase:      string;  // main window bg
  bgSurface:   string;  // cards, panels
  bgElevated:  string;  // header, current-hour row
  bgHighlight: string;  // current hour row highlight
  // Text
  textPrimary:   string;
  textSecondary: string;
  textMuted:     string;
  textInverse:   string;
  // Accent / brand
  accent:      string;  // gold / primary accent
  accentSoft:  string;  // muted accent for borders
  // Planet / special
  planetDay:   string;  // sun icon
  planetNight: string;  // moon icon
  // State
  danger:      string;
  success:     string;
  // Interactive
  btnBg:       string;
  btnText:     string;
  linkColor:   string;
}

export const themes: Record<string, Theme> = {
  // ── Original dark indigo (current default)
  midnight: {
    name:        'midnight',
    label:       '🌌 Midnight',
    bgBase:      '#000000',
    bgSurface:   '#0a0a0a',
    bgElevated:  '#141414',
    bgHighlight: '#1e1e1e',
    textPrimary:   '#f0f0f0',
    textSecondary: '#888888',
    textMuted:     '#444444',
    textInverse:   '#000000',
    accent:      '#c8a84b',
    accentSoft:  '#3a2e10',
    planetDay:   '#c8a84b',
    planetNight: '#6677bb',
    danger:      '#b06060',
    success:     '#4a9060',
    btnBg:       '#141414',
    btnText:     '#f0f0f0',
    linkColor:   '#6655bb',
  },

  // ── Moon & Eye — periwinkle blue / slate / cream / gold (from logo)
  lunar: {
    name:        'lunar',
    label:       '🌕 Lunar',
    bgBase:      '#1c1d2e',
    bgSurface:   '#252640',
    bgElevated:  '#3a3c60',
    bgHighlight: '#4a4e80',
    textPrimary:   '#e8e6f0',
    textSecondary: '#b0aec8',
    textMuted:     '#6e6c88',
    textInverse:   '#1c1d2e',
    accent:      '#d4c47a',     // warm gold like the star in the logo
    accentSoft:  '#4a4530',
    planetDay:   '#d4c47a',
    planetNight: '#9b9fd4',     // periwinkle like the spiral
    danger:      '#c47a8a',
    success:     '#7ab4a0',
    btnBg:       '#3a3c60',
    btnText:     '#e8e6f0',
    linkColor:   '#9b9fd4',
  },

  // ── Dusk — warm amber / deep rust / dark brown
  dusk: {
    name:        'dusk',
    label:       '🌇 Dusk',
    bgBase:      '#1e1510',
    bgSurface:   '#2a1e14',
    bgElevated:  '#3d2b1a',
    bgHighlight: '#4e3520',
    textPrimary:   '#f0dfc0',
    textSecondary: '#c0a878',
    textMuted:     '#806040',
    textInverse:   '#1e1510',
    accent:      '#f0a840',
    accentSoft:  '#604020',
    planetDay:   '#f0a840',
    planetNight: '#a090c0',
    danger:      '#c06060',
    success:     '#60a878',
    btnBg:       '#3d2b1a',
    btnText:     '#f0dfc0',
    linkColor:   '#d4885a',
  },

  // ── Forest — deep green / moss / cream
  forest: {
    name:        'forest',
    label:       '🌿 Forest',
    bgBase:      '#0d1f14',
    bgSurface:   '#152a1c',
    bgElevated:  '#1e3d28',
    bgHighlight: '#254d32',
    textPrimary:   '#d8f0d0',
    textSecondary: '#90b898',
    textMuted:     '#507860',
    textInverse:   '#0d1f14',
    accent:      '#c8d860',
    accentSoft:  '#304818',
    planetDay:   '#e0c840',
    planetNight: '#7090b8',
    danger:      '#c07060',
    success:     '#60c080',
    btnBg:       '#1e3d28',
    btnText:     '#d8f0d0',
    linkColor:   '#8ab870',
  },

  // ── Light parchment — warm light mode
  parchment: {
    name:        'parchment',
    label:       '📜 Parchment',
    bgBase:      '#f4efe4',
    bgSurface:   '#ece4d4',
    bgElevated:  '#d4c8b0',
    bgHighlight: '#c8b890',
    textPrimary:   '#2a2018',
    textSecondary: '#5a4a38',
    textMuted:     '#8a7a68',
    textInverse:   '#f4efe4',
    accent:      '#8060a0',
    accentSoft:  '#c0b0d0',
    planetDay:   '#b08000',
    planetNight: '#5060a0',
    danger:      '#a04040',
    success:     '#406040',
    btnBg:       '#8060a0',
    btnText:     '#f4efe4',
    linkColor:   '#7050a0',
  },
};

export const DEFAULT_THEME = 'midnight';

export function applyTheme(t: Theme) {
  const r = document.documentElement.style;
  r.setProperty('--bg-base',       t.bgBase);
  r.setProperty('--bg-surface',    t.bgSurface);
  r.setProperty('--bg-elevated',   t.bgElevated);
  r.setProperty('--bg-highlight',  t.bgHighlight);
  r.setProperty('--text-primary',  t.textPrimary);
  r.setProperty('--text-secondary',t.textSecondary);
  r.setProperty('--text-muted',    t.textMuted);
  r.setProperty('--text-inverse',  t.textInverse);
  r.setProperty('--accent',        t.accent);
  r.setProperty('--accent-soft',   t.accentSoft);
  r.setProperty('--planet-day',    t.planetDay);
  r.setProperty('--planet-night',  t.planetNight);
  r.setProperty('--danger',        t.danger);
  r.setProperty('--success',       t.success);
  r.setProperty('--btn-bg',        t.btnBg);
  r.setProperty('--btn-text',      t.btnText);
  r.setProperty('--link-color',    t.linkColor);
}
