-- 1. Elimina el índice antiguo e inútil que se basaba en 'reliability_score'
DROP INDEX IF EXISTS idi_bnetwork_scan_order;

-- 2. Crea un nuevo índice PARCIAL y ORDENADO.
-- Este índice es mucho más rápido porque:
-- a) Solo incluye los tipos de nodos que realmente escaneamos.
-- b) Está pre-ordenado exactamente como lo pide la consulta (next_attempt_time ASC NULLS FIRST).
CREATE INDEX idi_bnetwork_scan_queue
ON bnetwork (next_attempt_time ASC NULLS FIRST)
WHERE type IN ('ipv4', 'ipv6', 'onionv2', 'onionv3', 'i2p', 'cjdns', 'yggdrasil');