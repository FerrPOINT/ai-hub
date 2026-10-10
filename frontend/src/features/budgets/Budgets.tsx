import { useRef, useState } from 'react'
import { useInfiniteQuery } from '@tanstack/react-query'
import { useLocation } from 'react-router'
import { ApiError } from '@sdlc/ui/lib'
import { Button, Card, FormField, Input, ListPage, ResourceStateView, Select, UnsavedChangesGuard, resourceErrorState } from '@sdlc/ui/ui'
import { authGeneration, client, queryClient, type Budget, type BudgetInput, type BudgetPage, type Identity, type NamespaceBinding } from '../../shared/api/client'
import { contextFromSearch, namespaceQueryKey } from '../../shared/namespace'
import { money, readIntent, sameIdentity, sendIntent, thresholds, validatePolicy, type BudgetIntent } from './model'

export function Budgets({ identity, namespaces = [] }: { identity: Identity; namespaces?: NamespaceBinding[] }) {
  const location = useLocation()
  const context = contextFromSearch(location.search)
  const storageKey = `aihub.budget-intent:${identity.installation_id}:${identity.subject}:${namespaceQueryKey(location.search).join('/')}`
  const [recovery] = useState(() => { try { return { intent: readIntent(storageKey), error: '' } } catch { return { intent: null, error: 'Не удалось прочитать сохранённую операцию. Восстановите данные вкладки перед новым изменением.' } } })
  const [editor, setEditor] = useState<{ policy: BudgetInput; update?: { id: string; version: number } } | null>(recovery.intent)
  const [intent, setIntent] = useState<BudgetIntent | null>(recovery.intent)
  const [cap, setCap] = useState(recovery.intent?.policy.hard_limit ?? '0.10')
  const [warnings, setWarnings] = useState(recovery.intent?.policy.warning_thresholds.join(', ') ?? '80, 95')
  const [error, setError] = useState(recovery.error)
  const [pending, setPending] = useState(false)
  const [stale, setStale] = useState(false)
  const [notice, setNotice] = useState('')
  const sending = useRef(false)
  const writable = identity.capabilities.includes('config.write')
  function scopeLabel(b: Budget) {
    if (b.policy.scope_type === 'installation') return 'Установка'
    if (b.policy.scope_type === 'project') return namespaces.find(n => n.namespace.registry_instance_id === b.namespace?.registry_instance_id && n.namespace.namespace_id === b.namespace?.namespace_id)?.label ?? 'Проект'
    return `${b.policy.scope_type === 'client' ? 'Приложение' : 'Профиль'} ${b.policy.scope_id.slice(0, 8)}`
  }
  const budgets = useInfiniteQuery({ queryKey: ['budgets', authGeneration(), ...namespaceQueryKey(location.search)], initialPageParam: undefined as string | undefined, getNextPageParam: (page: BudgetPage) => page.next_cursor ?? undefined, queryFn: ({ pageParam, signal }) => {
    const query = new URLSearchParams(context.namespace ? { registry_instance_id: context.namespace.registry_instance_id, namespace_id: context.namespace.namespace_id } : {})
    query.set('limit', '100')
    if (pageParam) query.set('cursor', pageParam)
    return client.get<BudgetPage>(`/api/v1/budgets?${query}`, signal)
  }, enabled: identity.capabilities.includes('config.read') && !context.invalid })
  const records = budgets.isError ? [] : budgets.data?.pages.flatMap(p => p.items) ?? []
  const disabled = pending || !!intent || !!recovery.error
  function edit(budget?: Budget) {
    if (budget && !Number.isSafeInteger(budget.version)) { setError('Версия превышает поддерживаемый диапазон.'); return }
    const policy: BudgetInput = budget?.policy ?? { scope_type: context.namespace ? 'project' : 'installation', scope_id: context.namespace?.namespace_id ?? identity.installation_id, namespace: context.namespace ?? null, currency: 'USD', period: 'utc_day', hard_limit: '0.10', warning_thresholds: [80, 95] }
    setEditor({ policy, ...(budget ? { update: { id: budget.id, version: budget.version } } : {}) }); setCap(policy.hard_limit); setWarnings(policy.warning_thresholds.join(', ')); setError(''); setStale(false); setNotice('')
  }
  async function submit() {
    if (!editor || sending.current || recovery.error || stale) return
    const recovering = !!intent
    let current = intent
    try {
      if (!current) {
        const policy = { ...editor.policy, hard_limit: cap, warning_thresholds: thresholds(warnings) }
        validatePolicy(policy)
        current = { key: crypto.randomUUID(), policy, update: editor.update }
        // Persist this exact non-secret control intent before I/O; reload uses its original key/body.
        sessionStorage.setItem(storageKey, JSON.stringify(current))
        setIntent(current)
      }
    } catch (cause) { setError(cause instanceof Error ? cause.message : 'Не удалось сохранить операцию во вкладке.'); return }
    sending.current = true; setPending(true); setError('')
    try {
      const result = await sendIntent(current)
      sessionStorage.removeItem(storageKey); setIntent(null); setEditor(null); setStale(false)
      setNotice(`Бюджет сохранён · версия ${result.version}`)
      await queryClient.invalidateQueries({ queryKey: ['budgets'] })
    } catch (cause) {
      const rejected = cause instanceof ApiError && cause.status >= 400 && cause.status < 500 && ![408, 429].includes(cause.status)
      if (rejected && !recovering) { sessionStorage.removeItem(storageKey); setIntent(null) }
      const versionConflict = !recovering && !!editor.update && cause instanceof ApiError && cause.status === 412
      setStale(versionConflict)
      setError(versionConflict ? 'Бюджет изменён другим пользователем или изменился контекст проекта. Ваши значения сохранены. Загрузите актуальную версию перед повторным сохранением.' : rejected && !recovering ? cause.message : rejected ? 'Исходная операция сохранена, но её результат сейчас недоступен. Восстановите доступ и проверьте её тем же ключом.' : 'Результат сохранения пока неизвестен. Проверьте исходную операцию тем же ключом; поля временно заблокированы.')
    } finally { sending.current = false; setPending(false) }
  }
  async function rebase() {
    if (!editor?.update || sending.current) return
    setPending(true); sending.current = true
    try {
      let cursor: string | undefined
      const seen = new Set<string>()
      let found: Budget | undefined
      do {
        const query = new URLSearchParams({ limit: '100', ...(context.namespace ?? {}), ...(cursor ? { cursor } : {}) })
        const page = await client.get<BudgetPage>(`/api/v1/budgets?${query}`)
        found = page.items.find(b => b.id === editor.update?.id)
        if (found) break
        cursor = page.next_cursor ?? undefined
        if (cursor && (seen.has(cursor) || seen.size >= 100)) throw new Error('Некорректная пагинация бюджета.')
        if (cursor) seen.add(cursor)
      } while (cursor)
      if (!found) throw new Error('Бюджет больше недоступен в этом контексте.')
      if (!Number.isSafeInteger(found.version)) throw new Error('Версия превышает поддерживаемый диапазон.')
      if (!sameIdentity(found.policy, editor.policy)) throw new Error('Изменилась привязка бюджета. Откройте его заново; ваши значения остаются в форме.')
      setEditor({ ...editor, update: { id: found.id, version: found.version } }); setStale(false); setError('')
      setNotice(`Текущий лимит ${money(found.policy.hard_limit)} ${found.policy.currency}; пороги ${found.policy.warning_thresholds.join(', ')}%. Ваши изменения сохранены в форме.`)
    } catch (cause) { setError(cause instanceof Error ? cause.message : 'Не удалось загрузить актуальную версию.') }
    finally { sending.current = false; setPending(false) }
  }
  return <>
    <UnsavedChangesGuard when={!!editor || !!intent} />
    <ListPage title="Бюджеты" context="Лимиты, расходы и удержанные резервы" actions={writable && <Button disabled={!!editor || !!recovery.error} onClick={() => edit()}>Создать бюджет</Button>} state={!identity.capabilities.includes('config.read') ? { kind: 'permission-denied' } : budgets.isPending ? { kind: 'loading' } : budgets.isError ? resourceErrorState(budgets.error, () => void budgets.refetch()) : records.length ? { kind: 'ready' } : { kind: 'empty', message: 'Бюджеты не настроены' }}>
      <div className="grid min-w-0 gap-4 md:grid-cols-2 xl:grid-cols-3">{records.map(b => <Card key={b.id} className="min-w-0 p-5"><h2 className="font-semibold">{scopeLabel(b)} · {b.policy.currency}</h2><p className="my-3 text-sm text-text-secondary">{b.policy.period === 'utc_day' ? 'Лимит в день' : 'Лимит в месяц'} · {money(b.policy.hard_limit)}</p><dl className="hub-facts text-sm"><dt>Учтено</dt><dd>{money(b.charged)}</dd><dt>Резерв</dt><dd>{money(b.reserved)}</dd><dt>Доступно</dt><dd className={b.remaining.startsWith('-') ? 'text-danger' : ''}>{money(b.remaining)}</dd><dt>Сброс · UTC</dt><dd>{new Date(b.period_end).toLocaleString('ru-RU', { timeZone: 'UTC' })}</dd><dt>Пороги</dt><dd>{b.policy.warning_thresholds.join(', ')}%</dd></dl>{writable && <Button className="mt-4" variant="outline" disabled={!!editor || !!recovery.error} onClick={() => edit(b)}>Изменить</Button>}</Card>)}</div>
      {budgets.hasNextPage && <Button disabled={budgets.isFetchingNextPage} onClick={() => void budgets.fetchNextPage()}>Показать ещё</Button>}
    </ListPage>
    {notice && <p role="status" className="mt-4 text-sm">{notice}</p>}
    {error && <p role="alert" className="mt-4 text-danger">{error}</p>}
    {editor && <Card className="mt-5 min-w-0 p-5"><form aria-busy={pending} onSubmit={event => { event.preventDefault(); void submit() }}><h2 className="mb-2 text-lg font-semibold">{editor.update ? 'Изменить бюджет' : 'Создать бюджет'}</h2><p className="mb-4 text-sm text-text-secondary">Область: {editor.policy.scope_type === 'installation' ? 'вся установка' : editor.policy.scope_type === 'project' ? 'выбранный проект' : editor.policy.scope_type === 'client' ? 'приложение' : 'профиль'}</p><fieldset disabled={disabled} className="grid min-w-0 gap-4 border-0 p-0 md:grid-cols-2">
      <FormField id="budget-cap" label="Жёсткий лимит" required hint="Точное число с точкой; расходы и резервы сохраняются.">{attrs => <Input {...attrs} inputMode="decimal" value={cap} maxLength={39} onChange={e => setCap(e.target.value)} />}</FormField>
      <FormField id="budget-thresholds" label="Пороги предупреждения, %" required hint="От 1 до 99, через запятую.">{attrs => <Input {...attrs} value={warnings} maxLength={25} onChange={e => setWarnings(e.target.value)} />}</FormField>
      <FormField id="budget-currency" label="Валюта" required>{attrs => <Input {...attrs} value={editor.policy.currency} readOnly={!!editor.update} maxLength={3} onChange={e => setEditor({ ...editor, policy: { ...editor.policy, currency: e.target.value.toUpperCase() } })} />}</FormField>
      <FormField id="budget-period" label="Период · UTC" required>{attrs => <Select {...attrs} disabled={!!editor.update} value={editor.policy.period} onChange={e => setEditor({ ...editor, policy: { ...editor.policy, period: e.target.value as BudgetInput['period'] } })}><option value="utc_day">День</option><option value="utc_month">Календарный месяц</option></Select>}</FormField>
    </fieldset><div className="mt-5 flex flex-wrap gap-3">{stale ? <Button type="button" disabled={pending} onClick={() => void rebase()}>Загрузить актуальную версию</Button> : <Button disabled={pending || !writable || !!recovery.error}>{pending ? 'Проверка операции…' : intent ? 'Проверить исходную операцию' : 'Сохранить'}</Button>}<Button type="button" variant="outline" disabled={pending || !!intent} onClick={() => { setEditor(null); setError(''); setNotice('') }}>Отмена</Button></div></form></Card>}
    {!editor && recovery.error && <ResourceStateView state={{ kind: 'error', message: recovery.error }} />}
  </>
}
