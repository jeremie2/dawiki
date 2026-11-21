# Autenticazione locale in dawiki

> Documento didattico basato su `src/routes/auth.rs` nella versione con ruoli `user`, `staff` e `admin` (21 novembre 2026).  
> **Scopo:** capire come i componenti collaborano, senza duplicare l'intero sorgente.  
> **Ambito:** prototipo in esecuzione su `127.0.0.1`, non pronto per l'esposizione pubblica.

## 1. Tre concetti diversi

- **Identità:** un utente del wiki è identificato dal suo `users.id`, stabile anche se cambia nickname.
- **Autenticazione:** il server verifica che chi accede conosca la password associata all'account e crea una sessione.
- **Autorizzazione:** il server decide quali operazioni può compiere l'utente. È distinta dal login; i ruoli esistono già, ma non equivalgono ancora a un sistema completo di ACL.

Il modulo `auth.rs` gestisce soprattutto autenticazione e identificazione dell'utente corrente.

## 2. Mappa dei componenti

| Componente | Responsabilità |
|---|---|
| `src/routes/auth.rs` | Registrazione, login, cookie, sessioni, profilo e logout |
| `src/main.rs` | Associa URL e metodi HTTP alle funzioni di `auth.rs` |
| `src/app_state.rs` | Espone il pool SQLite e i template Tera attraverso `AppState` |
| `src/error.rs` | Traduce gli errori applicativi in risposte HTTP |
| `templates/registration.html` | Modulo di registrazione |
| `templates/registration_success.html` | Conferma registrazione |
| `templates/login.html` | Modulo di login |
| `templates/profile.html` | Pagina visibile dopo il login |
| `users` (SQLite) | Identità, email, hash della password, ruolo |
| `user_sessions` (SQLite) | Collegamento utente–sessione, hash del token, scadenza |

**Schema essenziale dei flussi**

```text
REGISTRAZIONE
browser → POST /~registrazione → validazione → Argon2id → INSERT users

LOGIN
browser → POST /~login → SELECT users → verifica Argon2id
        → token casuale → SHA-256(token) → INSERT user_sessions
        → cookie con token al browser

RICHIESTA AUTENTICATA
browser → Cookie → session_token() → SHA-256(token)
        → SELECT user_sessions JOIN users → CurrentUser

LOGOUT
browser → POST /~logout → DELETE user_sessions → cookie scaduto
```

## 3. Strutture e tipi

### `RegistrationForm` e `LoginForm`

Sono strutture con `#[derive(Deserialize)]`. Axum usa `Form<T>` per convertire i campi inviati dal browser in un valore Rust del tipo `T`.

- `RegistrationForm`: `nickname`, `email`, `password`.
- `LoginForm`: `email`, `password`.

I nomi dei campi nei moduli HTML devono corrispondere ai campi delle strutture.

### `UserRole`

L'enum rappresenta i tre ruoli: `User`, `Staff`, `Admin`. `from_db()` traduce le stringhe SQLite (`user`, `staff`, `admin`) in varianti Rust; `as_str()` compie la conversione inversa.

`from_db()` restituisce un errore se trova una stringa inattesa: evita di assegnare silenziosamente un ruolo errato.

### `CurrentUser`

Contiene `id`, `nickname` e `role`. È il risultato della ricerca dell'utente associato alla sessione corrente. Non contiene password o token.

## 4. Le funzioni di `auth.rs`, nell'ordine del file

### `show_registration()`

- **Input:** `State<Arc<AppState>>`, cioè lo stato condiviso dell'applicazione.
- **Azione:** Tera renderizza `registration.html` usando un contesto vuoto.
- **Output:** `Result<Html<String>, AppError>`.

`?` propaga l'eventuale errore del renderer, convertendolo nel tipo di errore applicativo.

### `register()`

