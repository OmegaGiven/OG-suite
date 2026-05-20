<script lang="ts">
  import type { AudioRecording, CrdtDocumentState, Note } from '@og-suite/contracts'
  import type { RuntimeServices } from '@og-suite/runtime'
  import ActionBar from '@og-suite/ui/ActionBar'
  import ActionButton from '@og-suite/ui/ActionButton'
  import FileNavigator from '@og-suite/ui/FileNavigator'
  import type { FileNavigatorFolder, FileNavigatorItem } from '@og-suite/ui/FileNavigator'
  import Icon from '@og-suite/ui/Icon'
  import MobileSuiteMenu from '@og-suite/ui/MobileSuiteMenu'
  import { onMount } from 'svelte'

  export let services: RuntimeServices
  export let mode: 'suite' | 'standalone' = 'suite'
  export let suiteNavItems: SuiteNavItem[] = []
  export let activeSuiteAppId = ''
  export let onSuiteAppSelect: ((appId: string) => void) | undefined = undefined
  export let onOpenSuiteSettings: (() => void) | undefined = undefined

  type SuiteNavItem = {
    id: string
    name: string
    disabled?: boolean
  }

  type DriveFile = {
    id: string
    name: string
    path: string
    mimeType: string
    sizeBytes: number
    createdAt: string
    updatedAt: string
  }

  type DriveEntry = DriveFile & {
    source: 'drive' | 'note' | 'audio'
    sourceId: string
    canManage: boolean
    canDownload: boolean
  }

  type DriveFolder = {
    id: string
    path: string
    name: string
    createdAt: string
  }

  type TypeBucket = {
    label: string
    count: number
    sizeBytes: number
    share: number
  }

  const metadataKey = 'og-suite:files:metadata'
  const dbName = 'og-suite-files'
  const storeName = 'file-blobs'

  let files: DriveFile[] = []
  let folders: DriveFolder[] = []
  let notes: Note[] = []
  let audioRecordings: AudioRecording[] = []
  let selectedFileId = ''
  let activeFolderPath = '/'
  let selectedFolderPath = ''
  let collapsedFolderPaths: string[] = []
  let searchOpen = false
  let searchQuery = ''
  let statusMessage = ''
  let error = ''
  let storageEstimate: StorageEstimate | null = null
  let uploadInputElement: HTMLInputElement | null = null

  $: driveEntries = files.map((file): DriveEntry => ({
    ...file,
    source: 'drive',
    sourceId: file.id,
    canManage: true,
    canDownload: true,
  }))
  $: noteEntries = notes.map((note): DriveEntry => ({
    id: `note:${note.id}`,
    sourceId: note.id,
    source: 'note',
    canManage: true,
    canDownload: true,
    name: note.title.endsWith('.md') ? note.title : `${note.title}.md`,
    path: linkedPath('/Notes', note.path),
    mimeType: 'text/markdown',
    sizeBytes: note.title.length,
    createdAt: note.createdAt,
    updatedAt: note.updatedAt,
  }))
  $: audioEntries = audioRecordings.map((recording): DriveEntry => ({
    id: `audio:${recording.id}`,
    sourceId: recording.id,
    source: 'audio',
    canManage: true,
    canDownload: Boolean(recording.assetRef),
    name: recording.title,
    path: '/Audio',
    mimeType: recording.mimeType,
    sizeBytes: recording.sizeBytes,
    createdAt: recording.createdAt,
    updatedAt: recording.updatedAt,
  }))
  $: allFiles = [...driveEntries, ...noteEntries, ...audioEntries]
  $: selectedFile = allFiles.find((file) => file.id === selectedFileId) ?? allFiles[0] ?? null
  $: filteredFiles = allFiles.filter((file) => {
    const query = searchQuery.trim().toLowerCase()
    if (!query) return true
    return [file.name, file.path, file.mimeType, typeLabel(file.mimeType), sourceLabel(file.source)].some((value) => value.toLowerCase().includes(query))
  })
  $: navigatorItems = filteredFiles.map((file): FileNavigatorItem => ({
    id: file.id,
    title: file.name,
    path: file.path,
    meta: `${sourceLabel(file.source)} · ${typeLabel(file.mimeType)} · ${formatBytes(file.sizeBytes)}`,
  }))
  $: virtualFolders = buildVirtualFolders(allFiles)
  $: navigatorFolders = [...folders.map((folder): FileNavigatorFolder => ({
    id: folder.id,
    path: folder.path,
    name: folder.name,
  })), ...virtualFolders]
  $: totalBytes = allFiles.reduce((sum, file) => sum + file.sizeBytes, 0)
  $: localBytes = files.reduce((sum, file) => sum + file.sizeBytes, 0)
  $: fileTypeBuckets = buildTypeBuckets(allFiles, totalBytes)
  $: largestFiles = [...allFiles].sort((left, right) => right.sizeBytes - left.sizeBytes).slice(0, 5)
  $: currentFolderFiles = allFiles.filter((file) => normalizeFolderPath(file.path) === activeFolderPath)
  $: browserUsedBytes = storageEstimate?.usage ?? totalBytes
  $: browserQuotaBytes = storageEstimate?.quota ?? 0
  $: browserUsagePercent = browserQuotaBytes ? Math.min(100, Math.round((browserUsedBytes / browserQuotaBytes) * 100)) : 0

  onMount(() => {
    void services.clientId
    loadMetadata()
    void refreshLinkedFiles()
    void refreshStorageEstimate()
  })

  function selectSuiteApp(appId: string) {
    onSuiteAppSelect?.(appId)
  }

  function loadMetadata() {
    const raw = localStorage.getItem(metadataKey)
    if (!raw) return
    try {
      const payload = JSON.parse(raw) as { files?: DriveFile[]; folders?: DriveFolder[]; selectedFileId?: string }
      files = payload.files ?? []
      folders = payload.folders ?? []
      selectedFileId = payload.selectedFileId ?? files[0]?.id ?? ''
    } catch {
      error = 'Files metadata could not be loaded.'
    }
  }

  function saveMetadata() {
    localStorage.setItem(metadataKey, JSON.stringify({ files, folders, selectedFileId }))
  }

  async function refreshLinkedFiles() {
    try {
      const [nextNotes, nextAudioRecordings] = await Promise.all([
        services.api.get<Note[]>('/api/v1/notes'),
        services.api.get<AudioRecording[]>('/api/v1/audio/recordings'),
      ])
      notes = nextNotes
      audioRecordings = nextAudioRecordings
      error = ''
    } catch (loadError) {
      error = loadError instanceof Error ? loadError.message : 'Notes and audio files failed to load'
    }
  }

  async function createFolder() {
    if (isLinkedSystemPath(activeFolderPath)) {
      statusMessage = 'Create folders from Notes or Audio for linked files.'
      return
    }
    const name = window.prompt('Folder name')?.trim()
    if (!name) return
    const path = normalizeFolderPath(`${activeFolderPath}/${name}`)
    if (folders.some((folder) => normalizeFolderPath(folder.path) === path)) {
      error = 'That folder already exists.'
      return
    }
    const folder = {
      id: crypto.randomUUID(),
      path,
      name: folderName(path),
      createdAt: new Date().toISOString(),
    }
    folders = [...folders, folder].sort((left, right) => left.path.localeCompare(right.path))
    activeFolderPath = path
    selectedFolderPath = path
    statusMessage = `Created folder ${path}.`
    error = ''
    saveMetadata()
  }

  function selectFolder(path: string) {
    activeFolderPath = normalizeFolderPath(path)
    selectedFolderPath = activeFolderPath === '/' ? '' : activeFolderPath
  }

  function selectFile(id: string) {
    selectedFileId = id
    selectedFolderPath = ''
    saveMetadata()
  }

  async function uploadFiles(event: Event) {
    const uploadFiles = Array.from((event.currentTarget as HTMLInputElement).files ?? [])
    if (uploadInputElement) uploadInputElement.value = ''
    if (!uploadFiles.length) return
    error = ''
    const db = await openDb()
    const now = new Date().toISOString()
    const incoming: DriveFile[] = []
    for (const file of uploadFiles) {
      const id = crypto.randomUUID()
      await idbRequest(db.transaction(storeName, 'readwrite').objectStore(storeName).put(file, id))
      incoming.push({
        id,
        name: file.name,
        path: activeFolderPath,
        mimeType: file.type || inferMimeType(file.name),
        sizeBytes: file.size,
        createdAt: now,
        updatedAt: now,
      })
    }
    db.close()
    files = [...incoming, ...files]
    selectedFileId = incoming[0]?.id ?? selectedFileId
    statusMessage = `Uploaded ${incoming.length} file${incoming.length === 1 ? '' : 's'}.`
    saveMetadata()
    await refreshStorageEstimate()
  }

  async function renameSelectedFile() {
    if (!selectedFile) return
    const name = window.prompt('File name', selectedFile.name)?.trim()
    if (!name) return
    if (selectedFile.source === 'note') {
      const title = name.replace(/\.md$/i, '')
      await services.api.patch<Note>(`/api/v1/notes/${selectedFile.sourceId}/metadata`, { title })
      await refreshLinkedFiles()
    } else if (selectedFile.source === 'audio') {
      await services.api.patch<AudioRecording>(`/api/v1/audio/recordings/${selectedFile.sourceId}`, { title: name })
      await refreshLinkedFiles()
    } else {
      files = files.map((file) => file.id === selectedFile.id ? { ...file, name, updatedAt: new Date().toISOString() } : file)
      saveMetadata()
    }
    statusMessage = 'File renamed.'
  }

  async function deleteSelectedFile() {
    if (!selectedFile) return
    if (!window.confirm(`Delete "${selectedFile.name}"?`)) return
    if (selectedFile.source === 'note') {
      await services.api.delete(`/api/v1/notes/${selectedFile.sourceId}`)
      await refreshLinkedFiles()
    } else if (selectedFile.source === 'audio') {
      await services.api.delete(`/api/v1/audio/recordings/${selectedFile.sourceId}`)
      await refreshLinkedFiles()
    } else {
      const deletedId = selectedFile.id
      const db = await openDb()
      await idbRequest(db.transaction(storeName, 'readwrite').objectStore(storeName).delete(deletedId))
      db.close()
      files = files.filter((file) => file.id !== deletedId)
      saveMetadata()
      await refreshStorageEstimate()
    }
    selectedFileId = allFiles.find((file) => file.id !== selectedFile.id)?.id ?? ''
    statusMessage = 'File deleted.'
    saveMetadata()
  }

  async function downloadSelectedFile() {
    if (!selectedFile) return
    if (selectedFile.source === 'note') {
      const note = notes.find((item) => item.id === selectedFile.sourceId)
      if (!note) return
      const document = await services.api.get<CrdtDocumentState>(`/api/v1/documents/${note.documentId}`)
      downloadBlob(new Blob([document.snapshot], { type: 'text/markdown' }), selectedFile.name)
      return
    }
    if (selectedFile.source === 'audio') {
      window.location.href = backendUrl(`/api/v1/audio/recordings/${selectedFile.sourceId}/audio`)
      return
    }
    const blob = await getBlob(selectedFile.sourceId)
    if (!blob) {
      error = 'File data is missing from local storage.'
      return
    }
    downloadBlob(blob, selectedFile.name)
  }

  async function moveFile(fileId: string, path: string) {
    const normalized = normalizeFolderPath(path)
    const file = allFiles.find((item) => item.id === fileId)
    if (!file) return
    if (file.source === 'note') {
      if (!isSameOrNestedPath(normalized, '/Notes')) return
      await services.api.patch<Note>(`/api/v1/notes/${file.sourceId}/metadata`, { path: stripLinkedPath('/Notes', normalized) })
      await refreshLinkedFiles()
    } else if (file.source === 'audio') {
      if (!isSameOrNestedPath(normalized, '/Audio')) return
      await services.api.patch<AudioRecording>(`/api/v1/audio/recordings/${file.sourceId}`, { path: '/' })
      await refreshLinkedFiles()
    } else {
      if (isLinkedSystemPath(normalized)) return
      files = files.map((item) => item.id === file.sourceId ? { ...item, path: normalized, updatedAt: new Date().toISOString() } : item)
      saveMetadata()
    }
    activeFolderPath = normalized
  }

  async function moveFolder(sourcePath: string, targetPath: string) {
    const source = normalizeFolderPath(sourcePath)
    const target = normalizeFolderPath(targetPath)
    if (source === '/' || source === target || target.startsWith(`${source}/`)) return
    if (isLinkedSystemPath(source) || isLinkedSystemPath(target)) return
    const nextPath = normalizeFolderPath(`${target}/${folderName(source)}`)
    if (source === nextPath) return
    folders = folders.map((folder) => {
      const path = normalizeFolderPath(folder.path)
      return isSameOrNestedPath(path, source) ? { ...folder, path: remapPath(path, source, nextPath), name: folderName(remapPath(path, source, nextPath)) } : folder
    })
    files = files.map((file) => {
      const path = normalizeFolderPath(file.path)
      return isSameOrNestedPath(path, source) ? { ...file, path: remapPath(path, source, nextPath), updatedAt: new Date().toISOString() } : file
    })
    activeFolderPath = nextPath
    selectedFolderPath = nextPath
    saveMetadata()
  }

  function toggleFolder(path: string) {
    const normalized = normalizeFolderPath(path)
    collapsedFolderPaths = collapsedFolderPaths.includes(normalized)
      ? collapsedFolderPaths.filter((item) => item !== normalized)
      : [...collapsedFolderPaths, normalized]
  }

  async function clearMissingFile() {
    if (!selectedFile) return
    files = files.filter((file) => file.id !== selectedFile.sourceId)
    selectedFileId = allFiles[0]?.id ?? ''
    saveMetadata()
  }

  async function refreshStorageEstimate() {
    storageEstimate = navigator.storage?.estimate ? await navigator.storage.estimate() : null
  }

  async function openDb() {
    return new Promise<IDBDatabase>((resolve, reject) => {
      const request = indexedDB.open(dbName, 1)
      request.onupgradeneeded = () => {
        request.result.createObjectStore(storeName)
      }
      request.onsuccess = () => resolve(request.result)
      request.onerror = () => reject(request.error)
    })
  }

  async function getBlob(id: string) {
    const db = await openDb()
    const blob = await idbRequest<Blob | undefined>(db.transaction(storeName, 'readonly').objectStore(storeName).get(id))
    db.close()
    return blob
  }

  function idbRequest<T = unknown>(request: IDBRequest<T>) {
    return new Promise<T>((resolve, reject) => {
      request.onsuccess = () => resolve(request.result)
      request.onerror = () => reject(request.error)
    })
  }

  function downloadBlob(blob: Blob, name: string) {
    const url = URL.createObjectURL(blob)
    const link = document.createElement('a')
    link.href = url
    link.download = name
    link.click()
    URL.revokeObjectURL(url)
  }

  function buildTypeBuckets(sourceFiles: DriveEntry[], total: number): TypeBucket[] {
    const buckets = new Map<string, { count: number; sizeBytes: number }>()
    for (const file of sourceFiles) {
      const label = typeLabel(file.mimeType)
      const bucket = buckets.get(label) ?? { count: 0, sizeBytes: 0 }
      bucket.count += 1
      bucket.sizeBytes += file.sizeBytes
      buckets.set(label, bucket)
    }
    return Array.from(buckets.entries())
      .map(([label, bucket]) => ({
        label,
        count: bucket.count,
        sizeBytes: bucket.sizeBytes,
        share: total ? Math.round((bucket.sizeBytes / total) * 100) : 0,
      }))
      .sort((left, right) => right.sizeBytes - left.sizeBytes)
  }

  function typeLabel(mimeType: string) {
    if (!mimeType) return 'Other'
    const [family, subtype] = mimeType.split('/')
    if (family === 'image') return 'Images'
    if (family === 'video') return 'Video'
    if (family === 'audio') return 'Audio'
    if (family === 'text') return 'Text'
    if (mimeType.includes('pdf')) return 'PDF'
    if (mimeType.includes('spreadsheet') || subtype?.includes('excel')) return 'Spreadsheets'
    if (mimeType.includes('document') || subtype?.includes('word')) return 'Documents'
    if (mimeType.includes('zip') || mimeType.includes('compressed')) return 'Archives'
    return 'Other'
  }

  function sourceLabel(source: DriveEntry['source']) {
    if (source === 'note') return 'Note'
    if (source === 'audio') return 'Audio'
    return 'Drive'
  }

  function linkedPath(root: '/Notes' | '/Audio', path: string) {
    const normalized = normalizeFolderPath(path)
    return normalized === '/' ? root : normalizeFolderPath(`${root}${normalized}`)
  }

  function stripLinkedPath(root: '/Notes' | '/Audio', path: string) {
    const normalized = normalizeFolderPath(path)
    if (normalized === root) return '/'
    return normalizeFolderPath(normalized.slice(root.length))
  }

  function isLinkedSystemPath(path: string) {
    const normalized = normalizeFolderPath(path)
    return normalized === '/Notes' || normalized.startsWith('/Notes/') || normalized === '/Audio' || normalized.startsWith('/Audio/')
  }

  function buildVirtualFolders(sourceFiles: DriveEntry[]): FileNavigatorFolder[] {
    const paths = new Set<string>()
    for (const root of ['/Notes', '/Audio'] as const) paths.add(root)
    for (const file of sourceFiles) {
      const parts = normalizeFolderPath(file.path).split('/').filter(Boolean)
      let current = ''
      for (const part of parts) {
        current = normalizeFolderPath(`${current}/${part}`)
        if (current !== '/') paths.add(current)
      }
    }
    return Array.from(paths)
      .filter((path) => !folders.some((folder) => normalizeFolderPath(folder.path) === path))
      .map((path) => ({
        id: `virtual:${path}`,
        path,
        name: folderName(path),
      }))
      .sort((left, right) => left.path.localeCompare(right.path))
  }

  function backendUrl(path: string) {
    const apiHost = typeof window === 'undefined' || window.location.hostname === 'localhost' ? '127.0.0.1' : window.location.hostname
    const defaultApiUrl = typeof window === 'undefined' ? 'http://127.0.0.1:8080' : `http://${apiHost}:8080`
    return `${import.meta.env.VITE_OG_API_URL ?? defaultApiUrl}${path}`
  }

  function inferMimeType(name: string) {
    const extension = name.split('.').at(-1)?.toLowerCase()
    if (extension === 'pdf') return 'application/pdf'
    if (['png', 'jpg', 'jpeg', 'gif', 'webp'].includes(extension ?? '')) return `image/${extension === 'jpg' ? 'jpeg' : extension}`
    if (['mp3', 'wav', 'ogg'].includes(extension ?? '')) return `audio/${extension}`
    if (['mp4', 'webm', 'mov'].includes(extension ?? '')) return `video/${extension}`
    if (['txt', 'md', 'csv', 'json'].includes(extension ?? '')) return 'text/plain'
    return 'application/octet-stream'
  }

  function normalizeFolderPath(path: string) {
    const trimmed = path.trim()
    if (!trimmed || trimmed === '/') return '/'
    return `/${trimmed.replace(/^\/+|\/+$/g, '')}`
  }

  function folderName(path: string) {
    const normalized = normalizeFolderPath(path)
    return normalized === '/' ? 'Root' : normalized.split('/').filter(Boolean).at(-1) ?? normalized
  }

  function isSameOrNestedPath(path: string, parent: string) {
    const normalized = normalizeFolderPath(path)
    const normalizedParent = normalizeFolderPath(parent)
    return normalized === normalizedParent || normalized.startsWith(`${normalizedParent}/`)
  }

  function remapPath(path: string, source: string, target: string) {
    const normalized = normalizeFolderPath(path)
    const normalizedSource = normalizeFolderPath(source)
    const normalizedTarget = normalizeFolderPath(target)
    if (normalized === normalizedSource) return normalizedTarget
    return normalizeFolderPath(`${normalizedTarget}/${normalized.slice(normalizedSource.length)}`)
  }

  function formatBytes(bytes: number) {
    if (!bytes) return '0 B'
    const units = ['B', 'KB', 'MB', 'GB', 'TB']
    const exponent = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1)
    const value = bytes / 1024 ** exponent
    return `${value >= 10 || exponent === 0 ? value.toFixed(0) : value.toFixed(1)} ${units[exponent]}`
  }

  function formatDate(value: string) {
    return new Intl.DateTimeFormat(undefined, { month: 'short', day: 'numeric', year: 'numeric' }).format(new Date(value))
  }
