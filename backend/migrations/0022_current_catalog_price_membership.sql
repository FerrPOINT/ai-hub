-- Preserve qualified historical quotes; automatic selection follows current membership.
ALTER VIEW eligible_catalog_prices RENAME TO unit_qualified_catalog_prices;
CREATE VIEW eligible_catalog_prices AS
SELECT p.* FROM unit_qualified_catalog_prices p
JOIN connections c ON c.installation_id=p.installation_id AND c.id=p.connection_id
JOIN connection_generations g ON g.connection_id=c.id AND g.generation=c.generation
WHERE g.catalog_snapshot_id IS NULL OR EXISTS(
    SELECT 1 FROM catalog_snapshots s WHERE s.installation_id=c.installation_id
    AND s.connection_id=c.id AND s.generation=c.generation AND s.id=g.catalog_snapshot_id
    AND s.models @> jsonb_build_array(jsonb_build_object('provider_model_id',p.model_id))
);
