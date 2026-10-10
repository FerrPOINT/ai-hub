import { useEffect, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { Button, Card, FormField, Input, ResourceStateView, resourceErrorState } from '@sdlc/ui/ui';
import { readModelContext, saveModelContext, validateContext, providerKeys } from './service';
import type { useProviderOperation } from './useOperation';
export function ModelContextEditor({ connection, model, operation, writable, onClose }: {
    connection: string;
    model: string;
    operation: ReturnType<typeof useProviderOperation>;
    writable: boolean;
    onClose: () => void;
}) {
    const saved = operation.intent?.model_context;
    const [tokens, setTokens] = useState(saved?.model_id === model ? String(saved.context_window_tokens) : '');
    const [version, setVersion] = useState<number | null>(null);
    const [initialized, setInitialized] = useState(false);
    const context = useQuery({ queryKey: providerKeys.context(connection, model), queryFn: ({ signal }) => readModelContext(connection, model, signal) });
    useEffect(() => { if (context.data && !initialized) {
        setVersion(context.data.version);
        if (!saved)
            setTokens(context.data.preference ? String(context.data.preference.context_window_tokens) : '');
        setInitialized(true);
    } }, [context.data, initialized, saved]);
    async function submit() { if (version === null || operation.locked || operation.stale)
        return; try {
        if (!/^[0-9]{1,10}$/.test(tokens))
            throw new Error('Введите целое число токенов.');
        const input = { model_id: model, context_window_tokens: Number(tokens) };
        validateContext(input);
        await operation.execute('model-context.write', connection, key => saveModelContext(key, connection, version, input), undefined, input);
    }
    catch (error) {
        operation.setError(error instanceof Error ? error.message : 'Не удалось сохранить контекст.');
    } }
    async function rebase() { try {
        const fresh = await readModelContext(connection, model);
        setVersion(fresh.version);
        operation.setStale(false);
        operation.setNotice(`Актуальная версия контекста ${fresh.version}. Ваше значение сохранено.`);
    }
    catch (error) {
        operation.setError(error instanceof Error ? error.message : 'Не удалось прочитать контекст.');
    } }
    return <Card className="mt-4 min-w-0 p-5"><h2 className="mb-2 text-lg font-semibold">Контекст модели</h2><p className="mb-4 break-all text-sm text-text-secondary">{model}</p><p className="mb-4 text-sm text-text-secondary">Сохранённый бюджет токенов. Physical limit и возможности модели подтверждаются отдельной проверкой.</p>{context.isPending ? <ResourceStateView state={{ kind: 'loading' }}/> : context.isError ? <ResourceStateView state={resourceErrorState(context.error, () => void context.refetch())}/> : <form onSubmit={e => { e.preventDefault(); void submit(); }} aria-busy={operation.pending}><fieldset disabled={operation.locked || !writable} className="border-0 p-0"><FormField id="model-context-tokens" label="Контекст, токены" required hint={context.data?.preference ? `Сохранено ${context.data.preference.context_window_tokens} · версия ${context.data.version}` : 'Контекст не задан; значение не подставляется автоматически.'}>{attrs => <Input {...attrs} inputMode="numeric" value={tokens} maxLength={10} onChange={e => setTokens(e.target.value)}/>}</FormField></fieldset><div className="mt-4 flex flex-wrap gap-3">{operation.stale ? <Button type="button" disabled={operation.locked} onClick={() => void rebase()}>Загрузить версию контекста</Button> : <Button disabled={operation.locked || !writable || version === null}>Сохранить контекст</Button>}<Button type="button" variant="outline" disabled={operation.locked} onClick={onClose}>Отмена</Button></div></form>}</Card>;
}
