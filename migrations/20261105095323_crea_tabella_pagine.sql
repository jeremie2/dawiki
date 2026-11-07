CREATE TABLE pagine_wiki (
    id INTEGER PRIMARY KEY NOT NULL,
    slug TEXT UNIQUE NOT NULL,
    titolo TEXT NOT NULL,'
    contenuto_markdown TEXT NOT NULL,
    contenuto_html TEXT NOT NULL,
    data_creazione DATETIME DEFAULT CURRENT_TIMESTAMP NOT NULL
);

CREATE INDEX idx_pagine_slug ON pagine_wiki(slug);
