<script lang="ts">
  import type { AuthSession, CurrentSession } from '@og-suite/contracts'
  import { createHttpApiClient } from '@og-suite/runtime'
  import type { RuntimeServices } from '@og-suite/runtime'
  import AudioApp from './AudioApp.svelte'
  import { createStandaloneRuntime } from './runtime'

  const defaultServerUrl = 'http://127.0.0.1:8080'

  let serverUrl = localStorage.getItem('og-suite:server-url') ?? defaultServerUrl
  let username = ''
  let password = ''
  let error = ''
  let checking = true
  let session: CurrentSession | null = null
  let services: RuntimeServices = createStandaloneRuntime(serverUrl)

  void loadSession()

  async function loadSession() {
    checking = true
    services = createStandaloneRuntime(serverUrl)
    try {
      session = await services.api.get<CurrentSession>('/api/v1/auth/session')
    } catch {
      session = null
    } finally {
      checking = false
    }
  }

  async function signIn() {
    error = ''
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
      serverUrl = normalizedServerUrl
      password = ''
      services = createStandaloneRuntime(normalizedServerUrl)
      session = {
        user: authSession.user,
        workspace: authSession.workspace,
        expiresAt: authSession.expiresAt,
      }
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
        <p class="eyebrow">OG Audio</p>
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
    </form>
  </main>
{:else}
  <AudioApp {services} mode="standalone" />
{/if}
