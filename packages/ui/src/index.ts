import type { BackgroundGradient, CustomFont, DesignTokens } from '@og-suite/contracts'

export const appearanceStorageKey = 'og-suite:appearance'
export const customFontStorageKey = 'og-suite:custom-fonts'
export const customFontsChangedEvent = 'og-suite:custom-fonts-changed'

export const defaultTokens: DesignTokens = {
  colorBackground: '#09111b',
  colorBackgroundGradient:
    'radial-gradient(circle at top left, rgba(24, 45, 73, 0.28), transparent 42%), radial-gradient(circle at top right, rgba(10, 75, 56, 0.22), transparent 38%)',
  backgroundGradients: [
    {
      id: 'default-left',
      name: 'Upper left',
      strength: 0.28,
      points: [{ id: 'default-left-point', color: '#182d49', strength: 0.28, x: 0, y: 0, stop: 42 }],
    },
    {
      id: 'default-right',
      name: 'Upper right',
      strength: 0.22,
      points: [{ id: 'default-right-point', color: '#0a4b38', strength: 0.22, x: 100, y: 0, stop: 38 }],
    },
  ],
  backgroundImage: '',
  backgroundImageOpacity: 0.35,
  panelOpacity: 0.84,
  colorSurface: 'rgba(22, 32, 45, 0.84)',
  colorSurfaceSubtle: 'rgba(255, 255, 255, 0.04)',
  colorSurfaceStrong: 'rgba(8, 13, 22, 0.86)',
  colorToolBackground: '#0c1724',
  colorActionBarBackground: '#111d2b',
  colorText: '#edf5fb',
  colorMuted: '#9aabbe',
  colorAccent: '#41b883',
  colorAccentSoft: 'rgba(65, 184, 131, 0.16)',
  colorAccentBorder: 'rgba(65, 184, 131, 0.7)',
  colorAccentContrast: '#071019',
  colorBorder: 'rgba(255, 255, 255, 0.1)',
  colorNav: 'rgba(7, 13, 22, 0.94)',
  shadow: '0 24px 60px rgba(0, 0, 0, 0.28)',
  margin: 16,
  radius: 20,
  density: 'compact',
  fontFamily: '"IBM Plex Sans", "Segoe UI", system-ui, sans-serif',
  confirmDelete: true,
}

export const lightTokens: DesignTokens = {
  colorBackground: '#f3f7fb',
  colorBackgroundGradient:
    'radial-gradient(circle at top left, rgba(105, 150, 210, 0.2), transparent 42%), radial-gradient(circle at top right, rgba(65, 184, 131, 0.18), transparent 38%)',
  backgroundGradients: [
    {
      id: 'light-left',
      name: 'Soft blue',
      strength: 0.2,
      points: [{ id: 'light-left-point', color: '#6996d2', strength: 0.2, x: 0, y: 0, stop: 42 }],
    },
    {
      id: 'light-right',
      name: 'Soft green',
      strength: 0.18,
      points: [{ id: 'light-right-point', color: '#41b883', strength: 0.18, x: 100, y: 0, stop: 38 }],
    },
  ],
  backgroundImage: '',
  backgroundImageOpacity: 0.35,
  panelOpacity: 0.88,
  colorSurface: 'rgba(255, 255, 255, 0.88)',
  colorSurfaceSubtle: 'rgba(255, 255, 255, 0.58)',
  colorSurfaceStrong: 'rgba(235, 242, 249, 0.92)',
  colorToolBackground: '#ffffff',
  colorActionBarBackground: '#eef5fb',
  colorText: '#162232',
  colorMuted: '#637282',
  colorAccent: '#197a5a',
  colorAccentSoft: 'rgba(25, 122, 90, 0.14)',
  colorAccentBorder: 'rgba(25, 122, 90, 0.58)',
  colorAccentContrast: '#ffffff',
  colorBorder: 'rgba(22, 34, 50, 0.14)',
  colorNav: 'rgba(246, 250, 253, 0.94)',
  shadow: '0 24px 60px rgba(35, 48, 66, 0.16)',
  margin: 16,
  radius: 20,
  density: 'compact',
  fontFamily: '"IBM Plex Sans", "Segoe UI", system-ui, sans-serif',
  confirmDelete: true,
}

