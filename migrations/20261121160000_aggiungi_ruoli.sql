-- Ruoli iniziali del wiki. Tutti gli utenti esistenti diventano utenti normali.
-- L'assegnazione di staff/admin è manuale durante lo sviluppo locale.
ALTER TABLE users
ADD COLUMN role TEXT NOT NULL DEFAULT 'user'
    CHECK (role IN ('user', 'staff', 'admin'));
