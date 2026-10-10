import { Button, Card } from '@sdlc/ui/ui';
import type { useProviderOperation } from './useOperation';
export function OperationStatus({ operation }: {
    operation: ReturnType<typeof useProviderOperation>;
}) {
    return <>{operation.notice && <p role="status" className="mt-4 text-sm">{operation.notice}</p>}{operation.error && <p role="alert" className="mt-4 text-danger">{operation.error}</p>}{operation.intent && <Card className="mt-4 p-4"><h2 className="font-semibold">Исходная операция</h2><p className="my-2 break-all text-sm text-text-secondary">{operation.intent.key}</p><div className="flex flex-wrap gap-3"><Button disabled={operation.pending} onClick={() => void operation.recover()}>Проверить исходную операцию</Button>{operation.notFound && <Button variant="outline" disabled={operation.pending} onClick={() => void operation.recover(true)}>Закрыть непринятый запрос</Button>}</div><p className="mt-3 text-sm text-text-secondary">Существующий запрос будет прочитан. Новое выполнение с тем же ключом не запускается.</p></Card>}</>;
}
