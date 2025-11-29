-- 1. Cambia el valor por defecto de la columna a NULL.
--    Ahora, las nuevas inserciones de 'batch_upsert' tendrán NULL.
ALTER TABLE bnetwork ALTER COLUMN next_attempt_time SET DEFAULT NULL;

-- 2. Actualiza todos los nodos existentes que aún no hemos escaneado
--    para que tengan NULL, metiéndolos al principio de la cola.
UPDATE bnetwork
SET next_attempt_time = NULL
WHERE scanned IS NULL;