export const builtInFontOptions = [
  { label: 'Plex Sans', value: '"IBM Plex Sans", "Segoe UI", system-ui, sans-serif' },
  { label: 'System UI', value: 'system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif' },
  { label: 'Avenir', value: '"Avenir Next", "Helvetica Neue", sans-serif' },
  { label: 'Serif', value: 'Georgia, "Times New Roman", serif' },
  { label: 'Mono', value: '"IBM Plex Mono", "SFMono-Regular", monospace' },
]

export function loadStoredFonts(storageKey = customFontStorageKey): CustomFont[] {
  if (typeof localStorage === 'undefined') return []
  const raw = localStorage.getItem(storageKey)
  if (!raw) return []
  try {
    const parsed = JSON.parse(raw) as unknown
    if (!Array.isArray(parsed)) return []
    return parsed.map(normalizeCustomFont).filter((font): font is CustomFont => Boolean(font))
  } catch {
    return []
  }
}

export function saveStoredFonts(fonts: CustomFont[], storageKey = customFontStorageKey) {
  if (typeof localStorage === 'undefined') return
  localStorage.setItem(storageKey, JSON.stringify(fonts.map(normalizeCustomFont).filter(Boolean)))
  applyStoredFonts(fonts)
  window.dispatchEvent(new CustomEvent(customFontsChangedEvent))
}

export function fontFamilyForCustomFont(font: Pick<CustomFont, 'family'>) {
  return `"${font.family.replaceAll('"', '\\"')}", var(--og-font), system-ui, sans-serif`
}

export function applyStoredFonts(fonts = loadStoredFonts()) {
  if (typeof document === 'undefined') return
  const styleId = 'og-suite-custom-fonts'
  let style = document.getElementById(styleId) as HTMLStyleElement | null
  if (!style) {
    style = document.createElement('style')
    style.id = styleId
    document.head.appendChild(style)
  }
  style.textContent = fonts
    .map((font) => `@font-face{font-family:"${escapeCssString(font.family)}";src:url("${font.dataUrl}") format("${escapeCssString(font.format)}");font-display:swap;}`)
    .join('\n')
}