</script>

<section class="files-app">
  <div class="files-library">
    <ActionBar ariaLabel="File actions" className="files-action-bar">
      <ActionButton icon="search" label="Search" iconOnly on:click={() => searchOpen = !searchOpen} />
      <ActionButton icon="new-folder" label="New folder" iconOnly on:click={createFolder} />
      <ActionButton icon="upload" label="Upload files" iconOnly on:click={() => uploadInputElement?.click()} />
      <ActionButton icon="rename" label="Rename selected file" iconOnly disabled={!selectedFile?.canManage} on:click={renameSelectedFile} />
      <ActionButton icon="download" label="Download selected file" iconOnly disabled={!selectedFile?.canDownload} on:click={downloadSelectedFile} />
      <ActionButton icon="delete" label="Delete selected file" iconOnly tone="danger" disabled={!selectedFile?.canManage} on:click={deleteSelectedFile} />
      <ActionButton icon="refresh" label="Refresh files" iconOnly on:click={() => { void refreshLinkedFiles(); void refreshStorageEstimate() }} />
      {#if mode === 'suite'}
        <MobileSuiteMenu
          title="Files"
          navItems={suiteNavItems}
          activeAppId={activeSuiteAppId}
          onSelectApp={selectSuiteApp}
          onOpenSettings={onOpenSuiteSettings}
        >
          <button on:click={() => searchOpen = !searchOpen}>
            <Icon name="search" size={16} />
            <span>Search</span>
          </button>
          <button on:click={createFolder}>
            <Icon name="new-folder" size={16} />
            <span>New folder</span>
          </button>
          <button on:click={() => uploadInputElement?.click()}>
            <Icon name="upload" size={16} />
            <span>Upload</span>
          </button>
        </MobileSuiteMenu>
      {/if}
    </ActionBar>

    <input bind:this={uploadInputElement} class="file-upload-input" type="file" multiple on:change={uploadFiles} />

    {#if searchOpen}
      <label class="files-search">
        <Icon name="search" size={16} />
        <input bind:value={searchQuery} type="search" placeholder="Search files" aria-label="Search files" />
      </label>
    {/if}

    <FileNavigator
      folders={navigatorFolders}
      items={navigatorItems}
      selectedItemId={selectedFileId}
      {activeFolderPath}
      {selectedFolderPath}
      {collapsedFolderPaths}
      itemLabel="file"
      onSelectItem={selectFile}
      onSelectFolder={selectFolder}
      onMoveItem={moveFile}
      onMoveFolder={moveFolder}
      onToggleFolder={toggleFolder}
    />
  </div>

  <div class="files-main">
    <section class="storage-panel">
      <div class="panel-heading">
        <div>
          <h2>Drive Storage</h2>
          <span>{allFiles.length} files · {navigatorFolders.length} folders</span>
        </div>
        <strong>{formatBytes(totalBytes)}</strong>
      </div>
      <div class="storage-meter" aria-label="Browser storage usage">
        <span style={`width: ${browserUsagePercent}%`}></span>
      </div>
      <div class="storage-meta">
        <span>{formatBytes(localBytes)} uploaded locally · {formatBytes(browserUsedBytes)} browser storage used</span>
        <span>{browserQuotaBytes ? `${formatBytes(browserQuotaBytes)} available in browser quota` : 'Browser quota unavailable'}</span>
      </div>
      <div class="type-grid">
        {#each fileTypeBuckets as bucket}
          <article class="type-card">
            <div>
              <strong>{bucket.label}</strong>
              <span>{bucket.count} file{bucket.count === 1 ? '' : 's'}</span>
            </div>
            <span>{formatBytes(bucket.sizeBytes)} · {bucket.share}%</span>
          </article>
        {/each}
        {#if fileTypeBuckets.length === 0}
          <p class="empty-copy">Upload files to see storage by type.</p>
        {/if}
      </div>
    </section>

    <section class="detail-panel">
      <div class="panel-heading">
        <div>
          <h2>{selectedFile?.name ?? 'No file selected'}</h2>
          <span>{activeFolderPath} · {currentFolderFiles.length} item{currentFolderFiles.length === 1 ? '' : 's'}</span>
        </div>
      </div>
      {#if selectedFile}
        <dl class="file-details">
          <div>
            <dt>Type</dt>
            <dd>{selectedFile.mimeType}</dd>
          </div>
          <div>
            <dt>Source</dt>
            <dd>{sourceLabel(selectedFile.source)}</dd>
          </div>
          <div>
            <dt>Category</dt>
            <dd>{typeLabel(selectedFile.mimeType)}</dd>
          </div>
          <div>
            <dt>Size</dt>
            <dd>{formatBytes(selectedFile.sizeBytes)}</dd>
          </div>
          <div>
            <dt>Folder</dt>
            <dd>{selectedFile.path}</dd>
          </div>
          <div>
            <dt>Added</dt>
            <dd>{formatDate(selectedFile.createdAt)}</dd>
          </div>
          <div>
            <dt>Updated</dt>
            <dd>{formatDate(selectedFile.updatedAt)}</dd>
          </div>
        </dl>
      {:else}
        <p class="empty-copy">Select or upload a file to inspect it.</p>
      {/if}
      {#if error === 'File data is missing from local storage.'}
        <ActionButton label="Remove missing file entry" tone="danger" on:click={clearMissingFile} />
      {/if}
      {#if statusMessage}
        <p class="status-copy">{statusMessage}</p>
      {/if}
      {#if error}
        <p class="error-copy">{error}</p>
      {/if}
    </section>

    <section class="largest-panel">
      <div class="panel-heading">
        <div>
          <h2>Largest Files</h2>
          <span>Quick cleanup targets</span>
        </div>
      </div>
      {#if largestFiles.length}
        <div class="largest-list">
          {#each largestFiles as file}
            <button class:active={selectedFileId === file.id} on:click={() => selectFile(file.id)}>
              <span>{file.name}</span>
              <small>{formatBytes(file.sizeBytes)} · {typeLabel(file.mimeType)}</small>
            </button>
          {/each}
        </div>
      {:else}
        <p class="empty-copy">No files yet.</p>
      {/if}
    </section>
  </div>
</section>
