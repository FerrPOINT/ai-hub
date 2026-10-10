import { authGeneration, client, type Profile, type ProfileInput, type ProfilePage } from '../../shared/api/client';
export const modelKeys = { all: ['models'] as const, list: () => ['models', 'list', authGeneration()] as const, detail: (id: string) => ['models', 'detail', authGeneration(), id] as const };
export async function allProfiles(signal?: AbortSignal) {
    // ponytail: bounded 10k actor snapshot; move filtering to API at this ceiling.
    const items: Profile[] = [];
    const seen = new Set<string>();
    let cursor: string | undefined;
    do {
        const page = await client.get<ProfilePage>(`/api/v1/virtual-models?${new URLSearchParams({ limit: '100', ...(cursor ? { cursor } : {}) })}`, signal);
        items.push(...page.items);
        cursor = page.next_cursor ?? undefined;
        if (items.length > 10000 || cursor && seen.has(cursor))
            throw new Error('Некорректная пагинация профилей.');
        if (cursor)
            seen.add(cursor);
    } while (cursor);
    return items;
}
export async function readProfile(id: string, signal?: AbortSignal) {
    const response = await client.raw(`/api/v1/virtual-models/${encodeURIComponent(id)}`, { signal });
    const value = await response.json() as Profile;
    const version = response.headers.get('etag');
    if (value.id !== id || !Number.isSafeInteger(value.draft_version) || value.draft_version < 1 || version !== `"${value.draft_version}"`)
        throw new Error('Не подтверждена версия профиля.');
    return value;
}
export function saveProfile(key: string, input: ProfileInput, update?: {
    id: string;
    version: number;
}) { return client.request<Profile>(update ? `/api/v1/virtual-models/${encodeURIComponent(update.id)}` : '/api/v1/virtual-models', { method: update ? 'PATCH' : 'POST', headers: { 'Content-Type': 'application/json', 'Idempotency-Key': key, ...(update ? { 'If-Match': `"${update.version}"` } : {}) }, body: JSON.stringify(input) }); }
const uuid = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
export function validateProfile(input: ProfileInput) {
    if (!/^[a-z][a-z0-9-]{1,62}$/.test(input.slug) || !input.display_name.trim() || Array.from(input.display_name).length > 120 || !['development', 'pinned_test'].includes(input.mode))
        throw new Error('Проверьте slug, название и режим профиля.');
    if (!input.deployments.length || input.deployments.length > 5 || input.deployments.some(d => !uuid.test(d.connection_id) || d.connection_id === '00000000-0000-0000-0000-000000000000' || !Number.isSafeInteger(d.generation) || d.generation < 1 || !d.model_id || new TextEncoder().encode(d.model_id).length > 256 || /[\u0000-\u001f\u007f]/.test(d.model_id)) || new Set(input.deployments.map(d => JSON.stringify([d.connection_id, d.generation, d.model_id]))).size !== input.deployments.length)
        throw new Error('Укажите от 1 до 5 разных точных deployment.');
    if ([input.input_limit, input.output_limit, input.context_limit].some(v => !Number.isInteger(v) || v < 1 || v > 4294967295) || input.input_limit > input.context_limit || input.output_limit > input.context_limit || !Number.isInteger(input.timeout_seconds) || input.timeout_seconds < 1 || input.timeout_seconds > 600 || !Number.isInteger(input.max_attempts) || input.max_attempts < 1 || input.max_attempts > 5)
        throw new Error('Проверьте лимиты токенов, timeout и число попыток.');
    if (input.mode === 'pinned_test' && (input.deployments.length !== 1 || input.max_attempts !== 1))
        throw new Error('Pinned test требует один deployment и одну попытку.');
    const p = input.parameters;
    if (p.temperature === null || p.top_p === null || p.reasoning_effort === null)
        throw new Error('Для отсутствующего параметра оставьте поле пустым.');
    if (p.temperature !== undefined && p.temperature !== null && (!Number.isFinite(p.temperature) || p.temperature < 0 || p.temperature > 2) || p.top_p !== undefined && p.top_p !== null && (!Number.isFinite(p.top_p) || p.top_p <= 0 || p.top_p > 1) || p.reasoning_effort !== undefined && p.reasoning_effort !== null && (!p.reasoning_effort || p.reasoning_effort.length > 32 || /[\u0000-\u001f\u007f]/.test(p.reasoning_effort)))
        throw new Error('Проверьте параметры генерации.');
    if (new Set(input.required_capabilities).size !== input.required_capabilities.length || input.required_capabilities.some(c => !['text', 'stream', 'function_tools', 'json_schema', 'responses', 'cancel'].includes(c)) || new Set(input.allowed_overrides ?? []).size !== (input.allowed_overrides ?? []).length || (input.allowed_overrides ?? []).some(c => !['temperature', 'top_p', 'max_output_tokens', 'reasoning_effort'].includes(c)))
        throw new Error('Проверьте capabilities и caller overrides.');
}
export const emptyProfile = (): ProfileInput => ({ slug: '', display_name: '', mode: 'development', deployments: [{ connection_id: '', generation: 1, model_id: '' }], parameters: {}, input_limit: 2048, output_limit: 512, context_limit: 4096, required_capabilities: ['text'], timeout_seconds: 120, max_attempts: 3, allowed_overrides: [] });
export const profileLabels: Record<string, string> = { draft: 'Черновик', active: 'Опубликован', disabled: 'Отключён', archived: 'В архиве' };
export const profileStatus = (p: Profile) => profileLabels[p.status] ?? p.status;
