<script lang="ts">
  import type { AppearanceTheme, BackgroundGradient, BackgroundGradientPoint, CurrentSession, CustomFont, DesignTokens } from '@og-suite/contracts'
  import type { RuntimeServices } from '@og-suite/runtime'
  import Icon from '@og-suite/ui/Icon'
  import {
    builtInFontOptions,
    buildAppearancePatch,
    createBackgroundGradient,
    createUiId,
    defaultTokens,
    fontFamilyForCustomFont,
    lightTokens,
    loadStoredFonts,
    normalizeFontFamilyName,
    normalizeTokens,
    saveStoredFonts,
  } from '@og-suite/ui'
  import { onMount } from 'svelte'

  export let tokens: DesignTokens
  export let onTokensChange: (tokens: DesignTokens) => void
  export let onClose: () => void
  export let services: RuntimeServices | undefined = undefined

  type ThemeMode = 'dark' | 'light' | 'custom'

  type SavedAppearanceTheme = AppearanceTheme

  const savedThemeStorageKey = 'og-suite:appearance-themes'

  let backgroundImageInput: HTMLInputElement | null = null
  let themeImportInput: HTMLInputElement | null = null
  let fontImportInput: HTMLInputElement | null = null
  let activeLocationPicker = ''
  let behaviorSection: HTMLElement | null = null
  let appearanceSection: HTMLElement | null = null
  let selectedThemeMode: ThemeMode = 'custom'
  let themeName = 'Custom theme'
  let savedThemes: SavedAppearanceTheme[] = loadSavedThemes()
  let shareCurrentTheme = true
  let importStatus = ''
  let themeStorageStatus = ''
  let currentUserId = ''
  let customFonts: CustomFont[] = loadStoredFonts()
  let fontStatus = ''
  $: gradientPointRows = tokens.backgroundGradients.flatMap((gradient) =>
    gradient.points.map((point) => ({ gradientId: gradient.id, point })),
  )
  $: serverThemeStorage = Boolean(services && services.runtimeMode !== 'local')
  $: fontOptions = [
    ...builtInFontOptions,
    ...customFonts.map((font) => ({ label: font.name, value: fontFamilyForCustomFont(font) })),
  ]

  onMount(() => {
    if (serverThemeStorage) void loadServerThemes()
  })

  function markThemeDirty() {
    selectedThemeMode = 'custom'
    themeName = ''
  }

  function patch(nextPatch: Partial<DesignTokens>) {
    markThemeDirty()
    onTokensChange(buildAppearancePatch(tokens, nextPatch))
  }

  function scrollToSection(section: HTMLElement | null) {
    section?.scrollIntoView({ block: 'start', behavior: 'smooth' })
  }

  function confirmDeleteAction(label: string) {
    return !tokens.confirmDelete || window.confirm(`Delete ${label}?`)
  }

  function resetAppearance() {
    applyThemePreset(defaultTokens, 'dark', 'Dark theme')
  }

  function activeThemeMode(): ThemeMode {
    if (tokensMatchPreset(tokens, defaultTokens)) return 'dark'
    if (tokensMatchPreset(tokens, lightTokens)) return 'light'
    return selectedThemeMode === 'light' || selectedThemeMode === 'dark' ? 'custom' : selectedThemeMode
  }

  function applyThemePreset(nextTokens: DesignTokens, mode: ThemeMode, name = mode === 'light' ? 'Light theme' : mode === 'dark' ? 'Dark theme' : 'Custom theme') {
    selectedThemeMode = mode
    themeName = name
    onTokensChange(normalizeTokens({ ...nextTokens, confirmDelete: tokens.confirmDelete }))
  }

  async function saveCurrentTheme() {
    const name = themeName.trim() || 'Custom theme'
    const request = {
      name,
      tokens: normalizeTokens(tokens),
      isShared: shareCurrentTheme,
    }
    if (serverThemeStorage && services) {
      try {
        const theme = await services.api.post<SavedAppearanceTheme>('/api/v1/appearance/themes', request)
        savedThemes = [theme, ...savedThemes.filter((savedTheme) => savedTheme.id !== theme.id)]
        themeName = `${name} copy`
        themeStorageStatus = theme.isShared ? 'Theme saved to server and shared with this workspace.' : 'Theme saved privately to the server.'
        return
      } catch (error) {
        themeStorageStatus = error instanceof Error ? error.message : 'Could not save theme to the server.'
      }
    }
    const now = new Date().toISOString()
    const theme: SavedAppearanceTheme = {
      id: createUiId('theme'),
      name,
      tokens: request.tokens,
      ownerId: 'local',
      workspaceId: 'local',
      isShared: false,
      createdAt: now,
      updatedAt: now,
    }
    savedThemes = [theme, ...savedThemes]
    saveSavedThemes(savedThemes)
    themeName = `${name} copy`
    themeStorageStatus = 'Theme saved on this device.'
  }

  function applySavedTheme(theme: SavedAppearanceTheme) {
    applyThemePreset(theme.tokens, 'custom', theme.name)
  }

  function canManageTheme(theme: SavedAppearanceTheme) {
    return !serverThemeStorage || !currentUserId || theme.ownerId === currentUserId
  }

  async function removeSavedTheme(themeId: string) {
    if (!confirmDeleteAction('this saved theme')) return
    if (serverThemeStorage && services) {
      try {
        await services.api.delete(`/api/v1/appearance/themes/${themeId}`)
        savedThemes = savedThemes.filter((theme) => theme.id !== themeId)
        themeStorageStatus = 'Theme deleted from the server.'
        return
      } catch (error) {
        themeStorageStatus = error instanceof Error ? error.message : 'Could not delete server theme.'
        return
      }
    }
    savedThemes = savedThemes.filter((theme) => theme.id !== themeId)
    saveSavedThemes(savedThemes)
  }

  async function renameSavedTheme(theme: SavedAppearanceTheme) {
    const nextName = window.prompt('Theme name', theme.name)?.trim()
    if (!nextName || nextName === theme.name) return
    if (serverThemeStorage && services) {
      try {
        const updated = await services.api.patch<SavedAppearanceTheme>(`/api/v1/appearance/themes/${theme.id}`, { name: nextName })
        savedThemes = savedThemes.map((savedTheme) => savedTheme.id === updated.id ? updated : savedTheme)
        if (themeName === theme.name) themeName = updated.name
        themeStorageStatus = 'Theme renamed on the server.'
        return
      } catch (error) {
        themeStorageStatus = error instanceof Error ? error.message : 'Could not rename server theme.'
        return
      }
    }
    savedThemes = savedThemes.map((savedTheme) => savedTheme.id === theme.id ? { ...savedTheme, name: nextName } : savedTheme)
    saveSavedThemes(savedThemes)
    if (themeName === theme.name) themeName = nextName
  }

  async function toggleThemeShared(theme: SavedAppearanceTheme) {
    if (!serverThemeStorage || !services) return
    try {
      const updated = await services.api.patch<SavedAppearanceTheme>(`/api/v1/appearance/themes/${theme.id}`, { isShared: !theme.isShared })
      savedThemes = savedThemes.map((savedTheme) => savedTheme.id === updated.id ? updated : savedTheme)
      themeStorageStatus = updated.isShared ? 'Theme is shared with this workspace.' : 'Theme is private to you.'
    } catch (error) {
      themeStorageStatus = error instanceof Error ? error.message : 'Could not update theme sharing.'
    }
  }

  async function loadServerThemes() {
    if (!services) return
    try {
      const session = await services.api.get<CurrentSession>('/api/v1/auth/session')
      currentUserId = session.user.id
      savedThemes = await services.api.get<SavedAppearanceTheme[]>('/api/v1/appearance/themes')
      themeStorageStatus = 'Themes loaded from the server.'
    } catch (error) {
      themeStorageStatus = error instanceof Error ? error.message : 'Could not load server themes.'
    }
  }

  function exportTheme(theme: SavedAppearanceTheme | null) {
    const payload = theme ?? {
      id: createUiId('theme'),
      name: themeName.trim() || 'Current appearance',
      tokens: normalizeTokens(tokens),
      createdAt: new Date().toISOString(),
    }
    const blob = new Blob([JSON.stringify(payload, null, 2)], { type: 'application/json' })
    const link = document.createElement('a')
    link.href = URL.createObjectURL(blob)
    link.download = `${slugify(payload.name)}.og-theme.json`
    link.click()
    URL.revokeObjectURL(link.href)
  }

  function importThemes(file: File | undefined) {
    if (!file) return
    const reader = new FileReader()
    reader.addEventListener('load', () => {
      try {
        const imported = parseThemeImport(reader.result)
        if (imported.length === 0) throw new Error('No themes found')
        savedThemes = [...imported, ...savedThemes]
        if (serverThemeStorage && services) {
          void uploadImportedThemes(imported)
        } else {
          saveSavedThemes(savedThemes)
        }
        importStatus = `Imported ${imported.length} theme${imported.length === 1 ? '' : 's'}`
      } catch {
        importStatus = 'Could not import theme file'
      } finally {
        if (themeImportInput) themeImportInput.value = ''
      }
    })
    reader.readAsText(file)
  }

  async function uploadImportedThemes(themes: SavedAppearanceTheme[]) {
    if (!services) return
    try {
      const created = []
      for (const theme of themes) {
        created.push(await services.api.post<SavedAppearanceTheme>('/api/v1/appearance/themes', {
          name: theme.name,
          tokens: theme.tokens,
          isShared: theme.isShared,
        }))
      }
      savedThemes = [...created, ...savedThemes.filter((theme) => !themes.some((imported) => imported.id === theme.id))]
      themeStorageStatus = `Imported ${created.length} theme${created.length === 1 ? '' : 's'} to the server.`
    } catch (error) {
      themeStorageStatus = error instanceof Error ? error.message : 'Could not upload imported themes.'
    }
  }

  function patchGradients(backgroundGradients: BackgroundGradient[]) {
    patch({ backgroundGradients })
  }

  function addGradientPoint() {
    if (tokens.backgroundGradients[0]) {
      addPoint(tokens.backgroundGradients[0].id)
      return
    }
    patchGradients([createBackgroundGradient(0, tokens.colorAccent)])
  }

  function addPoint(gradientId: string) {
    const point: BackgroundGradientPoint = {
      id: createUiId('point'),
      color: tokens.colorAccent,
      strength: 0.28,
      x: 50,
      y: 50,
      stop: 42,
    }
    patchGradients(tokens.backgroundGradients.map((gradient) => gradient.id === gradientId ? { ...gradient, points: [...gradient.points, point] } : gradient))
  }

  function removePoint(gradientId: string, pointId: string) {
    if (!confirmDeleteAction('this gradient point')) return
    patchGradients(
      tokens.backgroundGradients
        .map((gradient) =>
          gradient.id === gradientId ? { ...gradient, points: gradient.points.filter((point) => point.id !== pointId) } : gradient,
        )
        .filter((gradient) => gradient.points.length > 0),
    )
  }

  function updatePoint(gradientId: string, pointId: string, patchValue: Partial<BackgroundGradientPoint>) {
    patchGradients(
      tokens.backgroundGradients.map((gradient) =>
        gradient.id === gradientId
          ? { ...gradient, points: gradient.points.map((point) => point.id === pointId ? { ...point, ...patchValue } : point) }
          : gradient,
      ),
    )
  }

  function setPointLocation(event: MouseEvent, gradientId: string, pointId: string) {
    const box = event.currentTarget as HTMLElement
    const rect = box.getBoundingClientRect()
    updatePoint(gradientId, pointId, {
      x: Math.round(((event.clientX - rect.left) / rect.width) * 100),
      y: Math.round(((event.clientY - rect.top) / rect.height) * 100),
    })
  }

  function uploadBackgroundImage(file: File | undefined) {
    if (!file) return
    const reader = new FileReader()
    reader.addEventListener('load', () => {
      if (typeof reader.result === 'string') patch({ backgroundImage: reader.result })
    })
    reader.readAsDataURL(file)
  }

  function importFont(file: File | undefined) {
    if (!file) return
    const name = window.prompt('Font name', fontNameFromFile(file.name))?.trim()
    const family = normalizeFontFamilyName(name ?? '')
    if (!family) {
      if (fontImportInput) fontImportInput.value = ''
      return
    }
    const reader = new FileReader()
    reader.addEventListener('load', () => {
      if (typeof reader.result !== 'string') return
      const font: CustomFont = {
        id: createUiId('font'),
        name: family,
        family,
        dataUrl: reader.result,
        format: fontFormat(file.name, file.type),
        createdAt: new Date().toISOString(),
      }
      customFonts = [font, ...customFonts.filter((item) => item.family !== font.family)]
      saveStoredFonts(customFonts)
      fontStatus = `Imported ${font.name}.`
      if (fontImportInput) fontImportInput.value = ''
    })
    reader.readAsDataURL(file)
  }

  function removeCustomFont(font: CustomFont) {
    if (!confirmDeleteAction(`font "${font.name}"`)) return
    customFonts = customFonts.filter((item) => item.id !== font.id)
    saveStoredFonts(customFonts)
    if (tokens.fontFamily === fontFamilyForCustomFont(font)) patch({ fontFamily: defaultTokens.fontFamily })
    fontStatus = `Removed ${font.name}.`
  }

  function fontNameFromFile(filename: string) {
    return filename.replace(/\.[^.]+$/, '').replace(/[-_]+/g, ' ').trim() || 'Custom Font'
  }

  function fontFormat(filename: string, mimeType: string) {
    const lowerName = filename.toLowerCase()
    if (lowerName.endsWith('.woff2') || mimeType.includes('woff2')) return 'woff2'
    if (lowerName.endsWith('.woff') || mimeType.includes('woff')) return 'woff'
    if (lowerName.endsWith('.otf') || mimeType.includes('opentype')) return 'opentype'
    if (lowerName.endsWith('.ttf') || mimeType.includes('truetype')) return 'truetype'
    return 'woff2'
  }

  function loadSavedThemes(): SavedAppearanceTheme[] {
    if (typeof localStorage === 'undefined') return []
    const raw = localStorage.getItem(savedThemeStorageKey)
    if (!raw) return []
    try {
      const parsed = JSON.parse(raw) as unknown
      if (!Array.isArray(parsed)) return []
      return parsed.map(normalizeSavedTheme).filter((theme): theme is SavedAppearanceTheme => Boolean(theme))
    } catch {
      return []
    }
  }

  function saveSavedThemes(themes: SavedAppearanceTheme[]) {
    if (typeof localStorage === 'undefined') return
    localStorage.setItem(savedThemeStorageKey, JSON.stringify(themes))
  }

  function parseThemeImport(result: string | ArrayBuffer | null): SavedAppearanceTheme[] {
    if (typeof result !== 'string') return []
    const parsed = JSON.parse(result) as unknown
    const items = Array.isArray(parsed) ? parsed : [parsed]
    return items.map(normalizeSavedTheme).filter((theme): theme is SavedAppearanceTheme => Boolean(theme))
  }

  function normalizeSavedTheme(value: unknown): SavedAppearanceTheme | null {
    if (!value || typeof value !== 'object') return null
    const item = value as Partial<SavedAppearanceTheme> & { tokens?: Partial<DesignTokens> }
    if (!item.tokens || typeof item.tokens !== 'object') return null
    return {
      id: typeof item.id === 'string' ? item.id : createUiId('theme'),
      name: typeof item.name === 'string' && item.name.trim() ? item.name.trim() : 'Imported theme',
      tokens: normalizeTokens(item.tokens),
      ownerId: typeof item.ownerId === 'string' ? item.ownerId : 'local',
      workspaceId: typeof item.workspaceId === 'string' ? item.workspaceId : 'local',
      isShared: typeof item.isShared === 'boolean' ? item.isShared : false,
      createdAt: typeof item.createdAt === 'string' ? item.createdAt : new Date().toISOString(),
      updatedAt: typeof item.updatedAt === 'string' ? item.updatedAt : typeof item.createdAt === 'string' ? item.createdAt : new Date().toISOString(),
    }
  }

  function tokensMatchPreset(current: DesignTokens, preset: DesignTokens) {
    return JSON.stringify(themeComparable(current)) === JSON.stringify(themeComparable(preset))
  }

  function themeComparable(value: DesignTokens) {
    const { colorBackgroundGradient, confirmDelete, ...appearance } = normalizeTokens(value)
    return appearance
  }

  function slugify(value: string) {
    return value.trim().toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/(^-|-$)/g, '') || 'appearance-theme'
  }

  function colorPickerValue(value: string, fallback: string) {
    const trimmed = value.trim()
    const hex = parseHexColor(trimmed)
    if (hex) return hex
    const rgb = parseRgbColor(trimmed)
    if (rgb) return rgb
    return parseHexColor(fallback) ?? '#000000'
  }

  function parseHexColor(value: string) {
    const clean = value.replace('#', '')
    const expanded = clean.length === 3 ? clean.split('').map((part) => part + part).join('') : clean
    return /^[0-9a-fA-F]{6}$/.test(expanded) ? `#${expanded.toLowerCase()}` : null
  }

  function parseRgbColor(value: string) {
    const match = value.match(/^rgba?\(([^)]+)\)$/i)
    if (!match) return null
    const channels = match[1]
      .split(',')
      .slice(0, 3)
      .map((part) => Number(part.trim()))
    if (channels.length !== 3 || channels.some((channel) => !Number.isFinite(channel))) return null
    return `#${channels.map((channel) => Math.max(0, Math.min(255, Math.round(channel))).toString(16).padStart(2, '0')).join('')}`
  }
