import { useEffect, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { Link, useLocation, useNavigate, useSearchParams } from 'react-router';
import { Button, Card, FormField, Input, ListPage, ResourceStateView, Select, UnsavedChangesGuard, resourceErrorState } from '@sdlc/ui/ui';
import type { Identity, ConnectionInput } from '../../shared/api/client';
import { allConnections, connectionStatus, kindLabel, listPresets, providerKeys, saveConnection, validateSettings } from './service';
import { useProviderOperation } from './useOperation';
import { OperationStatus } from './OperationStatus';
import { SettingsFields } from './SettingsFields';
export function Providers({ identity }: {
    identity: Identity;
}) {
    const navigate = useNavigate();
    const location = useLocation();
    const [params, setParams] = useSearchParams();
    const writable = identity.capabilities.includes('config.write');
    const readable = identity.capabilities.includes('config.read');
    const [created, setCreated] = useState<string | null>(null);
    const operation = useProviderOperation(identity, 'new', result => { setEditing(false); setCreated(result.operation.resource_id ?? null); });
    const [editing, setEditing] = useState(!!operation.initial.intent?.settings);
    const [draft, setDraft] = useState<ConnectionInput>(operation.initial.intent?.settings ?? { display_name: '', endpoint_policy_ref: '', billing_mode: 'unknown' });
    useEffect(() => { if (created) {
        const search = new URLSearchParams(location.search);
        search.set('provider_tab', 'connection');
        navigate(`/providers/${created}?${search}`);
    } }, [created, navigate, location.search]);
    const list = useQuery({ queryKey: providerKeys.list(), queryFn: ({ signal }) => allConnections(signal), enabled: readable });
    const presets = useQuery({ queryKey: providerKeys.presets(), queryFn: ({ signal }) => listPresets(signal), enabled: readable || writable });
    const query = params.get('provider_q') ?? '';
    const state = params.get('provider_state') ?? '';
    const records = list.isError ? [] : (list.data ?? []).filter(c => (c.display_name.toLowerCase().includes(query.toLowerCase()) || c.id.includes(query)) && (!state || c.status === state));
    function filter(key: string, value: string) { const next = new URLSearchParams(params); if (value)
        next.set(key, value);
    else
        next.delete(key); setParams(next, { replace: true }); }
    async function create() { try {
        validateSettings(draft);
        const preset = presets.data?.find(p => p.policy_ref === draft.endpoint_policy_ref);
        if (!preset)
            throw new Error('Выберите разрешённый endpoint.');
        await operation.execute('connection.create', null, key => saveConnection(key, preset.provider_kind, draft), draft);
    }
    catch (error) {
        operation.setError(error instanceof Error ? error.message : 'Не удалось создать подключение.');
    } }
    return <><UnsavedChangesGuard when={editing || !!operation.intent}/><ListPage title="Провайдеры" context="Подключения и доступ к моделям" actions={writable && <Button disabled={editing || operation.locked || !presets.data?.length} onClick={() => { setEditing(true); operation.setError(''); }}>Добавить подключение</Button>} filters={<><FormField id="provider-search" label="Поиск подключения">{attrs => <Input {...attrs} value={query} maxLength={256} onChange={e => filter('provider_q', e.target.value)}/>}</FormField><FormField id="provider-state" label="Состояние">{attrs => <Select {...attrs} value={state} onChange={e => filter('provider_state', e.target.value)}><option value="">Все</option><option value="enabled">Авторизация подтверждена</option><option value="authorization_unknown">Требует проверки</option><option value="disabled">Отключено</option><option value="revoked">Ключ отозван</option></Select>}</FormField></>} state={!readable ? { kind: 'permission-denied' } : list.isPending ? { kind: 'loading' } : list.isError ? resourceErrorState(list.error, () => void list.refetch()) : records.length ? { kind: 'ready' } : { kind: 'empty', message: query || state ? 'Подходящих подключений нет' : 'Подключений пока нет' }}><div className="grid min-w-0 gap-4 md:grid-cols-2 xl:grid-cols-3">{records.map(c => <Card key={c.id} className="min-w-0 p-5"><Link className="break-all text-lg font-semibold text-accent underline-offset-4 hover:underline" to={`/providers/${c.id}${location.search}`}>{c.display_name}</Link><p className="my-3 text-sm text-text-secondary">{kindLabel[c.provider_kind]}</p><dl className="hub-facts text-sm"><dt>Авторизация</dt><dd>{connectionStatus(c)}</dd><dt>Ключ</dt><dd>{c.has_credentials ? 'Сохранён' : 'Не задан'}</dd><dt>Квота</dt><dd>Не подтверждена</dd><dt>Каталог</dt><dd>Доступен в подключении</dd></dl></Card>)}</div></ListPage>
    {presets.isError && <ResourceStateView state={resourceErrorState(presets.error, () => void presets.refetch())}/>} {presets.data?.length === 0 && writable && <p className="mt-4 text-text-secondary">Разрешённые endpoint ещё не настроены.</p>}
    {editing && <Card className="mt-5 p-5"><form onSubmit={e => { e.preventDefault(); void create(); }} aria-busy={operation.pending}><h2 className="mb-4 text-lg font-semibold">Новое подключение</h2><SettingsFields prefix="provider-create" value={draft} onChange={setDraft} presets={presets.data ?? []} disabled={operation.locked || !writable}/><div className="mt-5 flex flex-wrap gap-3"><Button disabled={operation.locked || !writable}>Создать</Button><Button type="button" variant="outline" disabled={operation.locked} onClick={() => { setEditing(false); operation.setError(''); }}>Отмена</Button></div></form></Card>}<OperationStatus operation={operation}/></>;
}
