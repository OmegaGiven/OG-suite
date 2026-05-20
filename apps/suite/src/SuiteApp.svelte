<script lang="ts">
  import type { DesignTokens } from '@og-suite/contracts'
  import AudioApp from '@og-suite/audio'
  import { audioManifest } from '@og-suite/audio/manifest'
  import FeedApp from '@og-suite/feed'
  import { feedManifest } from '@og-suite/feed/manifest'
  import FilesApp from '@og-suite/files'
  import { filesManifest } from '@og-suite/files/manifest'
  import NotesApp from '@og-suite/notes'
  import AppearanceSettings from '@og-suite/notes/AppearanceSettings'
  import { notesManifest } from '@og-suite/notes/manifest'
  import type { RuntimeServices } from '@og-suite/runtime'
  import { registerApp } from '@og-suite/runtime'
  import Icon from '@og-suite/ui/Icon'
  import { applyTokens, saveStoredTokens } from '@og-suite/ui'

  export let services: RuntimeServices

  type SuiteOpenTarget = {
    appId: string
    targetKind: string
    targetId: string
    targetLabel: string
    requestId: number
  }

  const apps = [
    {
      ...registerApp(feedManifest),
      component: FeedApp,
    },
    {
      ...registerApp(notesManifest),
      component: NotesApp,
    },
    {
      ...registerApp(filesManifest),
      component: FilesApp,
    },
    {
      ...registerApp(audioManifest),
      component: AudioApp,
    },
  ]

  let activeAppId = apps[0].manifest.id
  let settingsOpen = false
  let tokens = services.tokens
  let openTarget: SuiteOpenTarget | null = null
  $: activeApp = apps.find((app) => app.manifest.id === activeAppId) ?? apps[0]
  $: suiteNavItems = [
    ...apps.map((app) => ({ id: app.manifest.id, name: app.manifest.name })),
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

  function openActivityTarget(target: SuiteOpenTarget) {
    if (apps.some((app) => app.manifest.id === target.appId)) {
      activeAppId = target.appId
    }
    openTarget = { ...target, requestId: Date.now() }
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
      {openTarget}
      onSuiteAppSelect={selectApp}
      onOpenSuiteSettings={() => settingsOpen = true}
      onOpenActivityTarget={openActivityTarget}
    />
  </section>

  {#if settingsOpen}
    <AppearanceSettings {tokens} onTokensChange={updateTokens} onClose={() => settingsOpen = false} />
  {/if}
</main>
