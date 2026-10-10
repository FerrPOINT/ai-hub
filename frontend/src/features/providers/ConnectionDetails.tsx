import { useState } from 'react';
import { useInfiniteQuery, useQuery } from '@tanstack/react-query';
import { Link, useLocation, useParams, useSearchParams } from 'react-router';
import { Button, Card, DetailPage, FormField, Input, ResourceStateView, Tabs, TabsList, TabsTrigger, TabsContent, UnsavedChangesGuard, resourceErrorState } from '@sdlc/ui/ui';
import { client, type CatalogPage, type ConnectionInput, type Identity, type Version } from '../../shared/api/client';
import { connectionStatus, kindLabel, listPresets, providerKeys, readCatalog, readConnection, refreshCatalog, revokeCredential, saveConnection, validateSettings, writeCredential } from './service';
import { useProviderOperation } from './useOperation';
import { SettingsFields } from './SettingsFields';
import { OperationStatus } from './OperationStatus';
export function ConnectionDetails({ identity }: {
    identity: Identity;
}) {
    const route = useParams();
    const id = (route.id ?? '').toLowerCase();
    const location = useLocation();
    const [params, setParams] = useSearchParams();
    const readable = identity.capabilities.includes('config.read');
    const writable = identity.capabilities.includes('config.write');
    const [secret, setSecret] = useState('');
    const [draft, setDraft] = useState<ConnectionInput | null>(null);
    const [editVersion, setEditVersion] = useState<number | null>(null);
    const [revoke, setRevoke] = useState(false);
    const operation = useProviderOperation(identity, id, result => { setDraft(null); setEditVersion(null); setRevoke(false); operation.setNotice(result.action === 'credential.write' ? 'Ключ сохранён; требуется проверка аккаунта и модели.' : result.action === 'credential.revoke' ? 'Ключ отозван.' : result.action === 'catalog.refresh' ? 'Каталог обновлён; возможности моделей требуют проверки.' : 'Настройки сохранены.'); });
    const connection = useQuery({ queryKey: providerKeys.detail(id), queryFn: ({ signal }) => readConnection(id, signal), enabled: readable });
    const value = connection.isError ? undefined : connection.data;
    const presets = useQuery({ queryKey: providerKeys.presets(), queryFn: ({ signal }) => listPresets(signal), enabled: readable || writable });
    const version = useQuery({ queryKey: ['version'], queryFn: () => client.get<Version>('/version') });
    const tab = params.get('provider_tab') === 'connection' ? 'connection' : 'catalog';
    const query = params.get('catalog_q') ?? '';
    const catalog = useInfiniteQuery({ queryKey: providerKeys.catalog(id, value?.generation ?? 0, query), initialPageParam: undefined as string | undefined, getNextPageParam: (page: CatalogPage) => page.next_cursor ?? undefined, queryFn: ({ pageParam, signal }) => readCatalog(id, query, pageParam, signal), enabled: !!value && tab === 'catalog' });
    const metadata = catalog.data?.pages[0];
    const models = catalog.isError ? [] : catalog.data?.pages.flatMap(p => p.models) ?? [];
    const effectiveDraft = draft ?? operation.intent?.settings;
    const dirty = !!draft || !!secret || !!operation.intent || revoke;
    function parameter(key: string, input: string) { const next = new URLSearchParams(params); if (input)
        next.set(key, input);
    else
        next.delete(key); setParams(next, { replace: key === 'catalog_q' }); }
    function safeVersion(number: number) { if (!Number.isSafeInteger(number) || number < 1)
        throw new Error('Версия превышает поддерживаемый диапазон.'); return number; }
    function edit() { if (!value)
        return; try {
        setDraft({ ...value.settings });
        setEditVersion(safeVersion(value.version));
        operation.setStale(false);
        operation.setError('');
    }
    catch (error) {
        operation.setError(String(error));
    } }
    async function save() { if (!effectiveDraft || !value || editVersion === null)
        return; try {
        validateSettings(effectiveDraft);
        if (!presets.data?.some(p => p.policy_ref === effectiveDraft.endpoint_policy_ref && p.provider_kind === value.provider_kind))
            throw new Error('Выберите endpoint этого типа подключения.');
        await operation.execute('connection.update', id, key => saveConnection(key, value.provider_kind, effectiveDraft, { id, version: editVersion }), effectiveDraft);
    }
    catch (error) {
        operation.setError(error instanceof Error ? error.message : 'Не удалось сохранить настройки.');
    } }
    async function rebase() { if (!value || operation.locked)
        return; try {
        const latest = await readConnection(id);
        setEditVersion(safeVersion(latest.version));
        operation.setStale(false);
        operation.setNotice(`Актуальная версия ${latest.version}. Ваши значения сохранены в форме.`);
    }
    catch (error) {
        operation.setError(error instanceof Error ? error.message : 'Не удалось прочитать версию.');
    } }
    async function credential() { if (!value || operation.locked)
        return; if (!/^[\x21-\x7e]{1,16384}$/.test(secret)) {
        operation.setError('Введите API-ключ без пробелов и управляющих символов.');
        return;
    } try {
        const generation = safeVersion(value.generation);
        await operation.execute('credential.write', id, key => writeCredential(key, id, { secret, credential_type: 'api_key', expected_generation: generation }));
    }
    catch (error) {
        operation.setError(error instanceof Error ? error.message : 'Не удалось сохранить ключ.');
    }
    finally {
        setSecret('');
    } }
    async function withdraw() { if (!value)
        return; try {
        const current = safeVersion(value.version);
        await operation.execute('credential.revoke', id, key => revokeCredential(key, id, current));
    }
    catch (error) {
        operation.setError(error instanceof Error ? error.message : 'Не удалось отозвать ключ.');
    } }
    async function refresh() { if (!value)
        return; try {
        const generation = safeVersion(value.generation);
        await operation.execute('catalog.refresh', id, key => refreshCatalog(key, id, { expected_generation: generation }));
    }
    catch (error) {
        operation.setError(error instanceof Error ? error.message : 'Не удалось обновить каталог.');
    } }
    const state = !readable ? { kind: 'permission-denied' as const } : connection.isPending ? { kind: 'loading' as const } : connection.isError ? resourceErrorState(connection.error, () => void connection.refetch()) : { kind: 'ready' as const };
    return <><UnsavedChangesGuard when={dirty}/><DetailPage title={value?.display_name ?? 'Подключение'} context={value ? kindLabel[value.provider_kind] : 'Проверка подключения'} actions={<Link className="text-accent underline" to={`/providers${location.search}`}>К подключениям</Link>} state={state} aside={value && <Card className="p-5"><h2 className="mb-4 font-semibold">Состояние подключения</h2><dl className="hub-facts text-sm"><dt>Авторизация</dt><dd>{connectionStatus(value)}</dd><dt>Ключ</dt><dd>{value.has_credentials ? 'Сохранён' : 'Не задан'}</dd><dt>Квота</dt><dd>Не подтверждена</dd><dt>Валюта</dt><dd>Неизвестна</dd><dt>Версия ключа</dt><dd>{value.generation}</dd><dt>Версия настроек</dt><dd>{value.version}</dd></dl><p className="mt-4 text-sm text-text-secondary">Каталог описывает модели. Доступ к inference подтверждается отдельной проверкой.</p></Card>}>
    {value && <><Tabs value={tab} onValueChange={value => parameter('provider_tab', value)}><TabsList aria-label="Раздел подключения"><TabsTrigger value="catalog">Каталог</TabsTrigger><TabsTrigger value="connection">Авторизация и настройки</TabsTrigger></TabsList><TabsContent value="catalog"><Card className="mt-4 p-5"><div className="mb-4 flex flex-wrap items-end gap-4"><FormField id="catalog-search" label="Поиск модели">{attrs => <Input {...attrs} value={query} maxLength={256} onChange={e => parameter('catalog_q', e.target.value)}/>}</FormField>{writable && <Button disabled={operation.locked || !value.has_credentials || !value.catalog_refresh_supported || !["enabled", "authorization_unknown"].includes(value.status) || !version.data?.external_calls} onClick={() => void refresh()}>Обновить каталог</Button>}</div>{version.isError ? <ResourceStateView state={resourceErrorState(version.error, () => void version.refetch())}/> : version.data && !version.data.external_calls && <p className="mb-4 text-sm text-text-secondary">Внешние вызовы выключены.</p>}{!value.catalog_refresh_supported && <p className="mb-4 text-sm text-text-secondary">Обновление каталога для этого подключения пока недоступно.</p>}{catalog.isPending ? <ResourceStateView state={{ kind: 'loading' }}/> : catalog.isError ? <ResourceStateView state={resourceErrorState(catalog.error, () => void catalog.refetch())}/> : <><p className="mb-4 text-sm text-text-secondary">{metadata?.data_status === 'partial' ? 'Каталог ещё не получен' : `${metadata?.data_status === 'stale' ? 'Устаревший каталог' : 'Каталог'} · ${new Date(metadata?.as_of ?? '').toLocaleString('ru-RU')}`}</p>{models.length === 0 ? <ResourceStateView state={{ kind: 'empty', message: query ? 'Модели не найдены' : 'В каталоге пока нет моделей' }}/> : <div className="grid min-w-0 gap-3">{models.map(model => <section key={model.provider_model_id} className="min-w-0 border-b border-border py-3"><h2 className="break-all font-semibold">{model.provider_model_id}</h2><p className="mt-2 text-sm text-text-secondary">Объявленный контекст: {model.input_limit?.toLocaleString('ru-RU') ?? 'Неизвестен'} · Выход: {model.output_limit?.toLocaleString('ru-RU') ?? 'Неизвестен'}</p><p className="mt-2 break-words text-sm text-text-secondary">Возможности требуют проверки · {model.capabilities.length ? model.capabilities.join(', ') : 'Не заявлены'}</p></section>)}</div>}{catalog.hasNextPage && <Button className="mt-4" disabled={catalog.isFetchingNextPage} onClick={() => void catalog.fetchNextPage()}>Показать ещё</Button>}</>}</Card></TabsContent>
    <TabsContent value="connection"><div className="mt-4 grid min-w-0 gap-4"><Card className="p-5"><h2 className="mb-4 text-lg font-semibold">Ключ подключения</h2>{value.provider_kind === 'chatgpt_managed' ? <p>Для ChatGPT требуется собственная managed authorization. Запись API-ключа недоступна.</p> : <form onSubmit={e => { e.preventDefault(); void credential(); }} aria-busy={operation.pending}><fieldset disabled={!writable || operation.locked} className="border-0 p-0"><FormField id="connection-secret" label="API-ключ" required hint="Ключ передаётся один раз и не сохраняется во вкладке.">{attrs => <Input {...attrs} type="password" autoComplete="off" spellCheck={false} autoCapitalize="none" value={secret} maxLength={16384} onChange={e => setSecret(e.target.value)}/>}</FormField><Button className="mt-4" disabled={!secret}>Сохранить ключ</Button></fieldset></form>}{writable && value.has_credentials && <div className="mt-4">{revoke ? <><p className="mb-3 text-sm">Отозвать ключ подключения? История и финансовые записи сохранятся.</p><div className="flex flex-wrap gap-3"><Button disabled={operation.locked} onClick={() => void withdraw()}>Подтвердить отзыв</Button><Button variant="outline" disabled={operation.locked} onClick={() => setRevoke(false)}>Отмена</Button></div></> : <Button variant="outline" disabled={operation.locked} onClick={() => setRevoke(true)}>Отозвать ключ</Button>}</div>}</Card>
    <Card className="p-5"><h2 className="mb-4 text-lg font-semibold">Настройки подключения</h2>{effectiveDraft ? <form aria-busy={operation.pending} onSubmit={e => { e.preventDefault(); void save(); }}><SettingsFields prefix="provider-edit" value={effectiveDraft} onChange={setDraft} presets={(presets.data ?? []).filter(p => p.provider_kind === value.provider_kind)} disabled={operation.locked || !writable}/><div className="mt-4 flex flex-wrap gap-3">{operation.stale ? <Button type="button" disabled={operation.locked} onClick={() => void rebase()}>Загрузить актуальную версию</Button> : <Button disabled={operation.locked || !writable || editVersion === null}>Сохранить настройки</Button>}<Button type="button" variant="outline" disabled={operation.locked} onClick={() => { setDraft(null); setEditVersion(null); operation.setError(''); }}>Отмена</Button></div></form> : <><dl className="hub-facts text-sm"><dt>Endpoint</dt><dd>{presets.data?.find(p => p.policy_ref === value.settings.endpoint_policy_ref)?.base_url ?? 'Не удалось подтвердить адрес'}</dd><dt>Способ учёта</dt><dd>{({ unknown: 'Не подтверждён', metered: 'По использованию API', subscription: 'Подписка', local: 'Локальные вычисления' })[value.settings.billing_mode]}</dd></dl>{writable && <Button className="mt-4" variant="outline" disabled={operation.locked || !presets.data} onClick={edit}>Изменить настройки</Button>}</>}{presets.isError && <ResourceStateView state={resourceErrorState(presets.error, () => void presets.refetch())}/>}</Card></div></TabsContent></Tabs><OperationStatus operation={operation}/></>}
    </DetailPage></>;
}
