import { afterEach, describe, expect, it, vi } from 'vitest'
import { canSilentRefresh, client, queryClient, useAuth } from './client'

afterEach(() => { vi.unstubAllGlobals(); useAuth.getState().logout(); queryClient.clear() })
function session(token: string) { useAuth.getState().setAuth({ token, userId: '11876b59-559b-4cb7-8bdf-fd156c7ff72d', email: 'human@example.test', displayName: 'Human' }) }
const error = (status: number) => new Response(JSON.stringify({ error: { code: 'denied', message: 'denied' } }), { status })
describe('session ownership at the actual Base transport', () => {
  it('a late 401 from the previous session does not log out the next session', async () => {
    let reply!: (value: Response) => void
    vi.stubGlobal('fetch', vi.fn(() => new Promise<Response>(resolve => { reply = resolve })))
    session('first')
    const request = client.get('/api/v1/namespaces').catch(e => e)
    session('second'); reply(error(401)); await request
    expect(useAuth.getState().token).toBe('second')
    expect(canSilentRefresh()).toBe(true)
  })
  it('current revocation clears protected cached data; forbidden keeps the session', async () => {
    session('current')
    vi.stubGlobal('fetch', vi.fn(async () => error(403)))
    await expect(client.get('/api/v1/audit')).rejects.toMatchObject({ status: 403 })
    expect(useAuth.getState().token).toBe('current')
    queryClient.setQueryData(['audit'], ['private'])
    vi.stubGlobal('fetch', vi.fn(async () => error(401)))
    await expect(client.get('/api/v1/audit')).rejects.toMatchObject({ status: 401 })
    expect(useAuth.getState().token).toBeNull()
    expect(canSilentRefresh()).toBe(false)
    expect(queryClient.getQueryData(['audit'])).toBeUndefined()
  })
  it('tokens are absent from persisted profile and query keys', () => {
    session('private-bearer')
    expect(localStorage.getItem('aihub.profile')).not.toContain('private-bearer')
    expect(JSON.stringify(queryClient.getQueryCache().getAll().map(q => q.queryKey))).not.toContain('private-bearer')
  })
})
