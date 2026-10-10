import { describe, expect, it } from 'vitest'
import { contextFromSearch, namespaceQueryKey, selectedNamespaceUrl } from './namespace'

const registry = '122aa3c1-4cc8-4f2e-9b92-ce54b5f1fa10'
const namespace = '1f987ff0-5732-4d90-89f0-0ff610a9b3b8'
describe('tab-local namespace identity', () => {
  it('preserves unrelated URL state and does not persist context globally', () => {
    const url = selectedNamespaceUrl('/audit', '?tab=history', `${registry}/${namespace}`)
    expect(url).toContain('tab=history')
    expect(contextFromSearch(url.split('?')[1]).namespace).toEqual({ registry_instance_id: registry, namespace_id: namespace })
    expect(namespaceQueryKey('')).toEqual(['all'])
  })
  it('keeps incomplete, duplicate and nil context invalid instead of selecting all', () => {
    for (const search of [`?namespace_id=${namespace}`, `?registry_instance_id=${registry}&namespace_id=${namespace}&namespace_id=${namespace}`, `?registry_instance_id=${registry}&namespace_id=00000000-0000-0000-0000-000000000000`]) {
      expect(contextFromSearch(search).invalid).toBe(true)
      expect(namespaceQueryKey(search)[0]).toBe('invalid')
    }
  })
})
