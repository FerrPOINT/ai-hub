import { FormField, Input, Select } from '@sdlc/ui/ui';
import type { ConnectionInput, EndpointPolicyInput } from '../../shared/api/client';
import { kindLabel } from './service';
export function SettingsFields({ value, onChange, presets, disabled, prefix }: {
    value: ConnectionInput;
    onChange: (v: ConnectionInput) => void;
    presets: EndpointPolicyInput[];
    disabled: boolean;
    prefix: string;
}) {
    return <fieldset disabled={disabled} className="grid min-w-0 gap-4 border-0 p-0 md:grid-cols-2"><FormField id={`${prefix}-name`} label="Название подключения" required>{attrs => <Input {...attrs} value={value.display_name} maxLength={120} onChange={e => onChange({ ...value, display_name: e.target.value })}/>}</FormField><FormField id={`${prefix}-endpoint`} label="Endpoint" required hint="Доступны только разрешённые адреса.">{attrs => <Select {...attrs} value={value.endpoint_policy_ref} onChange={e => onChange({ ...value, endpoint_policy_ref: e.target.value })}><option value="">Выберите endpoint</option>{presets.map(p => <option key={p.policy_ref} value={p.policy_ref}>{kindLabel[p.provider_kind]} · {p.base_url}</option>)}</Select>}</FormField><FormField id={`${prefix}-billing`} label="Способ учёта" required hint="Цена и валюта проверяются отдельно.">{attrs => <Select {...attrs} value={value.billing_mode} onChange={e => onChange({ ...value, billing_mode: e.target.value as ConnectionInput['billing_mode'] })}><option value="unknown">Не подтверждён</option><option value="metered">По использованию API</option><option value="subscription">Подписка</option><option value="local">Локальные вычисления</option></Select>}</FormField></fieldset>;
}