1. Estrae il modulo con `Form<RegistrationForm>`.
2. Elimina spazi iniziali/finali da nickname ed email con `trim()`.
3. Valida il nickname: 3–40 caratteri ASCII alfanumerici, `_` o `-`.
4. Esegue una validazione basilare dell'email (lunghezza, spazi, `@`, dominio).
5. Richiede una password tra 12 e 1024 byte.
6. Genera un *salt* casuale con `SaltString::generate(&mut OsRng)`.
7. Calcola l'hash della password con `Argon2::default().hash_password(...)`.
8. Inserisce nickname, email e hash nella tabella `users`.
9. Se l'inserimento riesce, renderizza la conferma con HTTP `201 Created`; se nickname/email sono già presenti, restituisce un errore di conflitto.

**Perché il salt?** Due utenti con la stessa password non devono necessariamente avere lo stesso hash memorizzato. Il salt fa parte del formato dell'hash Argon2; non è una password segreta.

**Nota:** la validazione dell'email è intenzionalmente elementare; non verifica l'esistenza della casella.

### `token_hash()`

Calcola una rappresentazione esadecimale di `SHA-256(token)`. Serve per cercare una sessione nel database senza salvare il token originale.

**Da non confondere con Argon2:** Argon2 è usato per le password, che possono essere deboli e richiedono una funzione costosa; SHA-256 è usato per token di sessione generati casualmente con elevata entropia.

### `session_token()`

- Legge l'header HTTP `Cookie`.
- Divide le coppie `nome=valore` separate da `;`.
- Cerca il cookie `dawiki_session`.
- Accetta solo un token esadecimale di 64 caratteri.
- Restituisce `Option<&str>`: `Some(token)` se lo trova, `None` altrimenti.

Il riferimento `&str` punta ai dati già presenti negli header: non viene creata una nuova stringa per restituire il token.

### `current_user()`

È il ponte fra autenticazione e rotte del wiki.

1. Chiama `session_token()`.
2. Se il cookie manca, restituisce `Ok(None)`.
3. Calcola l'hash del token.
4. Esegue una query con `JOIN` tra `user_sessions` e `users`.
5. Accetta soltanto sessioni con `expires_at > CURRENT_TIMESTAMP`.
6. Converte il ruolo con `UserRole::from_db()`.
7. Restituisce `Ok(Some(CurrentUser))` oppure `Ok(None)`.

La firma `Result<Option<CurrentUser>, AppError>` distingue **tre esiti**:

| Esito | Significato |
|---|---|
| `Ok(Some(user))` | Sessione valida e utente identificato |
| `Ok(None)` | Nessuna sessione valida |
| `Err(error)` | Errore nel database o nei dati |

Il metodo `.transpose()` converte `Option<Result<T, E>>` in `Result<Option<T>, E>`, permettendo di propagare l'errore di conversione del ruolo.

### `show_login()`

Renderizza `login.html` tramite Tera. È analoga a `show_registration()`.

### `login()`

1. Controlla la lunghezza dei dati ricevuti.
2. Cerca `id` e `password_hash` nella tabella `users` usando l'email.
3. Se l'account non esiste o non ha una password locale, risponde con un messaggio generico.
4. Analizza l'hash memorizzato con `PasswordHash::new()`.
5. Verifica la password con `Argon2::default().verify_password(...)`.
6. Genera 32 byte casuali con `OsRng` e li codifica in 64 caratteri esadecimali.
7. Inserisce nella tabella `user_sessions` **l'hash** del token e una scadenza a sette giorni.
8. Invia al browser il token originale in un cookie e reindirizza a `/~profilo`.

Il cookie ha `HttpOnly`, `SameSite=Lax`, `Path=/` e `Max-Age=604800`.

- `HttpOnly`: JavaScript nel browser non può leggere il cookie.
- `SameSite=Lax`: limita alcune modalità di invio del cookie da siti esterni, ma **non sostituisce** una protezione CSRF.
- `Max-Age`: scadenza del cookie nel browser; la scadenza nel database viene controllata separatamente.
- Non è ancora presente `Secure`, perché il prototipo usa HTTP locale; su HTTPS andrà aggiunto.

### `profile()`

Chiama `current_user()`. Se manca una sessione valida, reindirizza a `/~login`. Altrimenti inserisce nickname, ID e ruolo nel contesto Tera e renderizza `profile.html`.

