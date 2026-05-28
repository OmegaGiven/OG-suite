<script lang="ts">
  import type {
    AdminSummary,
    AdminUserSummary,
    AppToolScope,
    CreateAdminRoleRequest,
    CreateAdminUserRequest,
    UpdateAdminUserAccessRequest,
  } from '@og-suite/contracts'
  import type { RuntimeServices } from '@og-suite/runtime'
  import MobileSuiteMenu from '@og-suite/ui/MobileSuiteMenu'
  import { onMount } from 'svelte'

  export let services: RuntimeServices
  export let mode: 'standalone' | 'suite' = 'suite'
  export let suiteNavItems: { id: string; name: string }[] = []
  export let activeSuiteAppId = 'admin'
  export let onSuiteAppSelect: (appId: string) => void = () => {}
  export let onOpenSuiteSettings: (() => void) | undefined

  const tabs = ['Users', 'Roles', 'Storage', 'Authentication', 'Deployment', 'Database', 'Audits']
  const scopeKeys: Array<keyof AppToolScope> = ['feed', 'notes', 'files', 'audio', 'admin']
  const defaultMemberScopes: AppToolScope = {
    feed: true,
    notes: true,
    files: true,
    audio: true,
    admin: false,
  }

  let summary: AdminSummary | null = null
  let activeTab = tabs[0]
  let loading = true
  let error = ''
  let createUserOpen = false
  let createRoleOpen = false
  let createUsername = ''
  let createDisplayName = ''
  let createPassword = ''
  let createRoles = ['owner']
  let createStorageLimitMb = 2048
  let createScopes: AppToolScope = { ...defaultMemberScopes }
  let createRoleName = ''
  let createRoleScopes: AppToolScope = { ...defaultMemberScopes }
  let createRoleAdminPanel = false
  let createRoleManageUsers = false
  let createRoleManageStorage = false
  let createRoleManageAuth = false
  let createRoleManageDeployment = false
  let createRoleManageDatabase = false
  let createRoleViewAudits = false
  let selectedDatabaseTableKey = ''
  let databaseSearch = ''
  $: roleOptions = summary?.rolePolicies.map((role) => role.name) ?? ['owner', 'admin']
  $: selectedDatabaseTable = summary?.database.tables.find((table) => table.key === selectedDatabaseTableKey)
    ?? summary?.database.tables[0]
  $: if (summary && !selectedDatabaseTableKey && summary.database.tables[0]) {
    selectedDatabaseTableKey = summary.database.tables[0].key
  }
  $: filteredDatabaseRows = selectedDatabaseTable
    ? selectedDatabaseTable.rows.filter((row) => JSON.stringify(row).toLowerCase().includes(databaseSearch.trim().toLowerCase()))
    : []
  onMount(() => {
    void loadSummary()
  })

  async function loadSummary() {
    loading = true
    error = ''
    try {
      summary = await services.api.get<AdminSummary>('/api/v1/admin/summary')
    } catch (requestError) {
      error = requestError instanceof Error ? requestError.message : 'Could not load admin settings.'
    } finally {
      loading = false
    }
  }

  function formatBytes(bytes: number) {
    if (bytes < 1024) return `${bytes} B`
    const units = ['KB', 'MB', 'GB', 'TB']
    let value = bytes / 1024
    let unit = units[0]
    for (const next of units.slice(1)) {
      if (value < 1024) break
      value /= 1024
      unit = next
    }
    return `${value.toFixed(value >= 10 ? 0 : 1)} ${unit}`
  }

  function upsertUser(user: AdminUserSummary) {
    if (!summary) return
    summary = {
      ...summary,
      users: summary.users.some((current) => current.id === user.id)
        ? summary.users.map((current) => current.id === user.id ? user : current)
        : [...summary.users, user],
    }
  }

  function normalizeSelectedRoles(roles: string[]) {
    const normalized = roles.map((role) => role.trim().toLowerCase()).filter(Boolean)
    return normalized.length ? [...new Set(normalized)] : ['owner']
  }

  function toggleCreateRole(role: string) {
    createRoles = createRoles.includes(role)
      ? normalizeSelectedRoles(createRoles.filter((current) => current !== role))
      : normalizeSelectedRoles([...createRoles, role])
  }

  function toggleCreateRoleScope(scope: keyof AppToolScope) {
    createRoleScopes = {
      ...createRoleScopes,
      [scope]: !createRoleScopes[scope],
    }
  }

  function toggleUserRole(user: AdminUserSummary, role: string) {
    user.roles = user.roles.includes(role)
      ? normalizeSelectedRoles(user.roles.filter((current) => current !== role))
      : normalizeSelectedRoles([...user.roles, role])
  }

  async function createUser() {
    error = ''
    const payload: CreateAdminUserRequest = {
      username: createUsername.trim(),
      displayName: createDisplayName.trim(),
      password: createPassword,
      roles: normalizeSelectedRoles(createRoles),
      storageLimitMb: createStorageLimitMb,
      appScopes: createScopes,
    }
    if (!payload.username || !payload.displayName || !payload.password) {
      error = 'Username, display name, and password are required.'
      return
    }
    try {
      const user = await services.api.post<AdminUserSummary>('/api/v1/admin/users', payload)
      upsertUser(user)
      createUsername = ''
      createDisplayName = ''
      createPassword = ''
      createRoles = ['owner']
      createStorageLimitMb = 2048
      createScopes = { ...defaultMemberScopes }
      createUserOpen = false
    } catch (requestError) {
      error = requestError instanceof Error ? requestError.message : 'Could not create user.'
    }
  }

  async function createRole() {
    error = ''
    const payload: CreateAdminRoleRequest = {
      name: createRoleName.trim().toLowerCase(),
      appScopes: createRoleScopes,
      adminPanel: createRoleAdminPanel,
      manageUsers: createRoleManageUsers,
      manageStorage: createRoleManageStorage,
      manageAuth: createRoleManageAuth,
      manageDeployment: createRoleManageDeployment,
      manageDatabase: createRoleManageDatabase,
      viewAudits: createRoleViewAudits,
    }
    if (!payload.name) {
      error = 'Role name is required.'
      return
    }
    try {
      const role = await services.api.post<CreateAdminRoleRequest>('/api/v1/admin/roles', payload)
      if (summary) {
        summary = {
          ...summary,
          rolePolicies: [...summary.rolePolicies, role],
        }
      }
      createRoleName = ''
      createRoleScopes = { ...defaultMemberScopes }
      createRoleAdminPanel = false
      createRoleManageUsers = false
      createRoleManageStorage = false
      createRoleManageAuth = false
      createRoleManageDeployment = false
      createRoleManageDatabase = false
      createRoleViewAudits = false
      createRoleOpen = false
    } catch (requestError) {
      error = requestError instanceof Error ? requestError.message : 'Could not create role.'
    }
  }

  async function updateUserAccess(user: AdminUserSummary) {
    error = ''
    const payload: UpdateAdminUserAccessRequest = {
      roles: user.roles,
      storageLimitMb: user.storageLimitMb,
      appScopes: user.appScopes,
    }
    try {
      const updated = await services.api.patch<AdminUserSummary>(`/api/v1/admin/users/${user.id}/access`, payload)
      upsertUser(updated)
    } catch (requestError) {
      error = requestError instanceof Error ? requestError.message : 'Could not update user access.'
    }
  }

  async function resetPassword(user: AdminUserSummary) {
    error = ''
    const password = window.prompt(`New password for ${user.displayName}`)
    if (!password) return
    const confirmPassword = window.prompt('Confirm new password')
    if (password !== confirmPassword) {
      error = 'Passwords do not match.'
      return
    }
    try {
      const updated = await services.api.post<AdminUserSummary>(`/api/v1/admin/users/${user.id}/reset-password`, {
        password,
        confirmPassword,
      })
      upsertUser(updated)
    } catch (requestError) {
      error = requestError instanceof Error ? requestError.message : 'Could not reset password.'
    }
  }

  function toggleUserScope(user: AdminUserSummary, scope: keyof AppToolScope) {
    user.appScopes = {
      ...user.appScopes,
      [scope]: !user.appScopes[scope],
    }
  }
