import { useEffect, useState } from 'react'
import { useInfiniteQuery, useQuery } from '@tanstack/react-query'
import { Route, Routes, useLocation, useNavigate } from 'react-router'
import { beginSso, completeSso, endSso, type SsoConfig } from '@sdlc/ui/sso'
import { PlatformProvider } from '@sdlc/ui/lib'
import { PlatformServicesProvider, AppShell, Button, Card, DashboardLayout, ErrorState, ListPage, LoadingState, NamespacePicker, ResourceStateView, resourceErrorState } from '@sdlc/ui/ui'
import { Activity, FileClock, Settings, Wallet,Plug } from 'lucide-react'
import { Budgets } from '../features/budgets/Budgets'
import {Providers} from '../features/providers/Providers'
import {ConnectionDetails} from '../features/providers/ConnectionDetails'
import {Models} from '../features/models/Models'
import {ModelDetails} from '../features/models/ModelDetails'
import { authGeneration, blockSilentRefresh, canSilentRefresh, client, useAuth, queryClient, listNamespaceBindings, type PublicConfig, type Identity, type Version, type AuditPage } from '../shared/api/client'
import { contextFromSearch, namespaceQueryKey, selectedNamespaceUrl } from '../shared/namespace'

let ssoCompletion: ReturnType<typeof completeSso> | null = null
let refreshPending: Promise<void> | null = null

function SsoCallback({ config }: { config: SsoConfig }) {
  const navigate = useNavigate()
  const [error, setError] = useState(false)
  useEffect(() => {
    if (!ssoCompletion) {
      ssoCompletion = completeSso(config)
      void ssoCompletion.finally(() => { ssoCompletion = null }).catch(() => undefined)
    }
    let active = true
    void ssoCompletion.then(session => {
      if (!active) return
      useAuth.getState().setAuth({ token: session.accessToken, userId: session.subject, email: session.email, displayName: session.name })
      navigate(session.returnTo, { replace: true })
    }).catch(() => { if (active) setError(true) })
    return () => { active = false }
  }, [config, navigate])
  // Token exchange is invisible; return directly to the requested product route.
  return error ? <Login config={config} message="Не удалось завершить вход. Повторите попытку." /> : null
}

function Login({ config, message }: { config: SsoConfig; message?: string }) {
  const location = useLocation()
  const [pending, setPending] = useState(false)
  const [error, setError] = useState(message)
  async function login() {
    setPending(true)
    try { await beginSso(config, location.pathname === '/login' || location.pathname === '/sso/callback' ? '/' : location.pathname + location.search, { interactive: true }) }
    catch { setError('Не удалось открыть вход в платформу. Повторите попытку.'); setPending(false) }
  }
  return <main className="hub-login"><Card className="hub-login-panel p-6"><h1 className="text-2xl font-semibold">Вход в платформу</h1><p className="my-4 text-text-secondary">AI Hub</p>{error && <p role="alert" className="mb-4 text-danger">{error}</p>}<Button disabled={pending} onClick={() => void login()} className="w-full min-h-11">Войти через SSO</Button></Card></main>
}