### `logout()`

Se il browser invia un token di sessione, elimina dal database la riga corrispondente al suo hash. In ogni caso risponde con un cookie avente `Max-Age=0` e reindirizza a `/~login`.

Il logout usa `POST`, non `GET`: modifica lo stato della sessione.

## 5. Concetti Rust da ripassare, senza bloccare lo sviluppo

| Concetto | Dove compare | Cosa conviene sapere ora |
|---|---|---|
| `async` / `.await` | Query SQLite, handler | Le operazioni I/O possono attendere senza bloccare l'intero runtime |
| `Result<T, E>` e `?` | Quasi tutti gli handler | Separano il valore valido dall'errore e facilitano la propagazione |
| `Option<T>` | Cookie, sessione, password hash | Rappresenta un valore che può mancare senza essere un errore |
| `Arc<AppState>` | `State(...)` | Consente di condividere lo stato applicativo fra richieste |
| `Deserialize` e `Form<T>` | Moduli HTML | Trasformano campi HTTP in strutture Rust |
| `&str`, `trim()` | Validazione e cookie | Usano viste sui dati senza richiedere sempre nuove allocazioni |
| `match`, `let ... else` | Registrazione e login | Gestiscono esplicitamente casi alternativi |
| `map(...).transpose()` | `current_user()` | Trasforma un risultato opzionale che può fallire |
| `.bind(...)` | Query SQLx | Passa parametri SQL separati dal testo della query |

**Metodo di studio suggerito:** prima capire *cosa entra e cosa esce* da una funzione; solo dopo approfondire i tipi o i metodi che risultano poco chiari.

## 6. Decisioni progettuali importanti

**Identità separata dal login.** Le altre parti del wiki useranno `users.id`, non il nickname o la password. In futuro potremo associare a quell'ID un'identità Ubuntu One.

**Il ruolo si legge dal database.** `current_user()` recupera `u.role` a ogni richiesta autenticata; il cookie non contiene privilegi da considerare attendibili.

**Il token originale non è nel database.** Il browser lo conserva nel cookie; SQLite conserva soltanto il suo SHA-256.

**Le password non sono reversibili.** Il sistema verifica una password candidata contro l'hash Argon2, non decifra una password salvata.

## 7. Limiti attuali e miglioramenti futuri

Questa implementazione è un **prototipo locale**. Prima di renderla pubblica occorrerà almeno:

- Proteggere le richieste che modificano lo stato contro CSRF, incluso il logout.
- Limitare i tentativi di login e prevenire abusi della registrazione.
- Servire il sito tramite HTTPS e usare cookie `Secure`.
- Valutare messaggi di errore e tempi di risposta che non facilitino l'enumerazione degli account.
- Gestire pulizia delle sessioni scadute, eventuale revoca delle sessioni e politiche di sospensione degli utenti.
- Aggiungere test automatici per registrazione, sessioni, ruoli e controlli di accesso.

**Non è necessario implementare tutto adesso:** questa sezione è un promemoria per non confondere un prototipo funzionante con un servizio pronto per Internet.

## 8. Da approfondire quando servirà

- Come Axum costruisce un handler con gli *extractor*.
- Come SQLx gestisce il pool di connessioni SQLite.
- Perché `Option<Result<T, E>>` e `Result<Option<T>, E>` sono due tipi differenti.
- Quali differenze pratiche esistono fra cookie di sessione e autenticazione federata.
- Come progettare test di integrazione per le rotte HTTP.

---

## 9. Manutenzione del documento

Quando cambia `src/routes/auth.rs`, verificare soprattutto:

1. Se sono cambiate firme, responsabilità o risultati delle funzioni.
2. Se sono cambiate query SQL, cookie o durate delle sessioni.
3. Se sono cambiate regole di validazione, ruoli o permessi.
4. Se i limiti di sicurezza descritti sono ancora corretti.

**Convenzione linguistica:** nomi e commenti brevi nel codice Rust in inglese; spiegazioni didattiche nella cartella `docs/` in italiano.
