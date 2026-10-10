import { useRef, useState } from 'react';
import { ApiError } from '@sdlc/ui/lib';
import { authGeneration, queryClient, type Identity, type OperationLookup, type ConnectionInput } from '../../shared/api/client';
import { closeUnstarted, lookupOperation, providerKeys, validateSettings, validateContext } from './service';
import { validateProfile } from '../models/service';
export type ProviderAction = 'connection.create' | 'connection.update' | 'credential.write' | 'credential.revoke' | 'catalog.refresh' | 'model-context.write' | 'connection.disable' | 'profile.draft.create' | 'profile.draft.update';
type SafeIntent = {
    key: string;
    action: ProviderAction;
    resource_id: string | null;
    settings?: ConnectionInput;
    model_context?: import('../../shared/api/client').ModelContextInput;
    profile?: import('../../shared/api/client').ProfileInput;
};
const actions: ProviderAction[] = ['connection.create', 'connection.update', 'credential.write', 'credential.revoke', 'catalog.refresh', 'model-context.write', 'connection.disable', 'profile.draft.create', 'profile.draft.update'];
const uuid = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
const validId = (value: unknown): value is string => typeof value === 'string' && uuid.test(value) && value !== '00000000-0000-0000-0000-000000000000';
function readIntent(key: string): SafeIntent | null {
    const raw = sessionStorage.getItem(key);
    if (!raw)
        return null;
    if (raw.length > 16384)
        throw new Error('Операция во вкладке повреждена.');
    const value = JSON.parse(raw) as SafeIntent;
    if (!validId(value.key) || !actions.includes(value.action) || !(value.resource_id === null || validId(value.resource_id)) || Object.keys(value).some(k => !['key', 'action', 'resource_id', 'settings', 'model_context', 'profile'].includes(k)))
        throw new Error('Операция во вкладке повреждена.');
    if (value.settings) {
        if (!['connection.create', 'connection.update'].includes(value.action) || Object.keys(value.settings).some(k => !['display_name', 'endpoint_policy_ref', 'billing_mode'].includes(k)))
            throw new Error('Операция во вкладке повреждена.');
        validateSettings(value.settings);
    }
    if (value.model_context) {
        if (value.action !== 'model-context.write' || Object.keys(value.model_context).some(k => !['model_id', 'context_window_tokens'].includes(k)))
            throw new Error('Операция во вкладке повреждена.');
        validateContext(value.model_context);
    }
    if (value.profile) {
        if (!['profile.draft.create', 'profile.draft.update'].includes(value.action) || Object.keys(value.profile).some(k => !['slug', 'display_name', 'mode', 'deployments', 'parameters', 'input_limit', 'output_limit', 'context_limit', 'required_capabilities', 'timeout_seconds', 'max_attempts', 'allowed_overrides'].includes(k)) || value.profile.deployments.some(d => Object.keys(d).some(k => !['connection_id', 'generation', 'model_id'].includes(k))) || Object.keys(value.profile.parameters).some(k => !['temperature', 'top_p', 'reasoning_effort'].includes(k)))
            throw new Error('Операция во вкладке повреждена.');
        validateProfile(value.profile);
    }
    return value;
}
export function useControlOperation(identity: Identity, resource: string, onSuccess: (value: OperationLookup) => void, kind: 'provider' | 'model' = 'provider') {
    const storageKey = `aihub.${kind === 'model' ? 'model' : 'provider'}-intent:${identity.installation_id}:${identity.subject}:${resource}`;
    const [initial] = useState(() => {
        try {
            return { intent: readIntent(storageKey), error: '' };
        }
        catch {
            return { intent: null, error: 'Не удалось прочитать операцию вкладки. Новые изменения заблокированы.' };
        }
    });
    const [intent, setIntent] = useState(initial.intent);
    const [pending, setPending] = useState(false);
    const [error, setError] = useState(initial.error);
    const [notice, setNotice] = useState('');
    const [notFound, setNotFound] = useState(false);
    const [stale, setStale] = useState(false);
    const busy = useRef(false);
    const locked = pending || !!intent || !!initial.error;
    function finish(result: OperationLookup, current: SafeIntent) {
        const closed = result.action === 'operation.close-unstarted' && result.operation.status === 'cancelled' && result.operation.resource_id === null;
        if (result.idempotency_key !== current.key || !validId(result.operation.id) || !closed && (result.action !== current.action || current.resource_id !== null && result.operation.resource_id !== current.resource_id))
            throw new Error('Результат относится к другой операции. Исходный запрос сохранён.');
        if (!['succeeded', 'failed', 'cancelled', 'pending', 'unknown'].includes(result.operation.status))
            throw new Error('Неизвестное состояние операции.');
        if (result.operation.status === 'succeeded' && !validId(result.operation.resource_id))
            throw new Error('Не подтверждён объект исходной операции.');
        if (['pending', 'unknown'].includes(result.operation.status)) {
            setError('Результат операции пока неизвестен. Проверьте исходную операцию; новые изменения заблокированы.');
            return;
        }
        sessionStorage.removeItem(storageKey);
        setIntent(null);
        setNotFound(false);
        setStale(false);
        if (result.operation.status === 'succeeded') {
            setError('');
            onSuccess(result);
            void queryClient.invalidateQueries({ queryKey: kind === 'model' ? ['models'] : providerKeys.all });
        }
        else {
            setError(result.operation.safe_error ?? 'Операция не выполнена.');
            setNotice(closed ? 'Позднее выполнение исходного запроса запрещено. Можно создать новую операцию.' : '');
        }
    }
    async function execute(action: ProviderAction, resource_id: string | null, send: (key: string) => Promise<unknown>, settings?: ConnectionInput, model_context?: import('../../shared/api/client').ModelContextInput, profile?: import('../../shared/api/client').ProfileInput) {
        if (locked || busy.current)
            return;
        const current: SafeIntent = { key: crypto.randomUUID(), action, resource_id, ...(settings ? { settings: { display_name: settings.display_name, endpoint_policy_ref: settings.endpoint_policy_ref, billing_mode: settings.billing_mode } } : {}), ...(model_context ? { model_context: { model_id: model_context.model_id, context_window_tokens: model_context.context_window_tokens } } : {}), ...(profile ? { profile } : {}) };
        try {
            sessionStorage.setItem(storageKey, JSON.stringify(current));
            setIntent(current);
        }
        catch {
            setError('Не удалось сохранить идентификатор операции во вкладке. Запрос не отправлен.');
            return;
        }
        busy.current = true;
        setPending(true);
        setError('');
        setNotice('');
        setStale(false);
        const generation = authGeneration();
        let replied = false;
        try {
            await send(current.key);
            replied = true;
            const result = await lookupOperation(current.key);
            if (generation === authGeneration())
                finish(result, current);
        }
        catch (cause) {
            if (generation === authGeneration()) {
                const rejected = !replied && cause instanceof ApiError && cause.status >= 400 && cause.status < 500 && ![408, 429].includes(cause.status);
                if (rejected) {
                    sessionStorage.removeItem(storageKey);
                    setIntent(null);
                    setStale(cause.status === 412);
                    setError(cause.status === 412 ? (kind === 'model' ? 'Конфигурация изменена. Ваши значения сохранены; загрузите актуальную версию.' : 'Подключение изменено. Ваши значения сохранены; загрузите актуальную версию.') : cause.message);
                }
                else
                    setError('Результат операции пока неизвестен. Проверьте исходную операцию; новые изменения заблокированы.');
            }
        }
        finally {
            busy.current = false;
            setPending(false);
        }
    }
    async function recover(close = false) {
        if (!intent || busy.current)
            return;
        busy.current = true;
        setPending(true);
        setError('');
        const generation = authGeneration();
        try {
            const result = await (close ? closeUnstarted(intent.key) : lookupOperation(intent.key));
            if (generation === authGeneration())
                finish(result, intent);
        }
        catch (cause) {
            if (generation === authGeneration()) {
                setNotFound(cause instanceof ApiError && cause.status === 404);
                setError(cause instanceof ApiError && cause.status === 404 ? 'Исходная операция пока не найдена. Это не подтверждает отказ; проверьте ещё раз или закройте непринятый запрос.' : cause instanceof Error ? cause.message : 'Не удалось проверить исходную операцию.');
            }
        }
        finally {
            busy.current = false;
            setPending(false);
        }
    }
    return { intent, initial, pending, locked, error, notice, notFound, stale, setStale, setError, setNotice, execute, recover };
}
export const useProviderOperation = useControlOperation;
