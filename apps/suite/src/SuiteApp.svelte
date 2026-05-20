<script lang="ts">
  import type { AuthSession, CurrentSession, DesignTokens } from '@og-suite/contracts'
  import AdminApp from '@og-suite/admin'
  import { adminManifest } from '@og-suite/admin/manifest'
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
  import { onMount } from 'svelte'

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
  const adminApp = {
    ...registerApp(adminManifest),
    component: AdminApp,
  }

  let activeAppId = apps[0].manifest.id
  let settingsOpen = false
  let profileMenuOpen = false
  let tokens = services.tokens
  let openTarget: SuiteOpenTarget | null = null
  let session: CurrentSession | null = null
  let authMode: 'login' | 'register' = 'login'
  let authUsername = ''
  let authDisplayName = ''
  let authPassword = ''
  let setupConfirmPassword = ''
  let authError = ''
  let checkingSession = true
  $: isAdmin = session?.user.roles.includes('admin') ?? false
  $: visibleApps = isAdmin ? [...apps, adminApp] : apps
  $: activeApp = visibleApps.find((app) => app.manifest.id === activeAppId) ?? visibleApps[0]
  $: suiteNavItems = [
    ...visibleApps.map((app) => ({ id: app.manifest.id, name: app.manifest.name })),
  ]

  $: applyTokens(tokens)

  onMount(() => {
    void loadSession()
  })

  function updateTokens(nextTokens: DesignTokens) {
    tokens = nextTokens
    services.tokens = nextTokens
    saveStoredTokens(nextTokens)
    applyTokens(nextTokens)
  }

  function selectApp(appId: string) {
    if (!visibleApps.some((app) => app.manifest.id === appId)) return
    activeAppId = appId
  }

  function openActivityTarget(target: SuiteOpenTarget) {
    if (visibleApps.some((app) => app.manifest.id === target.appId)) {
      activeAppId = target.appId
    }
    openTarget = { ...target, requestId: Date.now() }
  }

  async function loadSession() {
    checkingSession = true
    authError = ''
    try {
      session = await services.api.get<CurrentSession>('/api/v1/auth/session')
    } catch {
      session = null
    } finally {
      checkingSession = false
    }
  }

  async function submitAuth() {
    authError = ''
    const username = authUsername.trim()
    const password = authPassword
    if (!username || !password) {
      authError = 'Username and password are required.'
      return
    }
    try {
      const authSession = authMode === 'register'
        ? await services.api.post<AuthSession>('/api/v1/auth/register', {
            username,
            displayName: authDisplayName.trim() || username,
            password,
          })
        : await services.api.post<AuthSession>('/api/v1/auth/login', { username, password })
      saveAuthSession(authSession)
      session = {
        user: authSession.user,
        workspace: authSession.workspace,
        expiresAt: authSession.expiresAt,
      }
      authPassword = ''
    } catch (error) {
      authError = error instanceof Error ? error.message : 'Sign in failed.'
    }
  }

  async function completeSetup() {
    authError = ''
    const username = authUsername.trim()
    const displayName = authDisplayName.trim() || username
    if (!username || !displayName || !authPassword || !setupConfirmPassword) {
      authError = 'Username, display name, password, and confirmation are required.'
      return
    }
    if (authPassword !== setupConfirmPassword) {
      authError = 'Passwords do not match.'
      return
    }
    try {
      session = await services.api.post<CurrentSession>('/api/v1/auth/complete-setup', {
        username,
        displayName,
        password: authPassword,
        confirmPassword: setupConfirmPassword,
      })
      authPassword = ''
      setupConfirmPassword = ''
    } catch (error) {
      authError = error instanceof Error ? error.message : 'Setup failed.'
    }
  }

  async function logout() {
    try {
      await services.api.post('/api/v1/auth/logout', {})
    } catch {
      // Local token cleanup matters even if the server session is already gone.
    }
    localStorage.removeItem('og-suite:auth:access-token')
    localStorage.removeItem('og-suite:auth:refresh-token')
    localStorage.removeItem('og-suite:auth:expires-at')
    profileMenuOpen = false
    session = null
  }

  function saveAuthSession(authSession: AuthSession) {
    localStorage.setItem('og-suite:auth:access-token', authSession.accessToken)
    localStorage.setItem('og-suite:auth:refresh-token', authSession.refreshToken)
    localStorage.setItem('og-suite:auth:expires-at', authSession.expiresAt)
  }
