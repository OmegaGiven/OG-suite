<script lang="ts">
  import type { BackgroundGradient, BackgroundGradientPoint, DesignTokens } from '@og-suite/contracts'
  import Icon from '@og-suite/ui/Icon'
  import { buildAppearancePatch, createBackgroundGradient, defaultTokens, lightTokens, normalizeTokens } from '@og-suite/ui'

  export let tokens: DesignTokens
  export let onTokensChange: (tokens: DesignTokens) => void
  export let onClose: () => void

  type ThemeMode = 'dark' | 'light' | 'custom'

  type SavedAppearanceTheme = {
    id: string
    name: string
    tokens: DesignTokens
    createdAt: string
  }

  const savedThemeStorageKey = 'og-suite:appearance-themes'

  const fontOptions = [
    { label: 'Plex Sans', value: '"IBM Plex Sans", "Segoe UI", system-ui, sans-serif' },
    { label: 'System UI', value: 'system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif' },
    { label: 'Avenir', value: '"Avenir Next", "Helvetica Neue", sans-serif' },
    { label: 'Serif', value: 'Georgia, "Times New Roman", serif' },
    { label: 'Mono', value: '"IBM Plex Mono", "SFMono-Regular", monospace' },
  ]

  let backgroundImageInput: HTMLInputElement | null = null
  let themeImportInput: HTMLInputElement | null = null
  let activeLocationPicker = ''
  let behaviorSection: HTMLElement | null = null
  let appearanceSection: HTMLElement | null = null
  let selectedThemeMode: ThemeMode = 'custom'
  let themeName = 'Custom theme'
  let savedThemes: SavedAppearanceTheme[] = loadSavedThemes()
  let importStatus = ''
  $: gradientPointRows = tokens.backgroundGradients.flatMap((gradient) =>
    gradient.points.map((point) => ({ gradientId: gradient.id, point })),
  )

  function patch(nextPatch: Partial<DesignTokens>) {
    selectedThemeMode = 'custom'
    onTokensChange(buildAppearancePatch(tokens, nextPatch))
  }

  function scrollToSection(section: HTMLElement | null) {
    section?.scrollIntoView({ block: 'start', behavior: 'smooth' })
  }

  function confirmDeleteAction(label: string) {
    return !tokens.confirmDelete || window.confirm(`Delete ${label}?`)
  }

  function resetAppearance() {
    applyThemePreset(defaultTokens, 'dark')
  }

  function activeThemeMode(): ThemeMode {
    if (tokensMatchPreset(tokens, defaultTokens)) return 'dark'
    if (tokensMatchPreset(tokens, lightTokens)) return 'light'
    return selectedThemeMode === 'light' || selectedThemeMode === 'dark' ? 'custom' : selectedThemeMode
  }

  function applyThemePreset(nextTokens: DesignTokens, mode: ThemeMode) {
    selectedThemeMode = mode
    onTokensChange(normalizeTokens({ ...nextTokens, confirmDelete: tokens.confirmDelete }))
  }

  function saveCurrentTheme() {
    const name = themeName.trim() || 'Custom theme'
    const theme: SavedAppearanceTheme = {
      id: crypto.randomUUID(),
      name,
      tokens: normalizeTokens(tokens),
      createdAt: new Date().toISOString(),
    }
    savedThemes = [theme, ...savedThemes]
    saveSavedThemes(savedThemes)
    themeName = `${name} copy`
  }

  function applySavedTheme(theme: SavedAppearanceTheme) {
    applyThemePreset(theme.tokens, 'custom')
  }

  function removeSavedTheme(themeId: string) {
    if (!confirmDeleteAction('this saved theme')) return
    savedThemes = savedThemes.filter((theme) => theme.id !== themeId)
    saveSavedThemes(savedThemes)
  }

  function exportTheme(theme: SavedAppearanceTheme | null) {
    const payload = theme ?? {
      id: crypto.randomUUID(),
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
        saveSavedThemes(savedThemes)
        importStatus = `Imported ${imported.length} theme${imported.length === 1 ? '' : 's'}`
      } catch {
        importStatus = 'Could not import theme file'
      } finally {
        if (themeImportInput) themeImportInput.value = ''
      }
    })
    reader.readAsText(file)
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
      id: crypto.randomUUID(),
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
      id: typeof item.id === 'string' ? item.id : crypto.randomUUID(),
      name: typeof item.name === 'string' && item.name.trim() ? item.name.trim() : 'Imported theme',
      tokens: normalizeTokens(item.tokens),
      createdAt: typeof item.createdAt === 'string' ? item.createdAt : new Date().toISOString(),
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
        on:click={() => applyThemePreset(lightTokens, 'light')}
      >
        Light
      </button>
      <button
        class:active={activeThemeMode() === 'dark'}
        aria-pressed={activeThemeMode() === 'dark'}
        on:click={() => applyThemePreset(defaultTokens, 'dark')}
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
      <input value={themeName} on:input={(event) => themeName = event.currentTarget.value} />
    </label>

    <div class="settings-actions-inline">
      <button class="icon-label-button" on:click={saveCurrentTheme}>
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

    {#if savedThemes.length > 0}
      <div class="saved-theme-list">
        {#each savedThemes as theme}
          <div class="saved-theme-row">
            <button class="saved-theme-apply" on:click={() => applySavedTheme(theme)}>
              <span>{theme.name}</span>
              <small>{new Date(theme.createdAt).toLocaleDateString()}</small>
            </button>
            <button aria-label={`Export ${theme.name}`} title="Export theme" on:click={() => exportTheme(theme)}>
              <Icon name="download" size={16} />
            </button>
            <button aria-label={`Delete ${theme.name}`} title="Delete theme" on:click={() => removeSavedTheme(theme.id)}>
              <Icon name="delete" size={16} />
            </button>
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
      <span>Background</span>
      <input type="color" value={tokens.colorBackground} on:input={(event) => patch({ colorBackground: event.currentTarget.value })} />
      <input value={tokens.colorBackground} on:change={(event) => patch({ colorBackground: event.currentTarget.value })} />
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
    <label class="settings-field">
      <span>Panel background base</span>
      <input value={tokens.colorSurface} on:change={(event) => patch({ colorSurface: event.currentTarget.value })} />
    </label>
    <label class="settings-field color-field">
      <span>Notes editor background</span>
      <input type="color" value={tokens.colorToolBackground} on:input={(event) => patch({ colorToolBackground: event.currentTarget.value })} />
      <input value={tokens.colorToolBackground} on:change={(event) => patch({ colorToolBackground: event.currentTarget.value })} />
    </label>
    <label class="settings-field color-field">
      <span>Action bar background</span>
      <input type="color" value={tokens.colorActionBarBackground} on:input={(event) => patch({ colorActionBarBackground: event.currentTarget.value })} />
      <input value={tokens.colorActionBarBackground} on:change={(event) => patch({ colorActionBarBackground: event.currentTarget.value })} />
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
