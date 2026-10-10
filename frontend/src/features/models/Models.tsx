import { useEffect, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { Link, useLocation, useNavigate, useSearchParams } from 'react-router';
import { Button, Card, FormField, Input, ListPage, ResourceStateView, Select, UnsavedChangesGuard, resourceErrorState } from '@sdlc/ui/ui';
import type { Identity, ProfileInput } from '../../shared/api/client';
import { allConnections, providerKeys } from '../providers/service';
import { useControlOperation } from '../providers/useOperation';
import { OperationStatus } from '../providers/OperationStatus';
import { allProfiles, emptyProfile, modelKeys, profileStatus, saveProfile, validateProfile } from './service';
import { ProfileFields } from './ProfileFields';
export function Models({ identity }: {
    identity: Identity;
}) {
    const navigate = useNavigate();
    const location = useLocation();
    const [params, setParams] = useSearchParams();
    const readable = identity.capabilities.includes('config.read');
    const writable = identity.capabilities.includes('config.write');
    const [created, setCreated] = useState<string | null>(null);
    const operation = useControlOperation(identity, 'new', result => { setEditing(false); setCreated(result.operation.resource_id ?? null); }, 'model');
    const [editing, setEditing] = useState(!!operation.initial.intent?.profile);
    const [draft, setDraft] = useState<ProfileInput>(operation.initial.intent?.profile ?? emptyProfile());
    useEffect(() => {
        if (created)
            navigate(`/models/${created}${location.search}`);
    }, [created, navigate, location.search]);
    const list = useQuery({ queryKey: modelKeys.list(), queryFn: ({ signal }) => allProfiles(signal), enabled: readable });
    const connections = useQuery({ queryKey: providerKeys.list(), queryFn: ({ signal }) => allConnections(signal), enabled: readable || writable });
    const query = params.get('model_q') ?? '';
    const state = params.get('model_state') ?? '';
    const rows = (list.isError ? [] : list.data ?? []).filter(p => (p.draft.slug.includes(query) || p.draft.display_name.toLowerCase().includes(query.toLowerCase())) && (!state || p.status === state));
    function filter(key: string, value: string) {
        const next = new URLSearchParams(params);
        if (value)
            next.set(key, value);
        else
            next.delete(key);
        setParams(next, { replace: true });
    }
    async function submit() {
        try {
            validateProfile(draft);
            await operation.execute('profile.draft.create', null, key => saveProfile(key, draft), undefined, undefined, draft);
        }
        catch (e) {
            operation.setError(e instanceof Error ? e.message : 'Не удалось сохранить черновик.');
        }
    }
    return <><UnsavedChangesGuard when={editing || !!operation.intent}/><ListPage title="Виртуальные модели" context="Профили доступа и маршрутизации" actions={writable && <Button disabled={editing || operation.locked} onClick={() => { setEditing(true); setDraft(emptyProfile()); }}>Создать профиль</Button>} filters={<><FormField id="model-search" label="Поиск профиля">{attrs => <Input {...attrs} value={query} maxLength={256} onChange={e => filter('model_q', e.target.value)}/>}</FormField><FormField id="model-state" label="Состояние">{attrs => <Select {...attrs} value={state} onChange={e => filter('model_state', e.target.value)}><option value="">Все</option>{['draft', 'active', 'disabled', 'archived'].map(v => <option key={v} value={v}>{({ draft: 'Черновик', active: 'Опубликован', disabled: 'Отключён', archived: 'В архиве' })[v as 'draft']}</option>)}</Select>}</FormField></>} state={!readable ? { kind: 'permission-denied' } : list.isPending ? { kind: 'loading' } : list.isError ? resourceErrorState(list.error, () => void list.refetch()) : rows.length ? { kind: 'ready' } : { kind: 'empty', message: 'Подходящих профилей нет' }}><div className="grid min-w-0 gap-4 md:grid-cols-2 xl:grid-cols-3">{rows.map(p => <Card key={p.id} className="min-w-0 p-5"><Link className="break-all text-lg font-semibold text-accent" to={`/models/${p.id}${location.search}`}>{p.draft.display_name}</Link><p className="my-3 break-all text-sm text-text-secondary">{p.draft.slug}</p><dl className="hub-facts text-sm"><dt>Состояние</dt><dd>{profileStatus(p)}</dd><dt>Draft</dt><dd>v{p.draft_version}</dd><dt>Режим</dt><dd>{p.draft.mode}</dd><dt>Deployments</dt><dd>{p.draft.deployments.length}</dd></dl></Card>)}</div></ListPage>{editing && <Card className="mt-5 min-w-0 p-5"><h2 className="mb-4 text-lg font-semibold">Новый профиль</h2><p className="mb-4 text-sm text-text-secondary">Черновик сохраняется без вызова модели. Для публикации требуется проверка всего профиля.</p>{connections.isError && <ResourceStateView state={resourceErrorState(connections.error, () => void connections.refetch())}/>}<form onSubmit={e => { e.preventDefault(); void submit(); }} aria-busy={operation.pending}><ProfileFields value={draft} onChange={setDraft} connections={connections.data ?? []} disabled={operation.locked || !writable}/><div className="mt-5 flex flex-wrap gap-3"><Button disabled={operation.locked || !writable}>Сохранить черновик</Button><Button type="button" variant="outline" disabled={operation.locked} onClick={() => setEditing(false)}>Отмена</Button></div></form></Card>}<OperationStatus operation={operation}/></>;
}
