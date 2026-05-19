<script lang="ts">
  import type { DesignTokens } from '@og-suite/contracts'
  import NotesApp from '@og-suite/notes'
  import AppearanceSettings from '@og-suite/notes/AppearanceSettings'
  import { notesManifest } from '@og-suite/notes/manifest'
  import type { RuntimeServices } from '@og-suite/runtime'
  import { registerApp } from '@og-suite/runtime'
  import Icon from '@og-suite/ui/Icon'
  import { applyTokens, saveStoredTokens } from '@og-suite/ui'

  export let services: RuntimeServices

  const apps = [
    {
      ...registerApp(notesManifest),
      component: NotesApp,
    },
  ]

  let activeAppId = apps[0].manifest.id
  let settingsOpen = false
  let tokens = services.tokens
  $: activeApp = apps.find((app) => app.manifest.id === activeAppId) ?? apps[0]
  $: suiteNavItems = [
    ...apps.map((app) => ({ id: app.manifest.id, name: app.manifest.name })),
    { id: 'audio', name: 'Audio', disabled: true },
    { id: 'dump-catalog', name: 'Dump Catalog', disabled: true },
  ]

  $: applyTokens(tokens)

  function updateTokens(nextTokens: DesignTokens) {
    tokens = nextTokens
    services.tokens = nextTokens
    saveStoredTokens(nextTokens)
    applyTokens(nextTokens)
  }

  function selectApp(appId: string) {
    activeAppId = appId
  }
</script>

<main class="suite-shell">
  <header class="topbar">
    <nav class="nav-links" aria-label="Primary">
      {#each apps as app}
        <button class:active={activeAppId === app.manifest.id} class="nav-link" on:click={() => selectApp(app.manifest.id)}>
          <span>{app.manifest.name}</span>
        </button>
      {/each}
      <button class="nav-link" disabled><Icon name="microphone" size={18} /><span>Audio</span></button>
      <button class="nav-link" disabled><Icon name="add-list" size={18} /><span>Dump Catalog</span></button>
    </nav>

    <div class="topbar-actions">
      <button
        class:active={settingsOpen}
        class="icon-button"
        aria-label="Open settings"
        title="Settings"
        on:click={() => settingsOpen = true}
      >
        <Icon name="settings" size={20} />
      </button>
    </div>
  </header>

  <section class="suite-content">
    <svelte:component
      this={activeApp.component}
      {services}
      mode="suite"
      {suiteNavItems}
      activeSuiteAppId={activeAppId}
      onSuiteAppSelect={selectApp}
      onOpenSuiteSettings={() => settingsOpen = true}
    />
  </section>

  {#if settingsOpen}
    <AppearanceSettings {tokens} onTokensChange={updateTokens} onClose={() => settingsOpen = false} />
  {/if}
</main>
