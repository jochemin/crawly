-- migrations/20251022090000_initial_schema.sql

-- Habilitamos la extensión para los índices GIN (búsqueda de texto rápida)
CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- ---
-- Tabla principal 'bnetwork'
-- ---
CREATE TABLE IF NOT EXISTS bnetwork (
    added timestamp with time zone,
    detected timestamp with time zone,
    scanned timestamp with time zone,
    soft text,
    services text,
    type text,
    address text NOT NULL PRIMARY KEY,
    port integer,
    incoming boolean,
    notes text,
    country text,
    region text,
    city text,
    isp text,
    asn text,
    latitude real,
    longitude real,
    protocol_version integer,
    start_height integer,
    relay boolean,
    
    -- Columnas con valores por defecto
    reliability_score integer DEFAULT 0,
    consecutive_failures integer DEFAULT 0,
    next_attempt_time timestamp with time zone DEFAULT now()
);

-- ---
-- Tabla de estadísticas 'hourly_stats'
-- ---
CREATE TABLE IF NOT EXISTS hourly_stats (
    snapshot_time timestamp with time zone NOT NULL PRIMARY KEY,
    total_nodes bigint,
    incoming_nodes bigint,
    archive_nodes bigint,
    ipv4_nodes bigint,
    ipv6_nodes bigint,
    onion_nodes bigint,
    top_software jsonb
);

-- ---
-- Índices para optimizar las consultas
-- ---

-- Índice para la consulta principal del crawler (get_nodes_to_scan)
CREATE INDEX IF NOT EXISTS idi_bnetwork_scan_order ON bnetwork (reliability_score DESC, next_attempt_time ASC);

-- Índice para la tarea de enriquecimiento de IP (run_ip_enrichment_task)
CREATE INDEX IF NOT EXISTS idi_bnetwork_ip_country_null ON bnetwork (address, port, type) 
    WHERE (type = ANY (ARRAY['ipv4'::text, 'ipv6'::text])) AND (country IS NULL);

-- Índices para la API y estadísticas
CREATE INDEX IF NOT EXISTS idi_bnetwork_incoming_true ON bnetwork (incoming) WHERE (incoming = true);
CREATE INDEX IF NOT EXISTS idi_bnetwork_type ON bnetwork (type);
CREATE INDEX IF NOT EXISTS idi_bnetwork_soft ON bnetwork (soft);

-- Índices GIN para búsquedas de texto ('LIKE %...%') en 'soft' y 'services'
CREATE INDEX IF NOT EXISTS idi_bnetwork_soft_gin ON bnetwork USING gin (soft gin_trgm_ops);
CREATE INDEX IF NOT EXISTS idi_bnetwork_services_gin ON bnetwork USING gin (services gin_trgm_ops);