</script>

<main class="admin-app" data-mode={mode}>
  <section class="admin-panel">
    <div class="admin-heading">
      <div>
        <p class="eyebrow">Server Control</p>
        <h1>Admin settings</h1>
      </div>
      <div class="admin-heading-actions">
        <button type="button" class="admin-refresh" on:click={loadSummary}>Refresh</button>
        {#if mode === 'suite'}
          <MobileSuiteMenu
            title="Admin"
            navItems={suiteNavItems}
            activeAppId={activeSuiteAppId}
            onSelectApp={(appId: string) => {
              onSuiteAppSelect(appId)
            }}
            onOpenSettings={onOpenSuiteSettings}
          >
            <button on:click={loadSummary}>Refresh</button>
          </MobileSuiteMenu>
        {/if}
      </div>
    </div>

    {#if loading}
      <p class="admin-status">Loading admin settings...</p>
    {:else if error}
      <p class="admin-status admin-error">{error}</p>
    {:else if summary}
      <nav class="admin-tabs" aria-label="Admin settings sections">
        {#each tabs as tab}
          <button type="button" class:active={activeTab === tab} on:click={() => activeTab = tab}>
            {tab}
          </button>
        {/each}
      </nav>

      {#if activeTab === 'Users'}
        <div class="admin-users-header">
          <div>
            <h2>Users</h2>
            <p class="admin-muted">{summary.users.length} accounts on this server</p>
          </div>
          <button type="button" class="admin-refresh" on:click={() => createUserOpen = true}>Create user</button>
        </div>

        <div class="admin-table-shell">
          <table class="admin-users-table">
            <thead>
              <tr>
                <th>User</th>
                <th>Roles</th>
                <th>App scopes</th>
                <th>Setup</th>
                <th>Storage used</th>
                <th>Storage limit</th>
                <th>Created</th>
                <th>Updated</th>
                <th>Actions</th>
              </tr>
            </thead>
            <tbody>
              {#each summary.users as user}
                <tr>
                  <td>
                    <strong>{user.displayName}</strong>
                    <span>{user.username}</span>
                  </td>
                  <td>
                    <div class="selector-row">
                      {#each roleOptions as role}
                        <label>
                          <input type="checkbox" checked={user.roles.includes(role)} on:change={() => toggleUserRole(user, role)} />
                          <span>{role}</span>
                        </label>
                      {/each}
                    </div>
                  </td>
                  <td>
                    <div class="selector-row">
                      {#each scopeKeys as scope}
                        <label>
                          <input type="checkbox" checked={user.appScopes[scope]} on:change={() => toggleUserScope(user, scope)} />
                          <span>{scope}</span>
                        </label>
                      {/each}
                    </div>
                  </td>
                  <td>{user.mustChangePassword ? 'Required' : 'Complete'}</td>
                  <td>{formatBytes(user.storageUsedBytes)}</td>
                  <td>
                    <label class="table-number-field">
                      <input bind:value={user.storageLimitMb} type="number" min="0" />
                      <span>MB</span>
                    </label>
                  </td>
                  <td>{new Date(user.createdAt).toLocaleDateString()}</td>
                  <td>{new Date(user.updatedAt).toLocaleDateString()}</td>
                  <td>
                    <div class="table-actions">
                      <button type="button" class="admin-refresh" on:click={() => updateUserAccess(user)}>Save</button>
                      <button type="button" class="admin-refresh" on:click={() => resetPassword(user)}>Reset password</button>
                    </div>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>

        {#if createUserOpen}
          <button class="admin-modal-backdrop" aria-label="Close create user" on:click={() => createUserOpen = false}></button>
          <form class="admin-modal" aria-label="Create user" on:submit|preventDefault={createUser}>
            <div class="admin-card-title">
              <div>
                <h2>Create user</h2>
                <span>Temporary password requires setup on next login</span>
              </div>
              <button type="button" class="admin-modal-close" aria-label="Close create user" on:click={() => createUserOpen = false}>x</button>
            </div>
            <div class="admin-form-grid">
              <label>
                <span>Username</span>
                <input bind:value={createUsername} autocomplete="off" />
              </label>
              <label>
                <span>Display name</span>
                <input bind:value={createDisplayName} autocomplete="off" />
              </label>
              <label>
                <span>Password</span>
                <input bind:value={createPassword} type="password" autocomplete="new-password" />
              </label>
              <label>
                <span>Storage MB</span>
                <input bind:value={createStorageLimitMb} type="number" min="0" />
              </label>
            </div>
            <div>
              <p class="field-heading">Roles</p>
              <div class="selector-row">
                {#each roleOptions as role}
                  <label>
                    <input type="checkbox" checked={createRoles.includes(role)} on:change={() => toggleCreateRole(role)} />
                    <span>{role}</span>
                  </label>
                {/each}
              </div>
            </div>
            <div>
              <p class="field-heading">App scopes</p>
              <div class="selector-row">
                {#each scopeKeys as scope}
                  <label>
                    <input
                      type="checkbox"
                      checked={createScopes[scope]}
                      on:change={() => createScopes = { ...createScopes, [scope]: !createScopes[scope] }}
                    />
                    <span>{scope}</span>
                  </label>
                {/each}
              </div>
            </div>
            <div class="admin-card-actions">
              <button type="submit" class="admin-refresh">Create user</button>
              <button type="button" class="admin-refresh" on:click={() => createUserOpen = false}>Cancel</button>
            </div>
          </form>
        {/if}
      {:else if activeTab === 'Roles'}
        <div class="admin-users-header">
          <div>
            <h2>Roles</h2>
            <p class="admin-muted">{summary.rolePolicies.length} role policies available for user assignment</p>
          </div>
          <button type="button" class="admin-refresh" on:click={() => createRoleOpen = true}>Create role</button>
        </div>
        <div class="admin-grid">
          {#each summary.rolePolicies as role}
            <article class="admin-card">
              <div class="admin-card-title">
                <h2>{role.name}</h2>
                <span>{role.adminPanel ? 'Admin panel' : 'App access'}</span>
              </div>
              <div class="scope-row">
                {#each Object.entries(role.appScopes) as [scope, enabled]}
                  <span class:enabled>{scope}</span>
                {/each}
              </div>
              <dl>
                <div><dt>Users</dt><dd>{role.manageUsers ? 'Manage' : 'No access'}</dd></div>
                <div><dt>Storage</dt><dd>{role.manageStorage ? 'Manage' : 'View only'}</dd></div>
                <div><dt>Audits</dt><dd>{role.viewAudits ? 'Visible' : 'Hidden'}</dd></div>
              </dl>
            </article>
          {/each}
        </div>
        {#if createRoleOpen}
          <button class="admin-modal-backdrop" aria-label="Close create role" on:click={() => createRoleOpen = false}></button>
          <form class="admin-modal" aria-label="Create role" on:submit|preventDefault={createRole}>
            <div class="admin-card-title">
              <div>
                <h2>Create role</h2>
                <span>New roles become available in user role selectors.</span>
              </div>
              <button type="button" class="admin-modal-close" aria-label="Close create role" on:click={() => createRoleOpen = false}>x</button>
            </div>
            <div class="admin-form-grid">
              <label>
                <span>Role name</span>
                <input bind:value={createRoleName} autocomplete="off" />
              </label>
            </div>
            <div>
              <p class="field-heading">App scopes</p>
              <div class="selector-row">
                {#each scopeKeys as scope}
                  <label>
                    <input type="checkbox" checked={createRoleScopes[scope]} on:change={() => toggleCreateRoleScope(scope)} />
                    <span>{scope}</span>
                  </label>
                {/each}
              </div>
            </div>
            <div>
              <p class="field-heading">Admin permissions</p>
              <div class="selector-row">
                <label><input type="checkbox" bind:checked={createRoleAdminPanel} /><span>admin panel</span></label>
                <label><input type="checkbox" bind:checked={createRoleManageUsers} /><span>users</span></label>
                <label><input type="checkbox" bind:checked={createRoleManageStorage} /><span>storage</span></label>
                <label><input type="checkbox" bind:checked={createRoleManageAuth} /><span>auth</span></label>
                <label><input type="checkbox" bind:checked={createRoleManageDeployment} /><span>deployment</span></label>
                <label><input type="checkbox" bind:checked={createRoleManageDatabase} /><span>database</span></label>
                <label><input type="checkbox" bind:checked={createRoleViewAudits} /><span>audits</span></label>
              </div>
            </div>
            <div class="admin-card-actions">
              <button type="submit" class="admin-refresh">Create role</button>
              <button type="button" class="admin-refresh" on:click={() => createRoleOpen = false}>Cancel</button>
            </div>
          </form>
        {/if}
      {:else if activeTab === 'Storage'}
        <div class="admin-card admin-wide-card">
          <h2>Storage limits</h2>
          <dl>
            <div><dt>Total used</dt><dd>{formatBytes(summary.storage.totalUsedBytes)}</dd></div>
            <div><dt>Total limit</dt><dd>{summary.storage.totalLimitMb} MB</dd></div>
            <div><dt>Users</dt><dd>{summary.storage.userCount}</dd></div>
            <div><dt>Notes</dt><dd>{formatBytes(summary.storage.notesBytes)}</dd></div>
            <div><dt>Audio</dt><dd>{formatBytes(summary.storage.audioBytes)}</dd></div>
            <div><dt>Files</dt><dd>{formatBytes(summary.storage.filesBytes)}</dd></div>
          </dl>
        </div>
      {:else if activeTab === 'Authentication'}
        <div class="admin-card admin-wide-card">
          <h2>Authentication</h2>
          <dl>
            <div><dt>Default admin</dt><dd>{summary.authentication.defaultAdminEnabled ? 'Enabled' : 'Disabled'}</dd></div>
            <div><dt>Local passwords</dt><dd>{summary.authentication.localPasswordEnabled ? 'Enabled' : 'Disabled'}</dd></div>
            <div><dt>Forced setup change</dt><dd>{summary.authentication.requireSetupPasswordChange ? 'Required' : 'Optional'}</dd></div>
          </dl>
        </div>
      {:else if activeTab === 'Deployment'}
        <div class="admin-card admin-wide-card">
          <h2>Deployment</h2>
          <dl>
            <div><dt>Version</dt><dd>{summary.deployment.serverVersion}</dd></div>
            <div><dt>Build date</dt><dd>{summary.deployment.buildDate}</dd></div>
            <div><dt>API compatibility</dt><dd>{summary.deployment.apiCompatibilityVersion}</dd></div>
            <div><dt>Release channel</dt><dd>{summary.deployment.releaseChannel}</dd></div>
          </dl>
        </div>
      {:else if activeTab === 'Database'}
        <div class="admin-database-controls">
          <label>
            <span>Table</span>
            <select bind:value={selectedDatabaseTableKey}>
              {#each summary.database.tables as table}
                <option value={table.key}>{table.label}</option>
              {/each}
            </select>
          </label>
          <label>
            <span>Search</span>
            <input bind:value={databaseSearch} placeholder="Search selected table" />
          </label>
          <button type="button" class="admin-refresh">Search</button>
        </div>
        {#if selectedDatabaseTable}
          <div class="admin-users-header">
            <div>
              <h2>{selectedDatabaseTable.label}</h2>
              <p class="admin-muted">{filteredDatabaseRows.length} of {selectedDatabaseTable.rowCount} rows</p>
            </div>
          </div>
          <div class="admin-table-shell">
            <table class="admin-users-table admin-database-table">
              <thead>
                <tr>
                  {#each selectedDatabaseTable.columns as column}
                    <th>{column}</th>
                  {/each}
                </tr>
              </thead>
              <tbody>
                {#each filteredDatabaseRows as row}
                  <tr>
                    {#each selectedDatabaseTable.columns as column}
                      <td>{String(row[column] ?? '')}</td>
                    {/each}
                  </tr>
                {:else}
                  <tr>
                    <td colspan={selectedDatabaseTable.columns.length}>No rows match this search.</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {/if}
      {:else}
        <div class="admin-list">
          {#each summary.audits as audit}
            <article class="admin-card">
              <div class="admin-card-title">
                <h2>{audit.action}</h2>
                <span>{new Date(audit.occurredAt).toLocaleString()}</span>
              </div>
              <p class="admin-muted">{audit.actorLabel} -> {audit.targetKind}: {audit.targetLabel}</p>
            </article>
          {/each}
        </div>
      {/if}
    {/if}
  </section>
</main>

<style>
  .admin-app {
    min-height: calc(100vh - 51px);
    padding: var(--page-gutter, 16px);
    color: var(--text, var(--og-text));
  }

  .admin-panel {
    display: grid;
    gap: var(--space-md, 12px);
    width: 100%;
    max-width: 1180px;
    margin: 0 auto;
    padding: var(--panel-pad, 14px);
    border: 1px solid var(--border, var(--og-border));
    border-radius: var(--panel-radius, var(--og-radius));
    background: var(--panel-surface, var(--og-surface));
    background-image: var(--panel-texture, var(--og-panel-texture, none));
    background-blend-mode: soft-light;
    box-shadow: var(--shadow, var(--og-shadow));
    backdrop-filter: blur(16px);
    box-sizing: border-box;
  }

  .admin-heading,
  .admin-card-title {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-md, 12px);
  }

  .admin-heading-actions {
    display: flex;
    align-items: center;
    gap: var(--space-xs, 6px);
  }

  .admin-card-actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-xs, 6px);
  }

  .admin-heading h1,
  .admin-card h2,
  .admin-heading p,
  .admin-card p {
    margin: 0;
  }

  .admin-heading h1 {
    font-size: 24px;
    line-height: 1.1;
  }

  .admin-refresh,
  .admin-tabs button {
    min-height: 34px;
    border: 1px solid var(--border, var(--og-border));
    border-radius: var(--field-radius, var(--og-field-radius));
    background: var(--surface-subtle, var(--og-surface-subtle));
    color: var(--text, var(--og-text));
    cursor: pointer;
    font-weight: 800;
  }

  .admin-refresh {
    padding: 0 12px;
  }

  .admin-tabs {
    display: flex;
    gap: var(--space-xs, 6px);
    overflow-x: auto;
    scrollbar-width: none;
  }

  .admin-tabs::-webkit-scrollbar {
    display: none;
  }

  .admin-tabs button {
    flex: 0 0 auto;
    padding: 0 10px;
  }

  .admin-tabs button.active {
    border-color: color-mix(in srgb, var(--accent, var(--og-accent)) 42%, transparent);
    background: color-mix(in srgb, var(--accent-soft, var(--og-accent-soft)) 72%, transparent);
  }

  .admin-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
    gap: var(--space-md, 12px);
  }

  .admin-users-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-md, 12px);
  }

  .admin-users-header h2,
  .admin-users-header p {
    margin: 0;
  }

  .admin-table-shell {
    overflow-x: auto;
    border: 1px solid var(--border, var(--og-border));
    border-radius: var(--panel-radius, var(--og-radius));
    background: var(--panel-surface, var(--og-surface));
    background-image: var(--panel-texture, var(--og-panel-texture, none));
    background-blend-mode: soft-light;
    box-shadow: var(--shadow, var(--og-shadow));
    backdrop-filter: blur(16px);
  }

  .admin-users-table {
    width: 100%;
    min-width: 1080px;
    border-collapse: collapse;
  }

  .admin-database-table {
    min-width: 720px;
  }

  .admin-users-table th,
  .admin-users-table td {
    padding: var(--space-sm, 8px);
    border-bottom: 1px solid color-mix(in srgb, var(--border, var(--og-border)) 72%, transparent);
    text-align: left;
    vertical-align: top;
  }

  .admin-users-table th {
    color: var(--muted, var(--og-muted));
    font-size: 11px;
    font-weight: 900;
    text-transform: uppercase;
  }

  .admin-users-table tbody tr:last-child td {
    border-bottom: 0;
  }

  .admin-users-table td:first-child {
    display: grid;
    gap: 3px;
    min-width: 150px;
  }

  .admin-users-table td:first-child span {
    color: var(--muted, var(--og-muted));
    font-size: 12px;
  }

  .admin-database-controls {
    display: flex;
    align-items: end;
    flex-wrap: wrap;
    gap: var(--space-sm, 8px);
    padding: var(--panel-pad, 14px);
    border: 1px solid var(--border, var(--og-border));
    border-radius: var(--panel-radius, var(--og-radius));
    background: var(--panel-surface, var(--og-surface));
    background-image: var(--panel-texture, var(--og-panel-texture, none));
    background-blend-mode: soft-light;
    box-shadow: var(--shadow, var(--og-shadow));
    backdrop-filter: blur(16px);
  }

  .admin-database-controls label {
    display: grid;
    gap: 5px;
    min-width: 180px;
    color: var(--muted, var(--og-muted));
    font-size: 12px;
    font-weight: 800;
  }

  .admin-database-controls input,
  .admin-database-controls select {
    min-height: 34px;
    border: 1px solid var(--border, var(--og-border));
    border-radius: var(--field-radius, var(--og-field-radius));
    background: var(--surface-subtle, var(--og-surface-subtle));
    color: var(--text, var(--og-text));
    padding: 6px 8px;
  }

  .selector-row {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .selector-row label {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    min-height: 28px;
    padding: 0 8px;
    border-radius: var(--field-radius, var(--og-field-radius));
    background: var(--surface-subtle, var(--og-surface-subtle));
    color: var(--text, var(--og-text));
    font-size: 11px;
    font-weight: 800;
    white-space: nowrap;
  }

  .selector-row input {
    margin: 0;
  }

  .table-number-field {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }

  .table-number-field input {
    width: 76px;
    min-height: 30px;
    border: 1px solid var(--border, var(--og-border));
    border-radius: var(--field-radius, var(--og-field-radius));
    background: var(--surface-subtle, var(--og-surface-subtle));
    color: var(--text, var(--og-text));
    padding: 5px 7px;
  }

  .table-number-field span {
    color: var(--muted, var(--og-muted));
    font-size: 12px;
    font-weight: 800;
  }

  .table-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    min-width: 160px;
  }

  .admin-modal-backdrop {
    position: fixed;
    inset: 0;
    z-index: 2147483300;
    border: 0;
    border-radius: 0;
    background: color-mix(in srgb, var(--bg, var(--og-bg)) 54%, transparent);
    backdrop-filter: blur(8px);
    cursor: default;
  }

  .admin-modal {
    position: fixed;
    top: 50%;
    left: 50%;
    z-index: 2147483301;
    display: grid;
    gap: var(--space-md, 12px);
    width: min(620px, calc(100vw - 28px));
    max-height: calc(100vh - 28px);
    overflow: auto;
    padding: var(--panel-pad, 14px);
    border: 1px solid var(--border, var(--og-border));
    border-radius: var(--panel-radius, var(--og-radius));
    background: var(--panel-surface, var(--og-surface));
    background-image: var(--panel-texture, var(--og-panel-texture, none));
    background-blend-mode: soft-light;
    box-shadow: var(--shadow, var(--og-shadow));
    transform: translate(-50%, -50%);
  }

  .admin-modal-close {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border: 1px solid var(--border, var(--og-border));
    border-radius: var(--field-radius, var(--og-field-radius));
    background: var(--surface-subtle, var(--og-surface-subtle));
    color: var(--text, var(--og-text));
    cursor: pointer;
    font-weight: 900;
  }

  .field-heading {
    margin: 0 0 6px;
    color: var(--muted, var(--og-muted));
    font-size: 12px;
    font-weight: 900;
  }

  .admin-form-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    gap: var(--space-sm, 8px);
  }

  .admin-form-grid label {
    display: grid;
    gap: 5px;
    color: var(--muted, var(--og-muted));
    font-size: 12px;
    font-weight: 800;
  }

  .admin-form-grid input {
    min-height: 32px;
    border: 1px solid var(--border, var(--og-border));
    border-radius: var(--field-radius, var(--og-field-radius));
    background: var(--surface-subtle, var(--og-surface-subtle));
    color: var(--text, var(--og-text));
    padding: 6px 8px;
  }

  .admin-list {
    display: grid;
    gap: var(--space-sm, 8px);
  }

  .admin-card {
    display: grid;
    gap: var(--space-sm, 8px);
    padding: var(--panel-pad, 14px);
    border: 1px solid var(--border, var(--og-border));
    border-radius: var(--panel-radius, var(--og-radius));
    background: var(--panel-surface, var(--og-surface));
    background-image: var(--panel-texture, var(--og-panel-texture, none));
    background-blend-mode: soft-light;
    box-shadow: var(--shadow, var(--og-shadow));
    backdrop-filter: blur(16px);
  }

  .admin-wide-card {
    max-width: 720px;
  }

  .admin-card h2 {
    font-size: 16px;
  }

  .admin-card-title span,
  .admin-muted,
  .admin-status {
    color: var(--muted, var(--og-muted));
    font-size: 12px;
  }

  .admin-error {
    color: var(--danger, var(--og-danger));
  }

  .admin-card dl {
    display: grid;
    gap: 7px;
    margin: 0;
  }

  .admin-card dl div {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--space-md, 12px);
  }

  .admin-card dt {
    color: var(--muted, var(--og-muted));
    font-size: 12px;
    font-weight: 800;
  }

  .admin-card dd {
    margin: 0;
    text-align: right;
  }

  .scope-row {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .scope-row span {
    padding: 3px 7px;
    border-radius: var(--field-radius, var(--og-field-radius));
    background: color-mix(in srgb, var(--surface-subtle, var(--og-surface-subtle)) 82%, transparent);
    color: color-mix(in srgb, var(--muted, var(--og-muted)) 78%, transparent);
    font-size: 11px;
    font-weight: 800;
  }

  .scope-row span.enabled {
    background: color-mix(in srgb, var(--accent-soft, var(--og-accent-soft)) 72%, transparent);
    color: var(--text, var(--og-text));
  }

  @media (max-width: 760px) {
    .admin-app {
      min-height: 100vh;
      padding: var(--page-gutter, 16px);
    }

    .admin-panel {
      padding: var(--panel-pad, 14px);
    }

    .admin-heading {
      align-items: flex-start;
    }

    .admin-heading-actions > .admin-refresh {
      display: none;
    }
  }
</style>
