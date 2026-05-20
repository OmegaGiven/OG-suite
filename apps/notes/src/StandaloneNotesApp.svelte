<script lang="ts">
  import type { AuthSession, CurrentSession } from '@og-suite/contracts'
  import { createHttpApiClient } from '@og-suite/runtime'
  import type { RuntimeServices } from '@og-suite/runtime'
  import NotesApp from './NotesApp.svelte'
  import { createLocalOnlyRuntime, createStandaloneRuntime } from './runtime'

  const defaultServerUrl = 'http://127.0.0.1:8080'
  const localModeKey = 'og-suite:notes:local-only'

  let serverUrl = localStorage.getItem('og-suite:server-url') ?? defaultServerUrl
  let username = ''
  let password = ''
  let error = ''
  let backupStatus = ''
  let checking = true
  let session: CurrentSession | null = null
  let localOnly = localStorage.getItem(localModeKey) === 'true'
  let backupDialogOpen = false
  let runtimeKey = 0
  let services: RuntimeServices = localOnly ? createLocalOnlyRuntime() : createStandaloneRuntime(serverUrl)

  void loadSession()

  function withTimeout<T>(promise: Promise<T>, milliseconds: number): Promise<T> {
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => reject(new Error('Session check timed out.')), milliseconds)
      promise.then(
        (value) => {
          clearTimeout(timer)
          resolve(value)
        },
        (timeoutError) => {
          clearTimeout(timer)
          reject(timeoutError)
        },
      )
    })
  }

  function createLocalSession(): CurrentSession {
    return {
      user: {
        id: 'local-user',
        displayName: 'Local Notes',
        username: 'local',
        roles: ['owner'],
        mustChangePassword: false,
      },
      workspace: {
        id: 'local-workspace',
        name: 'Local Notes',
      },
      expiresAt: '9999-12-31T23:59:59.999Z',
    }
  }

  async function loadSession() {
    checking = true
    if (localOnly) {
      services = createLocalOnlyRuntime()
      session = createLocalSession()
      checking = false
      return
    }
    if (!localStorage.getItem('og-suite:auth:access-token')) {
      session = null
      checking = false
      return
    }
    services = createStandaloneRuntime(serverUrl)
    try {
      session = await withTimeout(services.api.get<CurrentSession>('/api/v1/auth/session'), 2500)
    } catch {
      session = null
    } finally {
      checking = false
    }
  }

  function continueLocally() {
    error = ''
    backupStatus = ''
    localOnly = true
    backupDialogOpen = false
    localStorage.setItem(localModeKey, 'true')
    services = createLocalOnlyRuntime()
    session = createLocalSession()
    runtimeKey += 1
  }

  function openBackupDialog() {
    error = ''
    backupStatus = ''
    backupDialogOpen = true
  }

  async function signIn() {
    error = ''
    backupStatus = ''
    const normalizedServerUrl = serverUrl.trim().replace(/\/$/, '')
    if (!normalizedServerUrl || !username.trim() || !password) {
      error = 'Server, username, and password are required.'
      return
    }
    try {
      const api = createHttpApiClient(normalizedServerUrl)
      const authSession = await api.post<AuthSession>('/api/v1/auth/login', {
        username: username.trim(),
        password,
      })
      if (authSession.user.mustChangePassword) {
        error = 'This account must finish first setup in OG Suite before mobile sign-in.'
        return
      }
      localStorage.setItem('og-suite:server-url', normalizedServerUrl)
      localStorage.setItem('og-suite:auth:access-token', authSession.accessToken)
      localStorage.setItem('og-suite:auth:refresh-token', authSession.refreshToken)
      localStorage.setItem('og-suite:auth:expires-at', authSession.expiresAt)
      localStorage.removeItem(localModeKey)
      serverUrl = normalizedServerUrl
      password = ''
      localOnly = false
      backupDialogOpen = false
      services = createStandaloneRuntime(normalizedServerUrl)
      session = {
        user: authSession.user,
        workspace: authSession.workspace,
        expiresAt: authSession.expiresAt,
      }
      runtimeKey += 1
      backupStatus = 'Signed in. Local notes queued on this device will back up to the server.'
      setTimeout(() => {
        backupStatus = ''
      }, 6000)
    } catch (requestError) {
      error = requestError instanceof Error ? requestError.message : 'Sign in failed.'
    }
  }
</script>

{#if checking}
  <main class="standalone-auth-screen">
    <section class="standalone-auth-card">
      <p>Checking session...</p>
    </section>
  </main>
{:else if !session}
  <main class="standalone-auth-screen">
    <form class="standalone-auth-card" on:submit|preventDefault={signIn}>
      <div>
        <p class="eyebrow">OG Notes</p>
        <h1>Sign in to a server</h1>
      </div>
      <label>
        <span>Server URL</span>
        <input bind:value={serverUrl} autocomplete="url" />
      </label>
      <label>
        <span>Username</span>
        <input bind:value={username} autocomplete="username" />
      </label>
      <label>
        <span>Password</span>
        <input bind:value={password} type="password" autocomplete="current-password" />
      </label>
      {#if error}
        <p class="standalone-auth-error">{error}</p>
      {/if}
      <button type="submit">Sign in</button>
      <button class="standalone-auth-secondary" type="button" on:click={continueLocally}>
        Continue without signing in
      </button>
    </form>
  </main>
{:else}
  <div class="standalone-notes-shell">
    {#if !localOnly && backupStatus}
      <div class="local-mode-banner synced">
        <div>
          <strong>Server backup active</strong>
          <span>{backupStatus}</span>
        </div>
      </div>
    {/if}

    {#key runtimeKey}
      <NotesApp {services} mode="standalone" onBackupToServer={localOnly ? openBackupDialog : undefined} />
    {/key}
  </div>

  {#if backupDialogOpen}
    <div class="local-backup-overlay">
      <button
        class="local-backup-backdrop"
        type="button"
        aria-label="Close server backup sign in"
        on:click={() => (backupDialogOpen = false)}
      ></button>
      <form class="standalone-auth-card local-backup-dialog" on:submit|preventDefault={signIn}>
        <div>
          <p class="eyebrow">Server backup</p>
          <h1>Sign in to back up notes</h1>
          <p class="standalone-auth-help">
            Your local notes stay on this device until you sign in. After sign-in, queued local changes upload to that server and are visible in the signed-in workspace.
          </p>
        </div>
        <label>
          <span>Server URL</span>
          <input bind:value={serverUrl} autocomplete="url" />
        </label>
        <label>
          <span>Username</span>
          <input bind:value={username} autocomplete="username" />
        </label>
        <label>
          <span>Password</span>
          <input bind:value={password} type="password" autocomplete="current-password" />
        </label>
        {#if error}
          <p class="standalone-auth-error">{error}</p>
        {/if}
        <div class="local-backup-actions">
          <button class="standalone-auth-secondary" type="button" on:click={() => (backupDialogOpen = false)}>
            Cancel
          </button>
          <button type="submit">Back up to server</button>
        </div>
      </form>
    </div>
  {/if}
{/if}
