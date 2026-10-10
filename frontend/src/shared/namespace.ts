import { parseNamespaceLocation, withNamespaceLocation } from '@sdlc/ui/lib'

export function contextFromSearch(search: string) {
  const params = new URLSearchParams(search)
  const present = params.has('registry_instance_id') || params.has('namespace_id')
  const namespace = parseNamespaceLocation(search)
  return { namespace, invalid: present && namespace === null }
}
export function namespaceQueryKey(search: string) {
  const { namespace, invalid } = contextFromSearch(search)
  if (invalid) return ['invalid', search] as const
  return namespace ? [namespace.registry_instance_id, namespace.namespace_id] as const : ['all'] as const
}
export function selectedNamespaceUrl(path: string, search: string, value: string) {
  const target = new URL(path + search, 'https://relative.invalid')
  const [registry_instance_id, namespace_id] = value.split('/')
  return withNamespaceLocation(target.pathname + target.search, value ? { registry_instance_id, namespace_id } : null)
}