</script>

<div class="settings-backdrop" role="presentation" on:click={onClose}></div>
<aside class="settings-drawer" aria-label="Appearance settings">
  <header class="settings-drawer-header">
    <div>
      <h2>Settings</h2>
    </div>
    <button class="settings-close" aria-label="Close settings" title="Close settings" on:click={onClose}>
      <span aria-hidden="true"></span>
    </button>
  </header>

  <nav class="settings-nav" aria-label="Settings sections">
    <button on:click={() => scrollToSection(behaviorSection)}>Behavior</button>
    <button on:click={() => scrollToSection(appearanceSection)}>Appearance</button>
  </nav>

  <section bind:this={behaviorSection} class="settings-section" aria-labelledby="behavior-settings-heading">
    <h3 id="behavior-settings-heading">Behavior</h3>
    <div class="settings-card">
      <label class="settings-toggle">
        <input type="checkbox" checked={tokens.confirmDelete} on:change={(event) => patch({ confirmDelete: event.currentTarget.checked })} />
        <span>Confirm before delete actions</span>
      </label>
    </div>
  </section>

  <section bind:this={appearanceSection} class="settings-section" aria-labelledby="appearance-settings-heading">
    <h3 id="appearance-settings-heading">Appearance</h3>
  </section>

  <section class="settings-card">
    <h3>Theme</h3>
    <div class="theme-mode-grid" role="group" aria-label="Theme mode">
      <button
        class:active={activeThemeMode() === 'light'}
        aria-pressed={activeThemeMode() === 'light'}
        on:click={() => applyThemePreset(lightTokens, 'light', 'Light theme')}
      >
        Light
      </button>
      <button
        class:active={activeThemeMode() === 'dark'}
        aria-pressed={activeThemeMode() === 'dark'}
        on:click={() => applyThemePreset(defaultTokens, 'dark', 'Dark theme')}
      >
        Dark
      </button>
      <button
        class:active={activeThemeMode() === 'custom'}
        aria-pressed={activeThemeMode() === 'custom'}
        on:click={() => selectedThemeMode = 'custom'}
      >
        Custom
      </button>
    </div>

    <label class="settings-field">
      <span>Theme name</span>
      <input value={themeName} placeholder="Unsaved custom theme" on:input={(event) => themeName = event.currentTarget.value} />
    </label>
    {#if serverThemeStorage}
      <label class="settings-toggle">
        <input type="checkbox" checked={shareCurrentTheme} on:change={(event) => shareCurrentTheme = event.currentTarget.checked} />
        <span>Share saved themes with this workspace</span>
      </label>
    {/if}

    <div class="settings-actions-inline">
      <button class="icon-label-button" on:click={() => void saveCurrentTheme()}>
        <Icon name="save" size={18} />
        <span>Save current</span>
      </button>
      <button class="icon-label-button" on:click={() => exportTheme(null)}>
        <Icon name="download" size={18} />
        <span>Export current</span>
      </button>
      <button class="icon-label-button" on:click={() => themeImportInput?.click()}>
        <Icon name="upload" size={18} />
        <span>Import</span>
      </button>
      <input
        bind:this={themeImportInput}
        class="hidden-file-input"
        type="file"
        accept="application/json,.json,.og-theme.json"
        on:change={(event) => importThemes(event.currentTarget.files?.[0])}
      />
    </div>

    {#if importStatus}
      <div class="settings-empty">{importStatus}</div>
    {/if}
    {#if themeStorageStatus}
      <div class="settings-empty">{themeStorageStatus}</div>
    {/if}

    {#if savedThemes.length > 0}
      <div class="saved-theme-list">
        {#each savedThemes as theme}
          <div class="saved-theme-row">
            <button class="saved-theme-apply" on:click={() => applySavedTheme(theme)}>
              <span>{theme.name}</span>
              <small>{serverThemeStorage ? (theme.isShared ? 'Shared' : 'Private') : 'This device'} · {new Date(theme.createdAt).toLocaleDateString()}</small>
            </button>
            <button aria-label={`Export ${theme.name}`} title="Export theme" on:click={() => exportTheme(theme)}>
              <Icon name="download" size={16} />
            </button>
            {#if serverThemeStorage && canManageTheme(theme)}
              <button aria-label={theme.isShared ? `Make ${theme.name} private` : `Share ${theme.name}`} title={theme.isShared ? 'Make private' : 'Share theme'} on:click={() => void toggleThemeShared(theme)}>
                <Icon name="share" size={16} />
              </button>
            {/if}
            {#if canManageTheme(theme)}
              <button aria-label={`Rename ${theme.name}`} title="Rename theme" on:click={() => void renameSavedTheme(theme)}>
                <Icon name="rename" size={16} />
              </button>
              <button aria-label={`Delete ${theme.name}`} title="Delete theme" on:click={() => void removeSavedTheme(theme.id)}>
                <Icon name="delete" size={16} />
              </button>
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  </section>

  <section class="settings-card">
    <h3>Shape and Density</h3>
    <label class="settings-field">
      <span>Margins: {tokens.margin}px</span>
      <input
        type="range"
        min="4"
        max="36"
        value={tokens.margin}
        on:input={(event) => patch({ margin: Number(event.currentTarget.value) })}
      />
    </label>
    <label class="settings-field">
      <span>Corner rounding: {tokens.radius}px</span>
      <input
        type="range"
        min="0"
        max="32"
        value={tokens.radius}
        on:input={(event) => patch({ radius: Number(event.currentTarget.value) })}
      />
    </label>
    <label class="settings-field">
      <span>Density</span>
      <select value={tokens.density} on:change={(event) => patch({ density: event.currentTarget.value === 'comfortable' ? 'comfortable' : 'compact' })}>
        <option value="compact">Compact</option>
        <option value="comfortable">Comfortable</option>
      </select>
    </label>
  </section>

  <section class="settings-card">
    <h3>Color</h3>
    <label class="settings-field color-field">
      <span>Accent</span>
      <input type="color" value={tokens.colorAccent} on:input={(event) => patch({ colorAccent: event.currentTarget.value })} />
      <input value={tokens.colorAccent} on:change={(event) => patch({ colorAccent: event.currentTarget.value })} />
    </label>
    <label class="settings-field color-field">
      <span>App background</span>
      <input type="color" value={tokens.colorBackground} on:input={(event) => patch({ colorBackground: event.currentTarget.value })} />
      <input value={tokens.colorBackground} on:change={(event) => patch({ colorBackground: event.currentTarget.value })} />
    </label>
    <label class="settings-field color-field">
      <span>Panels and cards</span>
      <input
        type="color"
        value={colorPickerValue(tokens.colorSurface, defaultTokens.colorSurface)}
        on:input={(event) => patch({ colorSurface: event.currentTarget.value })}
      />
      <input value={tokens.colorSurface} on:change={(event) => patch({ colorSurface: event.currentTarget.value })} />
    </label>
    <label class="settings-field color-field">
      <span>Secondary surfaces</span>
      <input
        type="color"
        value={colorPickerValue(tokens.colorSurfaceSubtle, defaultTokens.colorSurfaceSubtle)}
        on:input={(event) => patch({ colorSurfaceSubtle: event.currentTarget.value })}
      />
      <input value={tokens.colorSurfaceSubtle} on:change={(event) => patch({ colorSurfaceSubtle: event.currentTarget.value })} />
    </label>
    <label class="settings-field color-field">
      <span>Raised surfaces</span>
      <input
        type="color"
        value={colorPickerValue(tokens.colorSurfaceStrong, defaultTokens.colorSurfaceStrong)}
        on:input={(event) => patch({ colorSurfaceStrong: event.currentTarget.value })}
      />
      <input value={tokens.colorSurfaceStrong} on:change={(event) => patch({ colorSurfaceStrong: event.currentTarget.value })} />
    </label>
    <label class="settings-field color-field">
      <span>Text</span>
      <input type="color" value={tokens.colorText} on:input={(event) => patch({ colorText: event.currentTarget.value })} />
      <input value={tokens.colorText} on:change={(event) => patch({ colorText: event.currentTarget.value })} />
    </label>
    <label class="settings-field color-field">
      <span>Muted text</span>
      <input type="color" value={tokens.colorMuted} on:input={(event) => patch({ colorMuted: event.currentTarget.value })} />
      <input value={tokens.colorMuted} on:change={(event) => patch({ colorMuted: event.currentTarget.value })} />
    </label>
    <label class="settings-field color-field">
      <span>Notes editor background</span>
      <input
        type="color"
        value={colorPickerValue(tokens.colorToolBackground, defaultTokens.colorToolBackground)}
        on:input={(event) => patch({ colorToolBackground: event.currentTarget.value })}
      />
      <input value={tokens.colorToolBackground} on:change={(event) => patch({ colorToolBackground: event.currentTarget.value })} />
    </label>
    <label class="settings-field color-field">
      <span>Action bar background</span>
      <input
        type="color"
        value={colorPickerValue(tokens.colorActionBarBackground, defaultTokens.colorActionBarBackground)}
        on:input={(event) => patch({ colorActionBarBackground: event.currentTarget.value })}
      />
      <input value={tokens.colorActionBarBackground} on:change={(event) => patch({ colorActionBarBackground: event.currentTarget.value })} />
    </label>
    <label class="settings-field color-field">
      <span>Navigation/menu background</span>
      <input
        type="color"
        value={colorPickerValue(tokens.colorNav, defaultTokens.colorNav)}
        on:input={(event) => patch({ colorNav: event.currentTarget.value })}
      />
      <input value={tokens.colorNav} on:change={(event) => patch({ colorNav: event.currentTarget.value })} />
    </label>
    <label class="settings-field">
      <span>Panel opacity: {Math.round(tokens.panelOpacity * 100)}%</span>
      <input
        type="range"
        min="8"
        max="100"
        value={Math.round(tokens.panelOpacity * 100)}
        on:input={(event) => patch({ panelOpacity: Number(event.currentTarget.value) / 100 })}
      />
    </label>
  </section>

  <section class="settings-card">
    <div class="settings-section-header">
      <h3>Background Gradients</h3>
      <button class="icon-label-button" on:click={addGradientPoint}>
        <Icon name="add-list" size={18} />
        <span>Add</span>
      </button>
    </div>

    {#if gradientPointRows.length === 0}
      <div class="settings-empty">No gradient points</div>
    {/if}

    <div class="gradient-points">
      {#each gradientPointRows as row, pointIndex}
        <div class="gradient-point">
          <button class="point-delete-button" aria-label="Remove point" title="Remove point" on:click={() => removePoint(row.gradientId, row.point.id)}>
            <Icon name="delete" size={16} />
          </button>
          <label class="settings-field color-field">
            <span>Point {pointIndex + 1}</span>
            <input type="color" value={row.point.color} on:input={(event) => updatePoint(row.gradientId, row.point.id, { color: event.currentTarget.value })} />
            <input value={row.point.color} on:change={(event) => updatePoint(row.gradientId, row.point.id, { color: event.currentTarget.value })} />
          </label>
          <label class="settings-field">
            <span>Spread: {row.point.stop}%</span>
            <input
              type="range"
              min="8"
              max="100"
              value={row.point.stop}
              on:input={(event) => updatePoint(row.gradientId, row.point.id, { stop: Number(event.currentTarget.value) })}
            />
          </label>
          <label class="settings-field">
            <span>Strength: {Math.round(row.point.strength * 100)}%</span>
            <input
              type="range"
              min="0"
              max="100"
              value={Math.round(row.point.strength * 100)}
              on:input={(event) => updatePoint(row.gradientId, row.point.id, { strength: Number(event.currentTarget.value) / 100 })}
            />
          </label>
          <div class="point-actions">
            <button class="icon-label-button" on:click={() => activeLocationPicker = activeLocationPicker === row.point.id ? '' : row.point.id}>
              <Icon name="expand" size={16} />
              <span>{Math.round(row.point.x)}%, {Math.round(row.point.y)}%</span>
            </button>
          </div>
          {#if activeLocationPicker === row.point.id}
            <button
              class="location-picker"
              style={`--picker-x: ${row.point.x}%; --picker-y: ${row.point.y}%;`}
              aria-label="Pick gradient location"
              on:click={(event) => setPointLocation(event, row.gradientId, row.point.id)}
            >
              <span></span>
            </button>
          {/if}
        </div>
      {/each}
    </div>
  </section>

  <section class="settings-card">
    <h3>Background Image</h3>
    <div class="settings-actions-inline">
      <button class="icon-label-button" on:click={() => backgroundImageInput?.click()}>
        <Icon name="upload" size={18} />
        <span>Upload</span>
      </button>
      <button class="icon-label-button" on:click={() => confirmDeleteAction('the background image') && patch({ backgroundImage: '' })}>
        <Icon name="delete" size={18} />
        <span>Remove</span>
      </button>
      <input
        bind:this={backgroundImageInput}
        class="hidden-file-input"
        type="file"
        accept="image/*"
        on:change={(event) => uploadBackgroundImage(event.currentTarget.files?.[0])}
      />
    </div>
    <label class="settings-field">
      <span>Image opacity: {Math.round(tokens.backgroundImageOpacity * 100)}%</span>
      <input
        type="range"
        min="0"
        max="100"
        value={Math.round(tokens.backgroundImageOpacity * 100)}
        on:input={(event) => patch({ backgroundImageOpacity: Number(event.currentTarget.value) / 100 })}
      />
    </label>
    {#if tokens.backgroundImage}
      <div class="background-preview" style={`background-image: url("${tokens.backgroundImage}")`}></div>
    {/if}
  </section>

  <section class="settings-card">
    <h3>Typography</h3>
    <label class="settings-field">
      <span>Font</span>
      <select value={tokens.fontFamily} on:change={(event) => patch({ fontFamily: event.currentTarget.value })}>
        {#each fontOptions as option}
          <option value={option.value}>{option.label}</option>
        {/each}
      </select>
    </label>
    <div class="settings-actions-inline">
      <button class="icon-label-button" on:click={() => fontImportInput?.click()}>
        <Icon name="upload" size={18} />
        <span>Import font</span>
      </button>
      <input
        bind:this={fontImportInput}
        class="hidden-file-input"
        type="file"
        accept=".woff2,.woff,.ttf,.otf,font/woff2,font/woff,font/ttf,font/otf"
        on:change={(event) => importFont(event.currentTarget.files?.[0])}
      />
    </div>
    {#if fontStatus}
      <div class="settings-empty">{fontStatus}</div>
    {/if}
    {#if customFonts.length > 0}
      <div class="custom-font-list">
        {#each customFonts as font}
          <div class="custom-font-row">
            <button class="custom-font-apply" style={`font-family: ${fontFamilyForCustomFont(font)};`} on:click={() => patch({ fontFamily: fontFamilyForCustomFont(font) })}>
              <span>{font.name}</span>
              <small>{font.format}</small>
            </button>
            <button aria-label={`Delete ${font.name}`} title="Delete font" on:click={() => removeCustomFont(font)}>
              <Icon name="delete" size={16} />
            </button>
          </div>
        {/each}
      </div>
    {/if}
  </section>

  <footer class="settings-actions">
    <button class="icon-label-button" on:click={resetAppearance}>
      <Icon name="refresh" size={18} />
      <span>Reset</span>
    </button>
    <button class="primary icon-label-button" on:click={onClose}>
      <Icon name="save" size={18} />
      <span>Done</span>
    </button>
  </footer>
</aside>
