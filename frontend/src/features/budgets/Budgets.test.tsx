import { afterEach, expect, it, vi } from 'vitest'
import { cleanup, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { QueryClientProvider } from '@tanstack/react-query'
import { createMemoryRouter, RouterProvider } from 'react-router'
import { Budgets } from './Budgets'
import { money, sameIdentity } from './model'
import { queryClient, useAuth, type Budget, type BudgetInput, type Identity } from '../../shared/api/client'
import '../../app/i18n'

const identity: Identity = { subject: 'human-budget-test', installation_id: 'c47b4c36-5132-42bc-af03-5e581b90a3ce', capabilities: ['config.read', 'config.write'], project_grants: ['installation'] }
const policy: BudgetInput = { scope_type: 'installation', scope_id: identity.installation_id, namespace: null, currency: 'USD', period: 'utc_day', hard_limit: '0.10', warning_thresholds: [80, 95] }
const original: Budget = { id: '12259f65-c6ef-42a2-b031-5228cd0376c9', policy, namespace: null, charged: '0.04', reserved: '0.02', remaining: '0.04', version: 1, period_start: '2026-10-10T00:00:00Z', period_end: '2026-10-11T00:00:00Z' }
const json = (body: unknown, status = 200) => new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } })
function mount() {
  useAuth.getState().setAuth({ token: 'fixture-bearer', userId: identity.subject, email: 'test@example.test', displayName: 'Fixture' })
  return render(<QueryClientProvider client={queryClient}><RouterProvider router={createMemoryRouter([{ path: '*', element: <Budgets identity={identity} /> }], { initialEntries: ['/budgets'] })} /></QueryClientProvider>)
}
afterEach(() => { cleanup(); vi.unstubAllGlobals(); useAuth.getState().logout(); queryClient.clear(); sessionStorage.clear() })

it('reload after an ambiguous control reply retains exact key/body and locks editing until readback', async () => {
  const calls: { key: string; body: string }[] = []
  let saved: Budget | undefined
  vi.stubGlobal('fetch', vi.fn(async (_url: string, init?: RequestInit) => {
    if (init?.method !== 'POST') return json({ items: saved ? [saved] : [], next_cursor: null })
    const payload = String(init.body)
    calls.push({ key: new Headers(init.headers).get('Idempotency-Key')!, body: payload })
    saved = { ...original, policy: JSON.parse(payload) as BudgetInput }
    if (calls.length === 1) throw new TypeError('fixture lost reply after commit')
    if (calls.length === 2) return json({ error: { code: 'permission_denied', message: 'fixture revoked after original commit' } }, 403)
    return json(saved, 201)
  }))
  const user = userEvent.setup()
  mount()
  await screen.findByText('Бюджеты не настроены')
  await user.click(screen.getByRole('button', { name: 'Создать бюджет' }))
  await user.clear(screen.getByLabelText('Жёсткий лимит', { exact: false }))
  await user.type(screen.getByLabelText('Жёсткий лимит', { exact: false }), '0.100000000000000001')
  await user.click(screen.getByRole('button', { name: 'Сохранить' }))
  await screen.findByText(/Результат сохранения пока неизвестен/)
  const lockedCap = screen.getByLabelText('Жёсткий лимит', { exact: false }) as HTMLInputElement
  expect(lockedCap.matches(':disabled')).toBe(true)
  await user.type(lockedCap, '2')
  expect(lockedCap.value).toBe('0.100000000000000001')
  expect(Object.values(sessionStorage).join('')).not.toContain('fixture-bearer')
  cleanup()
  mount()
  await user.click(await screen.findByRole('button', { name: 'Проверить исходную операцию' }))
  await screen.findByText(/Исходная операция сохранена, но её результат сейчас недоступен/)
  expect(sessionStorage.length).toBe(1)
  expect((screen.getByLabelText('Жёсткий лимит', { exact: false }) as HTMLInputElement).matches(':disabled')).toBe(true)
  await user.click(screen.getByRole('button', { name: 'Проверить исходную операцию' }))
  await screen.findByText('Бюджет сохранён · версия 1')
  expect(calls).toHaveLength(3)
  expect(calls[1]).toEqual(calls[0])
  expect(calls[2]).toEqual(calls[0])
  expect(JSON.parse(calls[1].body).hard_limit).toBe('0.100000000000000001')
  expect(sessionStorage.length).toBe(0)
}, 10000)

it('412 preserves the draft and explicitly rebases the CAS version before a new mutation', async () => {
  let current = original
  const updates: { key: string; version: string; body: BudgetInput }[] = []
  vi.stubGlobal('fetch', vi.fn(async (_url: string, init?: RequestInit) => {
    if (init?.method !== 'PATCH') return json({ items: [current], next_cursor: null })
    const headers = new Headers(init.headers)
    const body = JSON.parse(String(init.body)) as BudgetInput
    updates.push({ key: headers.get('Idempotency-Key')!, version: headers.get('If-Match')!, body })
    if (updates.length === 1) {
      current = { ...original, version: 2, policy: { ...policy, hard_limit: '0.23' } }
      return json({ error: { code: 'precondition_failed', message: 'fixture conflict' } }, 412)
    }
    current = { ...current, policy: body, version: 3 }
    return json(current, 201)
  }))
  const user = userEvent.setup()
  mount()
  await user.click(await screen.findByRole('button', { name: 'Изменить' }))
  const cap = screen.getByLabelText('Жёсткий лимит', { exact: false }) as HTMLInputElement
  await user.clear(cap); await user.type(cap, '0.17')
  await user.click(screen.getByRole('button', { name: 'Сохранить' }))
  await screen.findByText(/Бюджет изменён другим пользователем/)
  expect(cap.value).toBe('0.17')
  expect(sessionStorage.length).toBe(0)
  await user.click(screen.getByRole('button', { name: 'Загрузить актуальную версию' }))
  await screen.findByText(/Текущий лимит 0,23 USD/)
  expect(cap.value).toBe('0.17')
  await user.click(screen.getByRole('button', { name: 'Сохранить' }))
  await waitFor(() => expect(updates.length).toBe(2))
  expect(updates.map(u => u.version)).toEqual(['"1"', '"2"'])
  expect(updates[1].key).not.toBe(updates[0].key)
  expect(updates[1].body.hard_limit).toBe('0.17')
  await screen.findByText('Бюджет сохранён · версия 3')
}, 10000)

it('identity comparison uses the full registry pair and exact display never rounds monetary strings', () => {
  expect(money('99999999999999999999.000000000000000001')).toBe('99999999999999999999,000000000000000001')
  expect(money('-0.030000000000000000')).toBe('-0,03')
  expect(sameIdentity(policy, { ...policy, hard_limit: '2', warning_thresholds: [95, 80] })).toBe(true)
  const project: BudgetInput = { ...policy, scope_type: 'project', scope_id: original.id, namespace: { registry_instance_id: identity.installation_id, namespace_id: original.id } }
  expect(sameIdentity(project, { ...project, namespace: { ...project.namespace!, registry_instance_id: 'ab693a78-396c-4fd8-a623-902b8ddff2ab' } })).toBe(false)
})
