import { describe, expect, it } from 'vitest'
import type { CrdtUpdate } from '@og-suite/contracts'
import { applyUpdates, createDocumentState, createTextDiffUpdate, createTextReplacementUpdate } from './index'

function updateFromClient(
  documentId: string,
  clientId: string,
  sequence: number,
  text: string,
  base = createDocumentState(documentId, 'note', ''),
): CrdtUpdate {
  return {
    ...createTextReplacementUpdate(documentId, clientId, sequence, text, base),
    id: `${clientId}-${sequence}`,
    createdAt: new Date(sequence).toISOString(),
  }
}

describe('Yjs document CRDT', () => {
  it('merges concurrent inserts from two clients without conflict copies', () => {
    const documentId = 'doc-1'
    const base = createDocumentState(documentId, 'note', '')
    const first = updateFromClient(documentId, 'client-a', 1, 'Alpha', base)
    const second = updateFromClient(documentId, 'client-b', 1, 'Beta', base)

    const merged = applyUpdates({
      ...base,
      updates: [first, second],
      version: 2,
    })

    expect(merged.text).toContain('Alpha')
    expect(merged.text).toContain('Beta')
    expect(merged.text).toHaveLength('AlphaBeta'.length)
  })

  it('preserves edits made against the same prior snapshot', () => {
    const documentId = 'doc-2'
    const base = createDocumentState(documentId, 'note', 'Start')
    const first = updateFromClient(documentId, 'client-a', 1, 'Start A', base)
    const second = updateFromClient(documentId, 'client-b', 1, 'Start B', base)

    const merged = applyUpdates({
      ...base,
      updates: [first, second],
      version: 2,
    })

    expect(merged.text).toContain('Start')
    expect(merged.text).toContain('A')
    expect(merged.text).toContain('B')
  })

  it('maps a local text diff onto a document that already has remote changes', () => {
    const documentId = 'doc-3'
    const base = createDocumentState(documentId, 'note', 'Seed\n')
    const remote = updateFromClient(documentId, 'client-b', 1, 'Seed\nRemote\n', base)
    const localBase = {
      ...base,
      updates: [remote],
      version: 1,
    }
    const local = {
      ...createTextDiffUpdate(documentId, 'client-a', 1, 'Seed\n', 'Seed\nLocal\n', localBase),
      id: 'client-a-1',
      createdAt: new Date(1).toISOString(),
    }

    const merged = applyUpdates({
      ...base,
      updates: [remote, local],
      version: 2,
    })

    expect(merged.text).toContain('Seed')
    expect(merged.text).toContain('Remote')
    expect(merged.text).toContain('Local')
  })
})