</script>

<main class="suite-shell">
  {#if checkingSession}
    <section class="auth-screen">
      <div class="auth-card">
        <p>Checking session...</p>
      </div>
    </section>
  {:else if !session}
    <section class="auth-screen">
      <form class="auth-card" on:submit|preventDefault={submitAuth}>
        <div>
          <p class="eyebrow">OG Suite Server</p>
          <h1>{authMode === 'register' ? 'Create profile' : 'Sign in'}</h1>
        </div>
        <label>
          <span>Username</span>
          <input bind:value={authUsername} autocomplete="username" />
        </label>
        {#if authMode === 'register'}
          <label>
            <span>Display name</span>
            <input bind:value={authDisplayName} autocomplete="name" />
          </label>
        {/if}
        <label>
          <span>Password</span>
          <input bind:value={authPassword} type="password" autocomplete={authMode === 'register' ? 'new-password' : 'current-password'} />
        </label>
        {#if authError}
          <p class="auth-error">{authError}</p>
        {/if}
        <button class="auth-submit" type="submit">{authMode === 'register' ? 'Create profile' : 'Sign in'}</button>
        <button
          class="auth-mode-toggle"
          type="button"
          on:click={() => {
            authMode = authMode === 'register' ? 'login' : 'register'
            authError = ''
          }}
        >
          {authMode === 'register' ? 'Use an existing profile' : 'Create a new profile'}
        </button>
      </form>
    </section>
  {:else if session.user.mustChangePassword}
    <section class="auth-screen">
      <form class="auth-card" on:submit|preventDefault={completeSetup}>
        <div>
          <p class="eyebrow">First Setup</p>
          <h1>Secure the admin account</h1>
          <p class="auth-hint">Choose the username and password this server should use going forward.</p>
        </div>
        <label>
          <span>New username</span>
          <input bind:value={authUsername} autocomplete="username" placeholder={session.user.username ?? 'admin'} />
        </label>
        <label>
          <span>Display name</span>
          <input bind:value={authDisplayName} autocomplete="name" placeholder={session.user.displayName} />
        </label>
        <label>
          <span>New password</span>
          <input bind:value={authPassword} type="password" autocomplete="new-password" />
        </label>
        <label>
          <span>Confirm password</span>
          <input bind:value={setupConfirmPassword} type="password" autocomplete="new-password" />
        </label>
        {#if authError}
          <p class="auth-error">{authError}</p>
        {/if}
        <button class="auth-submit" type="submit">Finish setup</button>
        <button class="auth-mode-toggle" type="button" on:click={logout}>Sign out</button>
      </form>
    </section>
  {:else}
  <header class="topbar">
    <nav class="nav-links" aria-label="Primary">
      {#each visibleApps as app}
        <button class:active={activeAppId === app.manifest.id} class="nav-link" on:click={() => selectApp(app.manifest.id)}>
          <span>{app.manifest.name}</span>
        </button>
      {/each}
    </nav>

    <div class="topbar-actions">
      <div class="profile-menu">
        <button
          class:active={profileMenuOpen}
          class="profile-chip"
          type="button"
          title={session.workspace.name}
          aria-haspopup="menu"
          aria-expanded={profileMenuOpen}
          on:click={() => profileMenuOpen = !profileMenuOpen}
        >
          <span>{session.user.displayName}</span>
          <span class="profile-caret" aria-hidden="true"></span>
        </button>
        {#if profileMenuOpen}
          <div class="profile-menu-popover" role="menu">
            <div class="profile-menu-header">
              <strong>{session.user.displayName}</strong>
              <span>{session.workspace.name}</span>
            </div>
            <button type="button" role="menuitem" on:click={logout}>Logout</button>
          </div>
        {/if}
      </div>
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
    <AppearanceSettings {tokens} {services} onTokensChange={updateTokens} onClose={() => settingsOpen = false} />
  {/if}
  {/if}
</main>
