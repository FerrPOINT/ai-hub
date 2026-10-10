import type { Budget, BudgetInput } from '../../shared/api/client'
import { client } from '../../shared/api/client'

export function money(value: string) { return value.replace(/(\.\d*?)0+$/, '$1').replace(/\.$/, '').replace('.', ',') }
export function thresholds(value: string): number[] {
  const entries = value.split(',').map(v => v.trim())
  if (!entries.length || entries.length > 5 || entries.some(v => !/^[1-9][0-9]?$/.test(v))) throw new Error('Укажите от одного до пяти порогов от 1 до 99, через запятую.')
  const result = entries.map(Number)
  if (new Set(result).size !== result.length) throw new Error('Пороги не должны повторяться.')
  return result
}
export function validatePolicy(policy: BudgetInput) {
  if (typeof policy.hard_limit !== 'string' || !/^(0|[1-9][0-9]{0,19})(\.[0-9]{1,18})?$/.test(policy.hard_limit)) throw new Error('Введите точный неотрицательный лимит, например 0.10.')
  if (typeof policy.currency !== 'string' || !/^[A-Z]{3}$/.test(policy.currency)) throw new Error('Валюта — три заглавные латинские буквы.')
  thresholds(policy.warning_thresholds.join(','))
}
export function sameIdentity(a: BudgetInput, b: BudgetInput) {
  return a.scope_type === b.scope_type && a.scope_id === b.scope_id && a.currency === b.currency && a.period === b.period && a.namespace?.registry_instance_id === b.namespace?.registry_instance_id && a.namespace?.namespace_id === b.namespace?.namespace_id
}
export type BudgetIntent = { key: string; policy: BudgetInput; update?: { id: string; version: number } }
const uuid = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i
const nonnil = (v: unknown): v is string => typeof v === 'string' && uuid.test(v) && !/^0{8}-0{4}-0{4}-0{4}-0{12}$/.test(v)
export function readIntent(storageKey: string): BudgetIntent | null {
  const raw = sessionStorage.getItem(storageKey)
  if (!raw) return null
  if (raw.length > 10000) throw new Error('Сохранённая операция повреждена.')
  const value = JSON.parse(raw) as BudgetIntent
  const p = value?.policy
  if (!nonnil(value?.key) || !p || !nonnil(p.scope_id) || !['installation', 'project', 'client', 'profile'].includes(p.scope_type) || !['utc_day', 'utc_month'].includes(p.period) || !Array.isArray(p.warning_thresholds) || p.warning_thresholds.some(v => !Number.isInteger(v))) throw new Error('Сохранённая операция повреждена.')
  if (p.scope_type === 'project' ? !p.namespace || !nonnil(p.namespace.registry_instance_id) || p.scope_id !== p.namespace.namespace_id : p.namespace !== null) throw new Error('Сохранённый Namespace операции повреждён.')
  if (value.update && (!nonnil(value.update.id) || !Number.isSafeInteger(value.update.version) || value.update.version < 1)) throw new Error('Сохранённая версия операции повреждена.')
  validatePolicy(p)
  return value
}
export async function sendIntent(intent: BudgetIntent): Promise<Budget> {
  return client.request<Budget>(intent.update ? `/api/v1/budgets/${intent.update.id}` : '/api/v1/budgets', {
    method: intent.update ? 'PATCH' : 'POST',
    headers: { 'Content-Type': 'application/json', 'Idempotency-Key': intent.key, ...(intent.update ? { 'If-Match': `"${intent.update.version}"` } : {}) },
    body: JSON.stringify(intent.policy),
  })
}
