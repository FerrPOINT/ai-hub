import { useQuery } from '@tanstack/react-query';
import { Button, ResourceStateView, resourceErrorState } from '@sdlc/ui/ui';
import { authGeneration, type Connection } from '../../shared/api/client';
import { qualifyAccount, readAccountAuthority } from './service';
import type { useProviderOperation } from './useOperation';
const authorityLabels: Record<string, string> = { active: 'Валюта подтверждена', unqualified: 'Не подтверждено', expired: 'Проверка устарела', invalidated: 'Проверка недействительна' };
export function AccountAuthority({ connection, operation, writable, externalCalls }: {
    connection: Connection;
    operation: ReturnType<typeof useProviderOperation>;
    writable: boolean;
    externalCalls: boolean | null;
}) {
    const account = useQuery({ queryKey: ['providers', 'account-authority', authGeneration(), connection.id, connection.generation], queryFn: ({ signal }) => readAccountAuthority(connection.id, connection.generation, signal), enabled: connection.catalog_refresh_supported });
    const value = account.isError ? null : account.data;
    async function verify() { await operation.execute('account.qualify', connection.id, key => qualifyAccount(key, connection.id, connection.generation)); }
    return <section className="mt-5 min-w-0 border-t border-border pt-4" aria-label="Проверка аккаунта"><h2 className="font-semibold">Проверка аккаунта</h2><p className="my-3 text-sm text-text-secondary">Подтверждает валюту расходов аккаунта. Доступ и возможности моделей проверяются отдельно.</p>{!connection.catalog_refresh_supported ? <p className="text-sm text-text-secondary">Проверка валюты для этого подключения пока не доступна.</p> : <>{account.isPending ? <ResourceStateView state={{ kind: 'loading' }}/> : account.isError ? <ResourceStateView state={resourceErrorState(account.error, () => void account.refetch())}/> : value && <dl className="hub-facts text-sm"><dt>Состояние</dt><dd>{authorityLabels[value.status] ?? value.status}</dd><dt>Валюта расходов</dt><dd>{value.currency ?? 'Не подтверждена'}</dd><dt>Расход по счётчику аккаунта</dt><dd className="break-all">{value.statement_usage !== null && value.statement_usage !== undefined ? `${value.statement_usage} ${value.currency ?? ''}` : 'Неизвестно'}</dd><dt>Действует до</dt><dd>{value.expires_at ? new Date(value.expires_at).toLocaleString('ru-RU') : 'Не задано'}</dd></dl>}{writable && <div className="mt-4 flex flex-wrap gap-2"><Button disabled={operation.locked || operation.stale || externalCalls !== true || !connection.has_credentials || !['enabled', 'authorization_unknown'].includes(connection.status)} onClick={() => void verify()}>Проверить аккаунт</Button><Button variant="outline" disabled={operation.pending} onClick={() => void account.refetch()}>Обновить статус аккаунта</Button></div>}{externalCalls === false && <p className="mt-3 text-sm text-text-secondary">Внешние проверки выключены в конфигурации сервиса.</p>}</>}</section>;
}
