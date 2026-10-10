import { afterEach, expect, it, vi } from 'vitest';
import { cleanup, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { QueryClientProvider } from '@tanstack/react-query';
import { createMemoryRouter, RouterProvider } from 'react-router';
import { ConnectionDetails } from './ConnectionDetails';
import { Providers } from './Providers';
import { queryClient, useAuth, type Connection, type Identity } from '../../shared/api/client';
import '../../app/i18n';
const identity: Identity = { subject: 'provider-test-human', installation_id: 'c47b4c36-5132-42bc-af03-5e581b90a3ce', capabilities: ['config.read', 'config.write'], project_grants: ['installation'] };
const connection: Connection = { id: '12259f65-c6ef-42a2-b031-5228cd0376c9', provider_kind: 'openai_compatible', display_name: 'Свой аккаунт', generation: 1, status: 'authorization_unknown', has_credentials: false, catalog_refresh_supported: true, quota: null, version: 1, settings: { display_name: 'Свой аккаунт', endpoint_policy_ref: 'openrouter-api-v1', billing_mode: 'metered' } };
const json = (value: unknown, status = 200) => new Response(JSON.stringify(value), { status, headers: { 'Content-Type': 'application/json' } });
function mount(id = connection.id) { useAuth.getState().setAuth({ token: 'fixture-provider-bearer', userId: identity.subject, email: 'fixture@example.test', displayName: 'Fixture' }); return render(<QueryClientProvider client={queryClient}><RouterProvider router={createMemoryRouter([{ path: '/providers/:id', element: <ConnectionDetails identity={identity}/> }], { initialEntries: [`/providers/${id}?provider_tab=connection`] })}/></QueryClientProvider>); }
afterEach(() => { cleanup(); vi.unstubAllGlobals(); useAuth.getState().logout(); queryClient.clear(); sessionStorage.clear(); });
it('disable requires confirmation and lost reply resolves the original operation without another DELETE', async () => {
    let writes = 0;
    let key = '';
    let current = connection;
    vi.stubGlobal('fetch', vi.fn(async (url: string, init?: RequestInit) => {
        if (init?.method === 'DELETE') {
            writes++;
            key = new Headers(init.headers).get('Idempotency-Key')!;
            expect(new Headers(init.headers).get('If-Match')).toBe('"1"');
            current = { ...connection, status: 'disabled', version: 2 };
            throw new TypeError('fixture disable committed, reply lost');
        }
        if (String(url).includes('/operations/by-key/'))
            return json({ idempotency_key: key, action: 'connection.disable', operation: { id: crypto.randomUUID(), resource_id: connection.id, status: 'succeeded', safe_error: null, version: 2 } });
        if (String(url).includes('/connection-presets'))
            return json({ items: [], next_cursor: null });
        if (String(url) === '/version')
            return json({ external_calls: false });
        return json(current);
    }));
    const user = userEvent.setup();
    mount();
    await user.click(await screen.findByRole('button', { name: 'Отключить подключение' }));
    expect(writes).toBe(0);
    await user.click(screen.getByRole('button', { name: 'Подтвердить отключение' }));
    await screen.findByText(/Результат операции пока неизвестен/);
    await user.click(screen.getByRole('button', { name: 'Проверить исходную операцию' }));
    await screen.findByText('Подключение отключено; история сохранена.');
    expect(writes).toBe(1);
    expect(sessionStorage.length).toBe(0);
    expect(screen.queryByRole('button', { name: 'Отключить подключение' })).toBeNull();
}, 10000);
it('configured model context starts absent and CAS zero creates only the exact model', async () => {
    const model = 'vendor/CaseSensitive-model';
    let key = '';
    let saved: unknown = null;
    const versions: string[] = [];
    vi.stubGlobal('fetch', vi.fn(async (url: string, init?: RequestInit) => {
        if (String(url).includes('/model-contexts')) {
            if (init?.method === 'PUT') {
                const headers = new Headers(init.headers);
                key = headers.get('Idempotency-Key')!;
                versions.push(headers.get('If-Match')!);
                const input = JSON.parse(String(init.body));
                saved = { connection_id: connection.id, ...input, version: 1, updated_at: '2026-10-10T00:00:00Z' };
                return json(saved);
            }
            return new Response(JSON.stringify({ items: saved ? [saved] : [], next_cursor: null }), { headers: { 'Content-Type': 'application/json', 'ETag': saved ? '"1"' : '"0"' } });
        }
        if (String(url).includes('/operations/by-key/'))
            return json({ idempotency_key: key, action: 'model-context.write', operation: { id: crypto.randomUUID(), resource_id: connection.id, status: 'succeeded', safe_error: null, version: 2 } });
        if (String(url).includes('/models'))
            return json({ connection_id: connection.id, generation: 1, models: [{ provider_model_id: model, input_limit: 128000, output_limit: null, capabilities: [], evidence_status: 'unverified', observed_at: '2026-10-10T00:00:00Z' }], data_status: 'complete', as_of: '2026-10-10T00:00:00Z', next_cursor: null });
        if (String(url).includes('/connection-presets'))
            return json({ items: [], next_cursor: null });
        if (String(url) === '/version')
            return json({ external_calls: false });
        return json(connection);
    }));
    const user = userEvent.setup();
    mount();
    await user.click(await screen.findByRole('tab', { name: 'Каталог' }));
    await user.click(await screen.findByRole('button', { name: 'Настроить контекст' }));
    const input = await screen.findByLabelText('Контекст, токены', { exact: false }) as HTMLInputElement;
    expect(input.value).toBe('');
    await user.type(input, '4096');
    await user.click(screen.getByRole('button', { name: 'Сохранить контекст' }));
    await screen.findByText(/Контекст модели сохранён/);
    expect(versions).toEqual(['"0"']);
    expect(saved).toMatchObject({ model_id: model, context_window_tokens: 4096 });
    expect(sessionStorage.length).toBe(0);
}, 10000);
it('context 412 preserves tokens and rebases only the selected model version', async () => {
    const model = 'vendor/CaseSensitive-model';
    let current = { connection_id: connection.id, model_id: model, context_window_tokens: 4096, version: 1, updated_at: '2026-10-10T00:00:00Z' };
    const writes: {
        key: string;
        version: string;
        tokens: number;
    }[] = [];
    let key = '';
    vi.stubGlobal('fetch', vi.fn(async (url: string, init?: RequestInit) => {
        if (String(url).includes('/model-contexts')) {
            if (init?.method === 'PUT') {
                const headers = new Headers(init.headers);
                key = headers.get('Idempotency-Key')!;
                const input = JSON.parse(String(init.body));
                writes.push({ key, version: headers.get('If-Match')!, tokens: input.context_window_tokens });
                if (writes.length === 1) {
                    current = { ...current, version: 2, context_window_tokens: 5120 };
                    return json({ error: { code: 'precondition_failed', message: 'fixture changed context' } }, 412);
                }
                current = { ...current, version: 3, context_window_tokens: input.context_window_tokens };
                return json(current);
            }
            return new Response(JSON.stringify({ items: [current], next_cursor: null }), { headers: { 'Content-Type': 'application/json', 'ETag': `"${current.version}"` } });
        }
        if (String(url).includes('/operations/by-key/'))
            return json({ idempotency_key: key, action: 'model-context.write', operation: { id: crypto.randomUUID(), resource_id: connection.id, status: 'succeeded', safe_error: null, version: 2 } });
        if (String(url).includes('/models'))
            return json({ connection_id: connection.id, generation: 1, models: [{ provider_model_id: model, input_limit: null, output_limit: null, capabilities: [], evidence_status: 'unverified', observed_at: '2026-10-10T00:00:00Z' }], data_status: 'complete', as_of: '2026-10-10T00:00:00Z', next_cursor: null });
        if (String(url).includes('/connection-presets'))
            return json({ items: [], next_cursor: null });
        if (String(url) === '/version')
            return json({ external_calls: false });
        return json(connection);
    }));
    const user = userEvent.setup();
    mount();
    await user.click(await screen.findByRole('tab', { name: 'Каталог' }));
    await user.click(await screen.findByRole('button', { name: 'Настроить контекст' }));
    const input = await screen.findByLabelText('Контекст, токены', { exact: false }) as HTMLInputElement;
    await waitFor(() => expect(input.value).toBe('4096'));
    await user.clear(input);
    await user.type(input, '8192');
    await user.click(screen.getByRole('button', { name: 'Сохранить контекст' }));
    await screen.findByText(/Подключение изменено/);
    expect(input.value).toBe('8192');
    await user.click(screen.getByRole('button', { name: 'Загрузить версию контекста' }));
    await screen.findByText(/Актуальная версия контекста 2/);
    expect(input.value).toBe('8192');
    await user.click(screen.getByRole('button', { name: 'Сохранить контекст' }));
    await screen.findByText(/Контекст модели сохранён/);
    expect(writes.map(w => w.version)).toEqual(['"1"', '"2"']);
    expect(writes[0].key).not.toBe(writes[1].key);
    expect(writes[1].tokens).toBe(8192);
}, 10000);
it('lost credential reply reloads only a safe key intent and resolves original operation without repeating PUT', async () => {
    let writes = 0;
    let lookup = 0;
    let key = '';
    vi.stubGlobal('fetch', vi.fn(async (url: string, init?: RequestInit) => {
        if (init?.method === 'PUT') {
            writes++;
            key = new Headers(init.headers).get('Idempotency-Key')!;
            throw new TypeError('fixture reply lost after commit');
        }
        if (String(url).includes('/operations/by-key/')) {
            lookup++;
            if (lookup === 1)
                return json({ error: { code: 'permission_denied', message: 'fixture lost access' } }, 403);
            return json({ idempotency_key: key, action: 'credential.write', operation: { id: crypto.randomUUID(), resource_id: connection.id, status: 'succeeded', safe_error: null, version: 2 } });
        }
        if (String(url).includes('/models'))
            return json({ connection_id: connection.id, generation: 1, models: [], data_status: 'partial', as_of: '2026-10-10T00:00:00Z', next_cursor: null });
        if (String(url) === '/version')
            return json({ external_calls: false });
        if (String(url).includes('/connection-presets'))
            return json({ items: [{ policy_ref: 'openrouter-api-v1', provider_kind: 'openai_compatible', base_url: 'https://openrouter.ai/api/v1/', allow_loopback: false }], next_cursor: null });
        return json(connection);
    }));
    const user = userEvent.setup();
    mount(connection.id.toUpperCase());
    await screen.findByRole('heading', { name: 'Ключ подключения' });
    await user.type(screen.getByLabelText('API-ключ', { exact: false }), 'private-synthetic-secret-canary');
    await user.click(screen.getByRole('button', { name: 'Сохранить ключ' }));
    await screen.findByText(/Результат операции пока неизвестен/);
    expect(Object.values(sessionStorage).join('')).not.toContain('private-synthetic-secret-canary');
    expect(Object.values(sessionStorage).join('')).not.toContain('fixture-provider-bearer');
    expect(Object.values(localStorage).join('')).not.toContain('private-synthetic-secret-canary');
    expect(Object.values(localStorage).join('')).not.toContain('fixture-provider-bearer');
    expect((screen.getByLabelText('API-ключ', { exact: false }) as HTMLInputElement).value).toBe('');
    cleanup();
    mount(connection.id.toUpperCase());
    await user.click(await screen.findByRole('button', { name: 'Проверить исходную операцию' }));
    await screen.findByText(/fixture lost access/);
    expect(sessionStorage.length).toBe(1);
    await user.click(screen.getByRole('button', { name: 'Проверить исходную операцию' }));
    await screen.findByText(/Ключ сохранён/);
    expect(writes).toBe(1);
    expect(lookup).toBe(2);
    expect(sessionStorage.length).toBe(0);
}, 10000);
it('catalog is cached, query lives in the URL, and disabled external calls do not dispatch refresh', async () => {
    const urls: string[] = [];
    let writes = 0;
    vi.stubGlobal('fetch', vi.fn(async (url: string, init?: RequestInit) => {
        urls.push(String(url));
        if (init?.method && init.method !== 'GET')
            writes++;
        if (String(url).includes('/models'))
            return json({ connection_id: connection.id, generation: 1, models: [{ provider_model_id: 'vendor/model-alpha', input_limit: 4096, output_limit: null, capabilities: ['text'], evidence_status: 'unverified', observed_at: '2026-10-10T00:00:00Z' }], data_status: 'complete', as_of: '2026-10-10T00:00:00Z', next_cursor: null });
        if (String(url).includes('/connection-presets'))
            return json({ items: [], next_cursor: null });
        if (String(url) === '/version')
            return json({ external_calls: false });
        return json({ ...connection, has_credentials: true });
    }));
    const user = userEvent.setup();
    mount();
    await user.click(await screen.findByRole('tab', { name: 'Каталог' }));
    await screen.findByRole('heading', { name: 'vendor/model-alpha' });
    const refresh = screen.getByRole('button', { name: 'Обновить каталог' }) as HTMLButtonElement;
    expect(refresh.disabled).toBe(true);
    await user.click(refresh);
    expect(writes).toBe(0);
    await user.type(screen.getByLabelText('Поиск модели'), 'alpha%');
    expect(urls.some(url => new URL(url, 'http://fixture.test').searchParams.get('q') === 'alpha%')).toBe(true);
    await screen.findByText('Внешние вызовы выключены.');
    expect(writes).toBe(0);
}, 10000);
it('creation selects an owned preset and opens authorization after confirmed operation', async () => {
    let key = '';
    let body: unknown;
    vi.stubGlobal('fetch', vi.fn(async (url: string, init?: RequestInit) => {
        if (init?.method === 'POST') {
            key = new Headers(init.headers).get('Idempotency-Key')!;
            body = JSON.parse(String(init.body));
            return json(connection, 201);
        }
        if (String(url).includes('/operations/by-key/'))
            return json({ idempotency_key: key, action: 'connection.create', operation: { id: crypto.randomUUID(), resource_id: connection.id, status: 'succeeded', safe_error: null, version: 2 } });
        if (String(url).includes('/connection-presets'))
            return json({ items: [{ policy_ref: 'openrouter-api-v1', provider_kind: 'openai_compatible', base_url: 'https://openrouter.ai/api/v1/', allow_loopback: false }], next_cursor: null });
        if (String(url).startsWith('/api/v1/connections?'))
            return json({ items: [], next_cursor: null });
        if (String(url) === '/version')
            return json({ external_calls: false });
        return json(connection);
    }));
    useAuth.getState().setAuth({ token: 'fixture-provider-bearer', userId: identity.subject, email: 'fixture@example.test', displayName: 'Fixture' });
    const router = createMemoryRouter([{ path: '/providers', element: <Providers identity={identity}/> }, { path: '/providers/:id', element: <ConnectionDetails identity={identity}/> }], { initialEntries: ['/providers'] });
    render(<QueryClientProvider client={queryClient}><RouterProvider router={router}/></QueryClientProvider>);
    const user = userEvent.setup();
    await user.click(await screen.findByRole('button', { name: 'Добавить подключение' }));
    await user.type(screen.getByLabelText('Название подключения', { exact: false }), 'Свой аккаунт');
    await user.selectOptions(screen.getByLabelText('Endpoint', { exact: false }), 'openrouter-api-v1');
    await user.selectOptions(screen.getByLabelText('Способ учёта', { exact: false }), 'metered');
    await user.click(screen.getByRole('button', { name: 'Создать' }));
    await screen.findByRole('heading', { name: 'Ключ подключения' });
    expect(router.state.location.pathname).toBe(`/providers/${connection.id}`);
    expect(router.state.location.search).toContain('provider_tab=connection');
    expect(body).toEqual(connection.settings);
    expect(sessionStorage.length).toBe(0);
}, 10000);
it('an absent lookup keeps mutation locked until the server fences the original key', async () => {
    const writes: string[] = [];
    let key = '';
    let closed = false;
    vi.stubGlobal('fetch', vi.fn(async (url: string, init?: RequestInit) => {
        if (init?.method === 'PUT') {
            key = new Headers(init.headers).get('Idempotency-Key')!;
            writes.push(key);
            throw new TypeError('fixture original request not received');
        }
        if (String(url).endsWith('/close-unstarted')) {
            closed = true;
            return json({ idempotency_key: key, action: 'operation.close-unstarted', operation: { id: crypto.randomUUID(), resource_id: null, status: 'cancelled', safe_error: 'Запрос закрыт до начала выполнения', version: 1 } });
        }
        if (String(url).includes('/operations/by-key/'))
            return json({ error: { code: 'not_found', message: 'not found' } }, 404);
        if (String(url).includes('/connection-presets'))
            return json({ items: [], next_cursor: null });
        if (String(url) === '/version')
            return json({ external_calls: false });
        return json(connection);
    }));
    const user = userEvent.setup();
    mount();
    const input = await screen.findByLabelText('API-ключ', { exact: false }) as HTMLInputElement;
    await user.type(input, 'synthetic-unknown-key');
    await user.click(screen.getByRole('button', { name: 'Сохранить ключ' }));
    await screen.findByText(/Результат операции пока неизвестен/);
    await user.click(screen.getByRole('button', { name: 'Проверить исходную операцию' }));
    await screen.findByText(/Исходная операция пока не найдена/);
    expect(input.matches(':disabled')).toBe(true);
    expect(closed).toBe(false);
    expect(writes).toHaveLength(1);
    await user.click(screen.getByRole('button', { name: 'Закрыть непринятый запрос' }));
    await screen.findByText(/Позднее выполнение исходного запроса запрещено/);
    expect(closed).toBe(true);
    expect(input.matches(':disabled')).toBe(false);
    expect(sessionStorage.length).toBe(0);
    expect(writes).toHaveLength(1);
}, 10000);
it('stale connection CAS preserves values and uses a fresh version only after explicit rebase', async () => {
    let current = connection;
    const updates: {
        key: string;
        version: string;
        name: string;
    }[] = [];
    let key = '';
    vi.stubGlobal('fetch', vi.fn(async (url: string, init?: RequestInit) => {
        if (init?.method === 'PATCH') {
            const headers = new Headers(init.headers);
            key = headers.get('Idempotency-Key')!;
            const body = JSON.parse(String(init.body)) as Connection['settings'];
            updates.push({ key, version: headers.get('If-Match')!, name: body.display_name });
            if (updates.length === 1) {
                current = { ...connection, version: 2 };
                return json({ error: { code: 'precondition_failed', message: 'fixture stale' } }, 412);
            }
            current = { ...current, settings: body, display_name: body.display_name, version: 3 };
            return json(current, 201);
        }
        if (String(url).includes('/operations/by-key/'))
            return json({ idempotency_key: key, action: 'connection.update', operation: { id: crypto.randomUUID(), resource_id: connection.id, status: 'succeeded', safe_error: null, version: 2 } });
        if (String(url).includes('/connection-presets'))
            return json({ items: [{ policy_ref: 'openrouter-api-v1', provider_kind: 'openai_compatible', base_url: 'https://openrouter.ai/api/v1/', allow_loopback: false }], next_cursor: null });
        if (String(url) === '/version')
            return json({ external_calls: false });
        return json(current);
    }));
    const user = userEvent.setup();
    mount();
    await user.click(await screen.findByRole('button', { name: 'Изменить настройки' }));
    const name = screen.getByLabelText('Название подключения', { exact: false }) as HTMLInputElement;
    await user.clear(name);
    await user.type(name, 'Новое имя');
    await user.click(screen.getByRole('button', { name: 'Сохранить настройки' }));
    await screen.findByText(/Подключение изменено/);
    expect(name.value).toBe('Новое имя');
    await user.click(screen.getByRole('button', { name: 'Загрузить актуальную версию' }));
    await screen.findByText(/Актуальная версия 2/);
    expect(name.value).toBe('Новое имя');
    await user.click(screen.getByRole('button', { name: 'Сохранить настройки' }));
    await screen.findByText('Настройки сохранены.');
    expect(updates.map(v => v.version)).toEqual(['"1"', '"2"']);
    expect(updates[0].key).not.toBe(updates[1].key);
    expect(updates[1].name).toBe('Новое имя');
    expect(sessionStorage.length).toBe(0);
}, 10000);