function ProtectedApp({ config }: { config: SsoConfig }) {
  const token = useAuth(s => s.token)
  const [bootstrap, setBootstrap] = useState(!token)
  const [refreshError, setRefreshError] = useState(false)
  const [logoutPending, setLogoutPending] = useState(false)
  const location = useLocation()
  const navigate = useNavigate()
  useEffect(() => {
    if (token) { setBootstrap(false); return }
    if (logoutPending || !canSilentRefresh()) { setBootstrap(false); return }
    const generation = authGeneration()
    refreshPending ??= fetch(`${config.issuer}/auth/refresh`, { method: 'POST', credentials: 'include' }).then(async response => {
      if (response.status === 401) return
      if (!response.ok) throw new Error('Auth unavailable')
      const result = await response.json() as { access_token: string }
      if (!result.access_token) throw new Error('Invalid auth reply')
      // Profile identity is resolved by the server, not from an unverified JWT.
      if (generation === authGeneration()) useAuth.setState({ token: result.access_token })
    }).finally(() => { refreshPending = null })
    let active = true
    void refreshPending.catch(() => { if (active) setRefreshError(true) }).finally(() => { if (active) setBootstrap(false) })
    return () => { active = false }
  }, [token, config.issuer, logoutPending])
  const identity = useQuery({ queryKey: ['identity', authGeneration()], queryFn: () => client.get<Identity>('/api/v1/auth/me'), enabled: !!token })
  const namespaces = useQuery({ queryKey: ['namespaces', authGeneration(), identity.data?.subject], queryFn: ({ signal }) => listNamespaceBindings(signal), enabled: !!identity.data })
  if (bootstrap || (token && identity.isPending)) return <LoadingState message="Проверка сессии…" />
  if (!token) return <Login config={config} message={refreshError ? 'Платформа временно недоступна.' : undefined} />
  if (identity.isError) return <ResourceStateView state={resourceErrorState(identity.error, () => void identity.refetch())} />
  if (!identity.data) return <ErrorState message="Не удалось подтвердить сессию" />
  const context = contextFromSearch(location.search)
  const selected = context.namespace ? `${context.namespace.registry_instance_id}/${context.namespace.namespace_id}` : ''
  function logout() {
    setLogoutPending(true); blockSilentRefresh(); queryClient.clear(); useAuth.getState().logout()
    endSso(config)
  }
  return <AppShell currentServiceKey="ai-hub" title="AI Hub" navigation={[{ to: '/', label: 'Обзор', icon: Activity }, { to: '/providers', label: 'Провайдеры', icon: Plug }, { to: '/models', label: 'Модели', icon: Plug }, { to: '/budgets', label: 'Бюджеты', icon: Wallet }, { to: '/audit', label: 'Аудит', icon: FileClock }, { to: '/settings', label: 'Настройки', icon: Settings }]} account={{ label: useAuth.getState().displayName ?? identity.data.subject, onLogout: logout, pending: logoutPending }} context={<NamespacePicker value={selected} loading={namespaces.isPending} unavailable={namespaces.isError || context.invalid} options={(namespaces.isError ? [] : namespaces.data ?? []).map(binding => ({ value: `${binding.namespace.registry_instance_id}/${binding.namespace.namespace_id}`, label: binding.label }))} onChange={value => navigate(selectedNamespaceUrl(location.pathname, location.search, value))} />}>
    {context.invalid ? <ResourceStateView state={{ kind: 'error', message: 'Некорректная UUID-пара Namespace в адресе.' }} /> : <Routes><Route path="/" element={<FoundationOverview />} /><Route path="/settings" element={<FoundationOverview settings />} /><Route path="/budgets" element={<Budgets key={`${identity.data.subject}/${namespaceQueryKey(location.search).join('/')}`} identity={identity.data} namespaces={namespaces.data} />} /><Route path="/providers" element={<Providers key={`${identity.data.installation_id}/${identity.data.subject}`} identity={identity.data} />} /><Route path="/providers/:id" element={<ConnectionDetails key={`${identity.data.installation_id}/${identity.data.subject}/${location.pathname}`} identity={identity.data} />} /><Route path="/models" element={<Models key={`${identity.data.installation_id}/${identity.data.subject}`} identity={identity.data} />} /><Route path="/models/:id" element={<ModelDetails key={`${identity.data.installation_id}/${identity.data.subject}/${location.pathname}`} identity={identity.data} />} /><Route path="/audit" element={<Audit />} /><Route path="*" element={<ResourceStateView state={{ kind: 'not-found' }} />} /></Routes>}
  </AppShell>
}

