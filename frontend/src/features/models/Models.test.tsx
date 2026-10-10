import { afterEach, expect, it, vi } from 'vitest';
import { cleanup, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { QueryClientProvider } from '@tanstack/react-query';
import { createMemoryRouter, RouterProvider } from 'react-router';
import { ModelDetails } from './ModelDetails';
import { Models } from './Models';
import { queryClient, useAuth, type Identity, type Profile, type Connection } from '../../shared/api/client';
import '../../app/i18n';
const identity: Identity = { subject: 'model-test-human', installation_id: '11111111-1111-1111-1111-111111111111', capabilities: ['config.read', 'config.write'], project_grants: [] };
const connection: Connection = { id: '22222222-2222-2222-2222-222222222222', provider_kind: 'openai_compatible', display_name: 'Fixture provider', generation: 2, status: 'authorization_unknown', has_credentials: false, quota: null, version: 1, settings: { display_name: 'Fixture provider', endpoint_policy_ref: 'fixture', billing_mode: 'unknown' }, catalog_refresh_supported: false };
const profile: Profile = { id: '33333333-3333-3333-3333-333333333333', status: 'draft', draft_version: 1, active_revision_id: null, draft: { slug: 'exact-model', display_name: 'Тестовый профиль', mode: 'development', deployments: [{ connection_id: connection.id, generation: 2, model_id: 'vendor/CaseSensitive' }], parameters: { temperature: 0.3 }, input_limit: 2048, output_limit: 512, context_limit: 4096, required_capabilities: ['text'], timeout_seconds: 120, max_attempts: 3, allowed_overrides: [] } };
const json = (body: unknown, status = 200, headers: Record<string, string> = {}) => new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json', ...headers } });
function mount(path = `/models/${profile.id}`, who = identity) { useAuth.getState().setAuth({ token: 'fixture-model-bearer', userId: who.subject, email: 'fixture@example.test', displayName: 'Fixture' }); return render(<QueryClientProvider client={queryClient}><RouterProvider router={createMemoryRouter([{ path: '/models', element: <Models identity={who}/> }, { path: '/models/:id', element: <ModelDetails identity={who}/> }], { initialEntries: [path] })}/></QueryClientProvider>); }
function read(url: string, current = profile) {
    if (url.includes('/connections?'))
        return json({ items: [connection], next_cursor: null });
    if (url.includes('/model-contexts'))
        return json({ items: [], next_cursor: null }, 200, { etag: '"0"' });
    if (url.includes('/connections/') && url.includes('/models'))
        return json({ connection_id: connection.id, generation: 2, models: [], as_of: '2026-10-11T00:00:00Z', data_status: 'unavailable', next_cursor: null });
    if (url.includes('/virtual-models?'))
        return json({ items: [current], next_cursor: null });
    return json(current, 200, { etag: `"${current.draft_version}"` });
}
afterEach(() => { cleanup(); queryClient.clear(); useAuth.getState().logout(); sessionStorage.clear(); vi.unstubAllGlobals(); });
it('keeps the saved generation until the operator explicitly chooses the current one', async () => {
    const current = structuredClone(profile);
    current.draft.deployments[0]!.generation = 1;
    vi.stubGlobal('fetch', vi.fn(async (url) => read(String(url), current)));
    const user = userEvent.setup();
    mount();
    await user.click(await screen.findByRole('button', { name: 'Изменить черновик' }));
    expect((screen.getByLabelText('Поколение авторизации') as HTMLInputElement).value).toBe('1');
    await user.click(await screen.findByRole('button', { name: 'Использовать текущее поколение 2' }));
    expect((screen.getByLabelText('Поколение авторизации') as HTMLInputElement).value).toBe('2');
    expect((screen.getByLabelText(/^Точный model ID/) as HTMLInputElement).value).toBe('vendor/CaseSensitive');
});
it('keeps a restored unknown draft locked after 404 until the original key is fenced', async () => {
    const key = crypto.randomUUID();
    const draft = { ...structuredClone(profile.draft), display_name: 'Restored draft' };
    sessionStorage.setItem(`aihub.model-intent:${identity.installation_id}:${identity.subject}:${profile.id}`, JSON.stringify({ key, action: 'profile.draft.update', resource_id: profile.id, profile: draft }));
    let patches = 0;
    let fences = 0;
    vi.stubGlobal('fetch', vi.fn(async (url, init?: RequestInit) => {
        if (init?.method === 'PATCH') {
            patches++;
            throw new TypeError('must not resend');
        }
        if (String(url).includes('/close-unstarted')) {
            fences++;
            return json({ idempotency_key: key, action: 'operation.close-unstarted', operation: { id: crypto.randomUUID(), resource_id: null, status: 'cancelled', safe_error: null, version: 1 } });
        }
        if (String(url).includes('/operations/by-key/'))
            return json({ error: { code: 'not_found', message: 'Не найдено', request_id: crypto.randomUUID() } }, 404);
        return read(String(url));
    }));
    const user = userEvent.setup();
    mount();
    await screen.findByText('Редактирование черновика');
    expect((screen.getByLabelText(/^Название профиля/) as HTMLInputElement).value).toBe('Restored draft');
    await user.click(screen.getByRole('button', { name: 'Проверить исходную операцию' }));
    await screen.findByText(/Исходная операция пока не найдена/);
    expect((screen.getByRole('button', { name: 'Загрузить версию профиля' }) as HTMLButtonElement).disabled).toBe(true);
    expect(patches).toBe(0);
    await user.click(screen.getByRole('button', { name: 'Закрыть непринятый запрос' }));
    await screen.findByText(/Позднее выполнение исходного запроса запрещено/);
    expect(fences).toBe(1);
    expect(patches).toBe(0);
    expect(sessionStorage.length).toBe(0);
    await user.click(screen.getByRole('button', { name: 'Загрузить версию профиля' }));
    await screen.findByRole('button', { name: 'Сохранить черновик' });
    expect((screen.getByLabelText(/^Название профиля/) as HTMLInputElement).value).toBe('Restored draft');
});
it('keeps exact draft through 412 and sends only after explicit rebase', async () => {
    let current = structuredClone(profile);
    const writes: RequestInit[] = [];
    vi.stubGlobal('fetch', vi.fn(async (url, init?: RequestInit) => {
        if (init?.method === 'PATCH') {
            writes.push(init);
            if (writes.length === 1) {
                current = { ...current, draft_version: 2 };
                return json({ error: { code: 'precondition_failed', message: 'Конфликт', request_id: crypto.randomUUID() } }, 412);
            }
            current = { ...current, draft_version: 3, draft: JSON.parse(String(init.body)) };
            return json(current, 200, { etag: '"3"' });
        }
        if (String(url).includes('/operations/by-key/'))
            return json({ idempotency_key: writes.at(-1)?.headers ? new Headers(writes.at(-1)!.headers).get('idempotency-key') : null, action: 'profile.draft.update', operation: { id: crypto.randomUUID(), resource_id: profile.id, status: 'succeeded', safe_error: null, version: 2 } });
        return read(String(url), current);
    }));
    const user = userEvent.setup();
    mount();
    await user.click(await screen.findByRole('button', { name: 'Изменить черновик' }));
    const name = screen.getByLabelText(/^Название профиля/);
    await user.clear(name);
    await user.type(name, 'Мой сохранённый draft');
    await user.click(screen.getByRole('button', { name: 'Сохранить черновик' }));
    await screen.findByText(/Конфигурация изменена/);
    expect((name as HTMLInputElement).value).toBe('Мой сохранённый draft');
    expect(writes).toHaveLength(1);
    await user.click(screen.getByRole('button', { name: 'Загрузить версию профиля' }));
    await user.click(await screen.findByRole('button', { name: 'Сохранить черновик' }));
    await screen.findByText(/Черновик сохранён/);
    expect(writes).toHaveLength(2);
    expect(new Headers(writes[1]!.headers).get('if-match')).toBe('"2"');
    expect(JSON.parse(String(writes[1]!.body)).deployments[0].model_id).toBe('vendor/CaseSensitive');
    expect(new Headers(writes[0]!.headers).get('idempotency-key')).not.toBe(new Headers(writes[1]!.headers).get('idempotency-key'));
});
it('recovers a lost response by original key without repeating PATCH', async () => {
    let writes = 0;
    let key = '';
    let recovered = false;
    let current = structuredClone(profile);
    vi.stubGlobal('fetch', vi.fn(async (url, init?: RequestInit) => {
        if (init?.method === 'PATCH') {
            writes++;
            key = new Headers(init.headers).get('idempotency-key')!;
            current = { ...current, draft_version: 2, draft: JSON.parse(String(init.body)) };
            throw new TypeError('lost reply');
        }
        if (String(url).includes('/operations/by-key/')) {
            recovered = true;
            expect(String(url).endsWith(key)).toBe(true);
            return json({ idempotency_key: key, action: 'profile.draft.update', operation: { id: crypto.randomUUID(), resource_id: profile.id, status: 'succeeded', safe_error: null, version: 2 } });
        }
        return read(String(url), current);
    }));
    const user = userEvent.setup();
    mount();
    await user.click(await screen.findByRole('button', { name: 'Изменить черновик' }));
    await user.click(screen.getByRole('button', { name: 'Сохранить черновик' }));
    await screen.findByText(/Результат операции пока неизвестен/);
    const saved = sessionStorage.getItem(`aihub.model-intent:${identity.installation_id}:${identity.subject}:${profile.id}`)!;
    expect(JSON.parse(saved).profile.deployments[0].model_id).toBe('vendor/CaseSensitive');
    expect(saved).not.toContain('fixture-model-bearer');
    expect((screen.getByRole('button', { name: 'Сохранить черновик' }) as HTMLButtonElement).disabled).toBe(true);
    await user.click(screen.getByRole('button', { name: 'Проверить исходную операцию' }));
    await screen.findByText(/Черновик сохранён/);
    expect(writes).toBe(1);
    expect(recovered).toBe(true);
    expect(sessionStorage.length).toBe(0);
});
it('blocks invalid pinned routing and preserves the draft for a valid neighboring case', async () => {
    let writes = 0;
    let key = '';
    let created: Profile | null = null;
    vi.stubGlobal('fetch', vi.fn(async (url, init?: RequestInit) => {
        if (init?.method === 'POST') {
            writes++;
            key = new Headers(init.headers).get('idempotency-key')!;
            created = { ...profile, draft: JSON.parse(String(init.body)) };
            return json(created, 201, { etag: '"1"' });
        }
        if (String(url).includes('/operations/by-key/'))
            return json({ idempotency_key: key, action: 'profile.draft.create', operation: { id: crypto.randomUUID(), resource_id: profile.id, status: 'succeeded', safe_error: null, version: 2 } });
        return read(String(url), created ?? profile);
    }));
    const user = userEvent.setup();
    mount('/models');
    await user.click(await screen.findByRole('button', { name: 'Создать профиль' }));
    await user.type(screen.getByLabelText(/^Slug/), 'new-pinned');
    await user.type(screen.getByLabelText(/^Название профиля/), 'Pinned draft');
    await user.selectOptions(screen.getByLabelText(/^Подключение/), connection.id);
    await user.type(screen.getByLabelText(/^Точный model ID/), 'vendor/Neighbour');
    await user.selectOptions(screen.getByLabelText(/^Режим/), 'pinned_test');
    await user.click(screen.getByRole('button', { name: 'Сохранить черновик' }));
    await screen.findByText(/Pinned test требует/);
    expect(writes).toBe(0);
    const attempts = screen.getByLabelText(/^Максимум попыток/);
    await user.clear(attempts);
    await user.type(attempts, '1');
    await user.click(screen.getByRole('button', { name: 'Сохранить черновик' }));
    await waitFor(() => expect(screen.queryByRole('heading', { name: 'Новый профиль' })).toBeNull());
    expect(writes).toBe(1);
    expect(created!.draft.deployments[0]!.model_id).toBe('vendor/Neighbour');
    expect(created!.draft.allowed_overrides).toEqual([]);
});