export function normalizeFontFamilyName(name: string) {
  return name.trim().replace(/\s+/g, ' ').replace(/["\\]/g, '')
}

function normalizeCustomFont(value: unknown): CustomFont | null {
  if (!value || typeof value !== 'object') return null
  const item = value as Partial<CustomFont>
  const name = typeof item.name === 'string' ? normalizeFontFamilyName(item.name) : ''
  const family = typeof item.family === 'string' ? normalizeFontFamilyName(item.family) : name
  if (!name || !family || typeof item.dataUrl !== 'string' || !item.dataUrl.startsWith('data:')) return null
  return {
    id: typeof item.id === 'string' ? item.id : createUiId('font'),
    name,
    family,
    dataUrl: item.dataUrl,
    format: typeof item.format === 'string' && item.format.trim() ? item.format.trim() : 'woff2',
    createdAt: typeof item.createdAt === 'string' ? item.createdAt : new Date().toISOString(),
  }
}

function escapeCssString(value: string) {
  return value.replace(/\\/g, '\\\\').replace(/"/g, '\\"')
}

export function loadStoredTokens(storageKey = appearanceStorageKey): DesignTokens {
  if (typeof localStorage === 'undefined') return defaultTokens
  const raw = localStorage.getItem(storageKey)
  if (!raw) return defaultTokens
  try {
    return normalizeTokens(JSON.parse(raw) as Partial<DesignTokens>)
  } catch {
    return defaultTokens
  }
}

export function saveStoredTokens(tokens: DesignTokens, storageKey = appearanceStorageKey) {
  if (typeof localStorage === 'undefined') return
  localStorage.setItem(storageKey, JSON.stringify(tokens))
}

export function normalizeTokens(tokens: Partial<DesignTokens>): DesignTokens {
  const backgroundGradients = Array.isArray(tokens.backgroundGradients)
    ? tokens.backgroundGradients.map(normalizeGradient).filter((gradient) => gradient.points.length > 0)
    : defaultTokens.backgroundGradients
  const normalized: DesignTokens = {
    ...defaultTokens,
    ...tokens,
    backgroundGradients,
    backgroundImage: typeof tokens.backgroundImage === 'string' ? tokens.backgroundImage : defaultTokens.backgroundImage,
    backgroundImageOpacity: clampNumber(tokens.backgroundImageOpacity, 0, 1, defaultTokens.backgroundImageOpacity),
    colorToolBackground: typeof tokens.colorToolBackground === 'string' ? tokens.colorToolBackground : defaultTokens.colorToolBackground,
    colorActionBarBackground: typeof tokens.colorActionBarBackground === 'string' ? tokens.colorActionBarBackground : defaultTokens.colorActionBarBackground,
    panelOpacity: clampNumber(tokens.panelOpacity, 0.08, 1, defaultTokens.panelOpacity),
    margin: clampNumber(tokens.margin, 4, 36, defaultTokens.margin),
    radius: clampNumber(tokens.radius, 0, 32, defaultTokens.radius),
    density: tokens.density === 'comfortable' ? 'comfortable' : 'compact',
    confirmDelete: tokens.confirmDelete !== false,
  }
  normalized.colorBackgroundGradient = composeBackgroundLayers(normalized)
  return normalized
}

export function buildAppearancePatch(tokens: DesignTokens, patch: Partial<DesignTokens>): DesignTokens {
  const next = normalizeTokens({ ...tokens, ...patch })
  if (patch.colorAccent) {
    next.colorAccentSoft = hexToRgba(patch.colorAccent, 0.16)
    next.colorAccentBorder = hexToRgba(patch.colorAccent, 0.7)
    next.colorAccentContrast = pickContrastColor(patch.colorAccent)
  }
  if (patch.colorBackground || patch.colorAccent) {
    next.colorNav = hexToRgba(mixHex(next.colorBackground, '#000000', 0.14), 0.94)
  }
  return next
}

export function tokensToCss(tokens: DesignTokens): string {
  const panelSurface = applyColorOpacity(tokens.colorSurface, tokens.panelOpacity)
  const panelSurfaceSubtle = applyColorOpacity(tokens.colorSurfaceSubtle, tokens.panelOpacity)
  const panelSurfaceStrong = applyColorOpacity(tokens.colorSurfaceStrong, Math.min(tokens.panelOpacity + 0.08, 1))
  const panelNav = applyColorOpacity(tokens.colorNav, Math.min(tokens.panelOpacity + 0.08, 1))
  const toolSurface = applyColorOpacity(tokens.colorToolBackground, tokens.panelOpacity)
  const actionBarSurface = applyColorOpacity(tokens.colorActionBarBackground, tokens.panelOpacity)
  return `
    --og-bg: ${tokens.colorBackground};
    --og-bg-gradient: ${tokens.colorBackgroundGradient};
    --og-surface: ${panelSurface};
    --og-surface-subtle: ${panelSurfaceSubtle};
    --og-surface-strong: ${panelSurfaceStrong};
    --og-tool-bg: ${toolSurface};
    --og-action-bar-bg: ${actionBarSurface};
    --og-text: ${tokens.colorText};
    --og-muted: ${tokens.colorMuted};
    --og-accent: ${tokens.colorAccent};
    --og-accent-soft: ${tokens.colorAccentSoft};
    --og-accent-border: ${tokens.colorAccentBorder};
    --og-accent-contrast: ${tokens.colorAccentContrast};
    --og-border: ${tokens.colorBorder};
    --og-nav-bg: ${panelNav};
    --og-panel-opacity: ${tokens.panelOpacity};
    --og-background-image-opacity: ${tokens.backgroundImageOpacity};
    --og-shadow: ${tokens.shadow};
    --og-margin: ${tokens.margin}px;
    --og-radius: ${tokens.radius}px;
    --og-card-radius: ${Math.max(tokens.radius - 4, 0)}px;
    --og-field-radius: ${Math.max(tokens.radius - 6, 0)}px;
    --og-control-height: ${tokens.density === 'compact' ? 32 : 40}px;
    --og-font: ${tokens.fontFamily};
    --page-gutter: ${tokens.margin}px;
    --space-xs: ${Math.round(tokens.margin * 0.35)}px;
    --space-sm: ${Math.round(tokens.margin * 0.5)}px;
    --space-md: ${Math.round(tokens.margin * 0.75)}px;
    --space-lg: ${tokens.margin}px;
    --space-xl: ${Math.round(tokens.margin * 1.25)}px;
    --panel-radius: ${tokens.radius}px;
    --card-radius: ${Math.max(tokens.radius - 4, 0)}px;
    --field-radius: ${Math.max(tokens.radius - 6, 0)}px;
    --accent: ${tokens.colorAccent};
    --accent-soft: ${tokens.colorAccentSoft};
    --accent-border: ${tokens.colorAccentBorder};
    --accent-contrast: ${tokens.colorAccentContrast};
    --app-font-family: ${tokens.fontFamily};
    --bg: ${tokens.colorBackground};
    --bg-gradient: ${tokens.colorBackgroundGradient};
    --text: ${tokens.colorText};
    --muted: ${tokens.colorMuted};
    --surface: ${panelSurface};
    --surface-subtle: ${panelSurfaceSubtle};
    --surface-strong: ${panelSurfaceStrong};
    --tool-bg: ${toolSurface};
    --action-bar-bg: ${actionBarSurface};
    --panel-surface: ${panelSurface};
    --panel-surface-subtle: ${panelSurfaceSubtle};
    --panel-surface-strong: ${panelSurfaceStrong};
    --border: ${tokens.colorBorder};
    --nav-bg: ${panelNav};
    --shadow: ${tokens.shadow};
  `
}

export function applyTokens(tokens: DesignTokens, root: HTMLElement = document.documentElement) {
  applyStoredFonts()
  root.setAttribute('style', tokensToCss(tokens))
}

export function createUiId(prefix = 'id'): string {
  if (globalThis.crypto && typeof globalThis.crypto.randomUUID === 'function') {
    return globalThis.crypto.randomUUID()
  }
  if (globalThis.crypto && typeof globalThis.crypto.getRandomValues === 'function') {
    const bytes = new Uint8Array(16)
    globalThis.crypto.getRandomValues(bytes)
    bytes[6] = (bytes[6] & 0x0f) | 0x40
    bytes[8] = (bytes[8] & 0x3f) | 0x80
    const hex = Array.from(bytes, (byte) => byte.toString(16).padStart(2, '0')).join('')
    return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`
  }
  return `${prefix}-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`
}

export function gradientForAppearance(enabled: boolean, background: string, accent: string) {
  if (!enabled) return 'none'
  return [
    `radial-gradient(circle at top left, ${hexToRgba(mixHex(background, '#28496f', 0.52), 0.28)}, transparent 42%)`,
    `radial-gradient(circle at top right, ${hexToRgba(mixHex(accent, '#0a4b38', 0.35), 0.22)}, transparent 38%)`,
  ].join(', ')
}

export function createBackgroundGradient(index: number, accent = defaultTokens.colorAccent): BackgroundGradient {
  return {
    id: createUiId('gradient'),
    name: `Gradient ${index + 1}`,
    strength: 0.28,
    points: [
      {
        id: createUiId('point'),
        color: accent,
        strength: 0.28,
        x: 50,
        y: 50,
        stop: 42,
      },
    ],
  }
}

export function composeBackgroundLayers(tokens: DesignTokens) {
  const gradientLayers = tokens.backgroundGradients.flatMap((gradient) =>
    gradient.points.map(
      (point) =>
        `radial-gradient(circle at ${clampNumber(point.x, 0, 100, 50)}% ${clampNumber(point.y, 0, 100, 50)}%, ${hexToRgba(point.color, clampNumber(point.strength, 0, 1, clampNumber(gradient.strength, 0, 1, 0.28)))}, transparent ${clampNumber(point.stop, 8, 100, 42)}%)`,
    ),
  )
  const imageLayers = tokens.backgroundImage
    ? [
        `linear-gradient(${hexToRgba(tokens.colorBackground, 1 - clampNumber(tokens.backgroundImageOpacity, 0, 1, 0.35))}, ${hexToRgba(tokens.colorBackground, 1 - clampNumber(tokens.backgroundImageOpacity, 0, 1, 0.35))})`,
        `url("${tokens.backgroundImage}")`,
      ]
    : []
  return [...gradientLayers, ...imageLayers].join(', ') || 'none'
}

function clampNumber(value: unknown, min: number, max: number, fallback: number) {
  return typeof value === 'number' && Number.isFinite(value) ? Math.min(max, Math.max(min, value)) : fallback
}

function hexToRgba(hex: string, alpha: number) {
  const normalized = normalizeHex(hex)
  const red = Number.parseInt(normalized.slice(1, 3), 16)
  const green = Number.parseInt(normalized.slice(3, 5), 16)
  const blue = Number.parseInt(normalized.slice(5, 7), 16)
  return `rgba(${red}, ${green}, ${blue}, ${alpha})`
}

function normalizeGradient(gradient: BackgroundGradient): BackgroundGradient {
  return {
    id: typeof gradient.id === 'string' ? gradient.id : createUiId('gradient'),
    name: typeof gradient.name === 'string' ? gradient.name : 'Gradient',
    strength: clampNumber(gradient.strength, 0, 1, 0.28),
    points: Array.isArray(gradient.points)
      ? gradient.points.map((point) => ({
          id: typeof point.id === 'string' ? point.id : createUiId('point'),
          color: normalizeHex(point.color),
          strength: clampNumber(point.strength, 0, 1, clampNumber(gradient.strength, 0, 1, 0.28)),
          x: clampNumber(point.x, 0, 100, 50),
          y: clampNumber(point.y, 0, 100, 50),
          stop: clampNumber(point.stop, 8, 100, 42),
        }))
      : [],
  }
}

function applyColorOpacity(color: string, opacity: number) {
  const alpha = clampNumber(opacity, 0.08, 1, 0.84)
  if (color.trim().startsWith('#')) return hexToRgba(color, alpha)
  const rgbaMatch = color.match(/^rgba?\(([^)]+)\)$/)
  if (!rgbaMatch) return color
  const parts = rgbaMatch[1].split(',').map((part) => part.trim())
  if (parts.length < 3) return color
  const baseAlpha = parts[3] ? Number(parts[3]) : 1
  const nextAlpha = Number.isFinite(baseAlpha) ? Math.min(1, baseAlpha * (alpha / defaultTokens.panelOpacity)) : alpha
  return `rgba(${parts[0]}, ${parts[1]}, ${parts[2]}, ${Number(nextAlpha.toFixed(3))})`
}

function normalizeHex(hex: string) {
  const clean = hex.trim().replace('#', '')
  const expanded = clean.length === 3 ? clean.split('').map((value) => value + value).join('') : clean
  return /^[0-9a-fA-F]{6}$/.test(expanded) ? `#${expanded}` : '#41b883'
}

function mixHex(baseHex: string, targetHex: string, factor: number) {
  const base = normalizeHex(baseHex)
  const target = normalizeHex(targetHex)
  const baseValues = [base.slice(1, 3), base.slice(3, 5), base.slice(5, 7)].map((part) => Number.parseInt(part, 16))
  const targetValues = [target.slice(1, 3), target.slice(3, 5), target.slice(5, 7)].map((part) => Number.parseInt(part, 16))
  const mixed = baseValues.map((value, index) => Math.round(value + (targetValues[index] - value) * factor))
  return `#${mixed.map((value) => value.toString(16).padStart(2, '0')).join('')}`
}

function pickContrastColor(hex: string) {
  const normalized = normalizeHex(hex)
  const red = Number.parseInt(normalized.slice(1, 3), 16)
  const green = Number.parseInt(normalized.slice(3, 5), 16)
  const blue = Number.parseInt(normalized.slice(5, 7), 16)
  const luminance = (0.299 * red + 0.587 * green + 0.114 * blue) / 255
  return luminance > 0.58 ? '#071019' : '#edf5fb'
}
