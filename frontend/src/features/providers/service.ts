import { authGeneration, client, type CatalogPage, type Connection, type ConnectionInput, type ConnectionPage, type CredentialInput, type EndpointPolicyInput, type EndpointPolicyPage, type MetadataRefreshInput, type Operation, type OperationLookup } from '../../shared/api/client';
export const providerKeys = { context: (id: string, model: string) => ["providers", "model-context", authGeneration(), id, model] as const, all: ['providers'] as const, list: () => ['providers', 'connections', authGeneration()] as const, detail: (id: string) => ['providers', 'connection', authGeneration(), id] as const, presets: () => ['providers', 'presets', authGeneration()] as const, catalog: (id: string, generation: number, query: string) => ['providers', 'catalog', authGeneration(), id, generation, query] as const };
export function listConnections(cursor?: string, signal?: AbortSignal) { return client.get<ConnectionPage>(`/api/v1/connections?${new URLSearchParams({ limit: '100', ...(cursor ? { cursor } : {}) })}`, signal); }
export async function allConnections(signal?: AbortSignal) {
    // ponytail: bounded 10k snapshot; move search to the API when this ceiling is reached.
    const items: Connection[] = [];
    const seen = new Set<string>();
    let cursor: string | undefined;
    do {
        const page = await listConnections(cursor, signal);
        items.push(...page.items);
        cursor = page.next_cursor ?? undefined;
        if (items.length > 10000 || cursor && seen.has(cursor))
            throw new Error('Некорректная пагинация подключений.');
        if (cursor)
            seen.add(cursor);
    } while (cursor);
    return items;
}
export function readConnection(id: string, signal?: AbortSignal) { return client.get<Connection>(`/api/v1/connections/${encodeURIComponent(id)}`, signal); }
export async function listPresets(signal?: AbortSignal) {
    const items: EndpointPolicyInput[] = [];
    const seen = new Set<string>();
    let cursor: string | undefined;
    do {
        const page = await client.get<EndpointPolicyPage>(`/api/v1/connection-presets?${new URLSearchParams({ limit: '100', ...(cursor ? { cursor } : {}) })}`, signal);
        items.push(...page.items);
        cursor = page.next_cursor ?? undefined;
        if (items.length > 10000 || cursor && seen.has(cursor))
            throw new Error('Некорректная пагинация подключений.');
        if (cursor)
            seen.add(cursor);
    } while (cursor);
    return items;
}
export function readCatalog(id: string, query: string, cursor?: string, signal?: AbortSignal) { return client.get<CatalogPage>(`/api/v1/connections/${encodeURIComponent(id)}/models?${new URLSearchParams({ q: query, limit: '100', ...(cursor ? { cursor } : {}) })}`, signal); }
export function lookupOperation(key: string) { return client.get<OperationLookup>(`/api/v1/operations/by-key/${encodeURIComponent(key)}`); }
export function closeUnstarted(key: string) { return client.post<OperationLookup>(`/api/v1/operations/by-key/${encodeURIComponent(key)}/close-unstarted`); }
export function saveConnection(key: string, kind: Connection['provider_kind'], input: ConnectionInput, update?: {
    id: string;
    version: number;
}) { return client.request<Connection>(update ? `/api/v1/connections/${encodeURIComponent(update.id)}` : `/api/v1/providers/${kind}/connections`, { method: update ? 'PATCH' : 'POST', headers: { 'Content-Type': 'application/json', 'Idempotency-Key': key, ...(update ? { 'If-Match': `"${update.version}"` } : {}) }, body: JSON.stringify(input) }); }
export function writeCredential(key: string, id: string, input: CredentialInput) { return client.request<Operation>(`/api/v1/connections/${encodeURIComponent(id)}/credentials`, { method: 'PUT', headers: { 'Content-Type': 'application/json', 'Idempotency-Key': key }, body: JSON.stringify(input) }); }
export function revokeCredential(key: string, id: string, version: number) { return client.request<Operation>(`/api/v1/connections/${encodeURIComponent(id)}/credentials`, { method: 'DELETE', headers: { 'Idempotency-Key': key, 'If-Match': `"${version}"` } }); }
export function refreshCatalog(key: string, id: string, input: MetadataRefreshInput) { return client.request<Operation>(`/api/v1/connections/${encodeURIComponent(id)}/models/refresh`, { method: 'POST', headers: { 'Content-Type': 'application/json', 'Idempotency-Key': key }, body: JSON.stringify(input) }); }
export function validateSettings(input: ConnectionInput) {
    if (!input.display_name.trim() || Array.from(input.display_name).length > 120 || !/^[A-Za-z0-9_.-]{1,120}$/.test(input.endpoint_policy_ref) || !['unknown', 'metered', 'subscription', 'local'].includes(input.billing_mode))
        throw new Error('Проверьте название, endpoint и способ учёта.');
}
export const kindLabel: Record<Connection['provider_kind'], string> = { openai_compatible: 'Совместимый API', ollama: 'Ollama', zai: 'Z.AI', chatgpt_managed: 'ChatGPT' };
export function connectionStatus(value: Connection) { return value.status === 'disabled' ? 'Отключено' : value.status === 'revoked' ? 'Ключ отозван' : value.status === 'enabled' ? 'Авторизация подтверждена' : 'Авторизация требует проверки'; }
export function validateContext(input: import('../../shared/api/client').ModelContextInput) { if (!input.model_id || new TextEncoder().encode(input.model_id).length > 256 || /[\u0000-\u001f\u007f]/.test(input.model_id) || !Number.isInteger(input.context_window_tokens) || input.context_window_tokens < 1 || input.context_window_tokens > 4294967295)
    throw new Error('Укажите точный model ID и целый контекст от 1 до 4294967295 токенов.'); }
export async function readModelContext(id: string, model: string, signal?: AbortSignal) {
    const response = await client.raw(`/api/v1/connections/${encodeURIComponent(id)}/model-contexts?${new URLSearchParams({ model_id: model })}`, { signal });
    const page = await response.json() as import('../../shared/api/client').ModelContextPage;
    const etag = response.headers.get('etag');
    if (!etag || !/^"[0-9]+"$/.test(etag))
        throw new Error('Не подтверждена версия контекста.');
    const version = Number(etag.slice(1, -1));
    const preference = page.items[0] ?? null;
    if (!Number.isSafeInteger(version) || version < 0 || page.items.length > 1)
        throw new Error('Не подтверждена версия контекста.');
    if (preference ? preference.connection_id !== id || preference.model_id !== model || preference.version !== version : version !== 0)
        throw new Error('Ответ относится к другому контексту.');
    return { version, preference };
}
export function saveModelContext(key: string, id: string, version: number, input: import('../../shared/api/client').ModelContextInput) { return client.request<import('../../shared/api/client').ModelContextPreference>(`/api/v1/connections/${encodeURIComponent(id)}/model-contexts`, { method: 'PUT', headers: { 'Content-Type': 'application/json', 'Idempotency-Key': key, 'If-Match': `"${version}"` }, body: JSON.stringify(input) }); }
