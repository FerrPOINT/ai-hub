import { createApiClient, type ApiClient } from '@sdlc/ui/lib'
import { createAuthStore } from '@sdlc/ui/auth'
import { QueryClient } from '@tanstack/react-query'
import type { components } from './schema'

export const useAuth = createAuthStore({ storageKey: 'aihub.profile' })
export const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false, refetchOnWindowFocus: false }, mutations: { retry: false } } })
let sessionGeneration = 0
let silentRefreshAllowed = true
export function authGeneration() { return sessionGeneration }
export function canSilentRefresh() { return silentRefreshAllowed }
export function blockSilentRefresh() { silentRefreshAllowed = false }
useAuth.subscribe((next, previous) => {
  if (next.token !== previous.token) {
    if (next.token) silentRefreshAllowed = true
    sessionGeneration += 1
    queryClient.removeQueries({ predicate: query => !['public-config', 'version'].includes(String(query.queryKey[0])) })
  }
})
function sessionClient() {
  const generation = authGeneration()
  const token = useAuth.getState().token
  return createApiClient({ getAccessToken: () => token, onUnauthorized: () => {
    if (generation === authGeneration()) { blockSilentRefresh(); useAuth.getState().logout() }
  } })
}
// Each Base transport owns its captured session; late 401 cannot log out a new user.
// No automatic refresh/replay: mutations use their own durable intent.
export const client: ApiClient = {
  request: <T,>(path: string, init?: RequestInit) => sessionClient().request<T>(path, init),
  get: <T,>(path: string, signal?: AbortSignal) => sessionClient().get<T>(path, signal),
  post: <T,>(path: string, body?: unknown, signal?: AbortSignal) => sessionClient().post<T>(path, body, signal),
  put: <T,>(path: string, body?: unknown, signal?: AbortSignal) => sessionClient().put<T>(path, body, signal),
  patch: <T,>(path: string, body?: unknown, signal?: AbortSignal) => sessionClient().patch<T>(path, body, signal),
  delete: <T,>(path: string, signal?: AbortSignal) => sessionClient().delete<T>(path, signal),
  raw: (path, init) => sessionClient().raw(path, init),
}
export type Identity = components['schemas']['Identity']
export type NamespaceBinding = components['schemas']['NamespaceBinding']
export type PublicConfig = components['schemas']['PublicConfig']
export type Version = components['schemas']['Version']
export type AuditPage = components['schemas']['AuditPage']
export type NamespacePage = components['schemas']['NamespacePage']
export type PriceInput = components['schemas']['PriceInput']
export type PriceRevision = components['schemas']['PriceRevision']
export type PricePage = components['schemas']['PricePage']
export type BudgetInput = components['schemas']['BudgetInput']
export type Budget = components['schemas']['Budget']
export type BudgetPage = components['schemas']['BudgetPage']

export async function listNamespaceBindings(signal?: AbortSignal): Promise<NamespaceBinding[]> {
  const items: NamespaceBinding[] = []
  const seen = new Set<string>()
  let cursor: string | undefined
  do {
    const query = new URLSearchParams({ limit: '100', ...(cursor ? { cursor } : {}) })
    const page = await client.get<NamespacePage>(`/api/v1/namespaces?${query}`, signal)
    items.push(...page.items)
    if (items.length > 10000 || (page.next_cursor && seen.has(page.next_cursor))) throw new Error('Некорректная пагинация Namespace')
    cursor = page.next_cursor ?? undefined
    if (cursor) seen.add(cursor)
  } while (cursor)
  return items
}