function FoundationOverview({ settings = false }: { settings?: boolean }) {
  const version = useQuery({ queryKey: ['version'], queryFn: () => client.get<Version>('/version') })
  return <DashboardLayout title={settings ? 'Настройки' : 'Обзор'} context="Установка" state={version.isPending ? { kind: 'loading' } : version.isError ? resourceErrorState(version.error, () => void version.refetch()) : { kind: 'ready' }}>
    {version.data && <Card className="p-5"><h2 className="mb-4 text-lg font-semibold">Состояние сервиса</h2><dl className="hub-facts"><dt>Установка</dt><dd>{version.data.installation_id}</dd><dt>Схема</dt><dd>{version.data.schema_revision}</dd><dt>Внешние вызовы</dt><dd>{version.data.external_calls ? 'Разрешены' : 'Выключены'}</dd><dt>Исходники</dt><dd>{version.data.source_revision || 'Локальная сборка'}{version.data.source_dirty && ' · незакоммиченные изменения'}</dd><dt>Base SDK</dt><dd>{version.data.base_revision}</dd><dt>Namespace cohort</dt><dd>{version.data.namespace_base_revision}</dd></dl></Card>}
  </DashboardLayout>
}

function Audit() {
  const location = useLocation()
  const context = contextFromSearch(location.search)
  const audit = useInfiniteQuery({ queryKey: ['audit', authGeneration(), ...namespaceQueryKey(location.search)], initialPageParam: undefined as string | undefined, getNextPageParam: (page: AuditPage) => page.next_cursor ?? undefined, queryFn: ({ pageParam, signal }) => {
    const query = new URLSearchParams(context.namespace ? { registry_instance_id: context.namespace.registry_instance_id, namespace_id: context.namespace.namespace_id } : {})
    if (pageParam) query.set('cursor', pageParam)
    return client.get<AuditPage>(`/api/v1/audit?${query}`, signal)
  }, enabled: !context.invalid })
  const items = audit.isError ? [] : audit.data?.pages.flatMap(page => page.items) ?? []
  return <ListPage title="Аудит" context="Изменения конфигурации и финансовые операции" state={audit.isPending ? { kind: 'loading' } : audit.isError ? resourceErrorState(audit.error, () => void audit.refetch()) : items.length === 0 ? { kind: 'empty', message: 'Доступных событий пока нет' } : { kind: 'ready' }}>
    <div className="hub-audit">{items.map(event => <Card key={event.id} className="p-4"><h2 className="font-semibold">{event.action}</h2><p className="my-2 break-all text-sm text-text-secondary">{event.object_id}</p><dl className="hub-facts text-sm"><dt>Автор</dt><dd>{event.actor}</dd><dt>Время</dt><dd>{new Date(event.happened_at).toLocaleString('ru-RU')}</dd><dt>Операция</dt><dd>{event.operation_id}</dd></dl></Card>)}</div>
    {audit.hasNextPage && <Button className="mt-4" disabled={audit.isFetchingNextPage} onClick={() => void audit.fetchNextPage()}>Показать ещё</Button>}
  </ListPage>
}

export function App() {
  const config = useQuery({ queryKey: ['public-config'], queryFn: () => client.get<PublicConfig>('/api/v1/public/config') })
  if (config.isPending) return <LoadingState message="Загрузка AI Hub…" />
  if (config.isError) return <ErrorState message="AI Hub временно недоступен" onRetry={() => void config.refetch()} />
  const ssoConfig: SsoConfig = { issuer: config.data.auth_issuer, clientId: config.data.oidc_client_id }
  return <PlatformProvider configUrl={config.data.branding_url ?? null} getAccessToken={() => useAuth.getState().token}><PlatformServicesProvider catalogUrl={config.data.service_catalog_url ?? null}><Routes><Route path="/sso/callback" element={<SsoCallback config={ssoConfig} />} /><Route path="/login" element={<Login config={ssoConfig} />} /><Route path="*" element={<ProtectedApp config={ssoConfig} />} /></Routes></PlatformServicesProvider></PlatformProvider>
}
