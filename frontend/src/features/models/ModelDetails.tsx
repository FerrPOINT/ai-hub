import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { Link, useLocation, useParams } from 'react-router';
import { Button, Card, DetailPage, ResourceStateView, UnsavedChangesGuard, resourceErrorState } from '@sdlc/ui/ui';
import type { Identity, ProfileInput } from '../../shared/api/client';
import { authGeneration, queryClient } from '../../shared/api/client';
import { allConnections, providerKeys } from '../providers/service';
import { useControlOperation } from '../providers/useOperation';
import { OperationStatus } from '../providers/OperationStatus';
import { modelKeys, profileStatus, readProfile, saveProfile, validateProfile } from './service';
import { ProfileFields } from './ProfileFields';
export function ModelDetails({ identity }: {
    identity: Identity;
}) {
    const id = (useParams().id ?? '').toLowerCase();
    const location = useLocation();
    const readable = identity.capabilities.includes('config.read');
    const writable = identity.capabilities.includes('config.write');
    const operation = useControlOperation(identity, id, () => { setDraft(null); setVersion(null); operation.setNotice('Черновик сохранён. Активная revision не изменена.'); }, 'model');
    const [draft, setDraft] = useState<ProfileInput | null>(operation.initial.intent?.profile ?? null);
    const [version, setVersion] = useState<number | null>(null);
    const profile = useQuery({ queryKey: modelKeys.detail(id), queryFn: ({ signal }) => readProfile(id, signal), enabled: readable });
    const value = profile.isError ? undefined : profile.data;
    const connections = useQuery({ queryKey: providerKeys.list(), queryFn: ({ signal }) => allConnections(signal), enabled: readable || writable });
    async function rebase() {
        const generation = authGeneration();
        try {
            const fresh = await readProfile(id);
            if (generation !== authGeneration())
                return;
            queryClient.setQueryData(modelKeys.detail(id), fresh);
            if (fresh.status === 'archived') {
                operation.setError('Профиль перенесён в архив. Черновик сохранён в форме; запись запрещена.');
                return;
            }
            setVersion(fresh.draft_version);
            operation.setStale(false);
            operation.setNotice(`Актуальная версия ${fresh.draft_version}; ваши значения сохранены.`);
        }
        catch (e) {
            if (generation === authGeneration())
                operation.setError(e instanceof Error ? e.message : 'Не удалось прочитать профиль.');
        }
    }
    async function submit() {
        if (!draft || version === null || operation.stale || operation.locked || value?.status === 'archived')
            return;
        try {
            validateProfile(draft);
            await operation.execute('profile.draft.update', id, key => saveProfile(key, draft, { id, version }), undefined, undefined, draft);
        }
        catch (e) {
            operation.setError(e instanceof Error ? e.message : 'Не удалось сохранить черновик.');
        }
    }
    return <><UnsavedChangesGuard when={!!draft || !!operation.intent}/><DetailPage title={value?.draft.display_name ?? 'Виртуальная модель'} context="Профиль доступа и маршрутизации" actions={writable && value && value.status !== 'archived' && <Button disabled={!!draft || operation.locked} onClick={() => { setDraft(structuredClone(value.draft)); setVersion(value.draft_version); operation.setError(''); }}>Изменить черновик</Button>} state={!readable ? { kind: 'permission-denied' } : profile.isPending ? { kind: 'loading' } : profile.isError ? resourceErrorState(profile.error, () => void profile.refetch()) : { kind: 'ready' }}><Link className="mb-4 inline-block text-accent underline" to={`/models${location.search}`}>Все профили</Link>{value && <Card className="min-w-0 p-5"><dl className="hub-facts text-sm"><dt>Slug</dt><dd className="break-all">{value.draft.slug}</dd><dt>Состояние</dt><dd>{profileStatus(value)}</dd><dt>Draft</dt><dd>v{value.draft_version}</dd><dt>Активная revision</dt><dd className="break-all">{value.active_revision_id ?? 'Не опубликована'}</dd></dl><p className="mt-4 text-text-secondary">{value.draft.mode === 'pinned_test' ? 'Один deployment, одна попытка.' : 'Fallback только в указанном порядке.'}</p><ol className="mt-4 list-inside list-decimal space-y-2">{value.draft.deployments.map((d, i) => <li key={i} className="break-all">{d.model_id} · {connections.data?.find(c => c.id === d.connection_id)?.display_name ?? d.connection_id} · поколение {d.generation}</li>)}</ol></Card>}</DetailPage>{draft && <Card className="mt-5 min-w-0 p-5"><h2 className="mb-4 text-lg font-semibold">Редактирование черновика</h2>{connections.isError && <ResourceStateView state={resourceErrorState(connections.error, () => void connections.refetch())}/>}<form onSubmit={e => { e.preventDefault(); void submit(); }} aria-busy={operation.pending}><ProfileFields value={draft} onChange={setDraft} connections={connections.data ?? []} existing disabled={operation.locked || !writable || value?.status === 'archived'}/><div className="mt-5 flex flex-wrap gap-3">{operation.stale || version === null ? <Button type="button" disabled={operation.locked} onClick={() => void rebase()}>Загрузить версию профиля</Button> : <Button disabled={operation.locked || !writable || value?.status === 'archived'}>Сохранить черновик</Button>}<Button type="button" variant="outline" disabled={operation.locked} onClick={() => { setDraft(null); setVersion(null); }}>Отмена</Button></div></form></Card>}<OperationStatus operation={operation}/></>;
}
