# sylvia-iot Security Scan Report

| Scope | Scan date | Baseline |
|---|---|---|
| `general-mq/src` (10 files, about 2,000 lines) | 2026-10-31 | branch `20261023-update-dependencies` (`238dfc9f6`) |
| `sylvia-iot-corelib/src` (9 files, about 1,100 lines) | 2026-11-01 | `main` (`41a9a7f9a`) plus the then-uncommitted general-mq changes |
| `sylvia-iot-auth/src` (55 files, about 8,500 lines) | 2026-11-01 | Same as above |
| `sylvia-iot-coremgr/src` (25 files, about 8,000 lines) | 2026-11-01 | Same as above |
| `sylvia-iot-broker/src` (74 files, about 22,000 lines) | 2026-11-01 | Same as above |
| `sylvia-iot-data/src` (53 files, about 9,400 lines) | 2026-11-01 | Same as above |
| `sylvia-iot-sdk/src` (10 files, about 1,800 lines) | 2026-11-01 | Same as above |
| `sylvia-iot-coremgr-cli/src` (21 files, about 11,600 lines) | 2026-11-01 | Same as above |
| `sylvia-router/src` (19 files, about 3,700 lines) | 2026-11-01 | Same as above |

- Out of scope: `stress-simple`.
- `sylvia-router` is an IP-sharing router project. It is unrelated to IoT itself and is never deployed on K8S, so per the maintainer it is "handled as needed" and not part of the main fix priority.
- Dependency versions: `rumqttc` 0.25.1, `amqprs` 2.1.5, `lapin` 4.12.3, `url` 2.5.8, `tera` 2.4.0.
- `tests/` and `examples/` are out of scope.
- Line numbers in each section refer to the baseline version at scan time. Fixed items keep their original defect description for reference.
- The fixes are on branch `20261122-security-fixes`. This round of security fixes was completed on 2026-11-22, one commit per crate:
  - `d1e5329f2`: general-mq (MQ-1, MQ-3, MQ-4), plus packet size handling in the SDK and the broker.
  - `1a78a3c3f`: corelib (CORE-1 to CORE-4), plus the matching auth changes.
  - `1c7a4999c`: auth (AUTH-1 to AUTH-5, AUTH-7, and the timing and hash concurrency parts of AUTH-6).
  - `c3d32c0ed`: broker (BRK-1, BRK-2, and the broker side of CMGR-1).
  - `a07542aa1`: coremgr (CMGR-2 to CMGR-7, CMGR-10).
  - `a5cda738a`: data (DATA-1).
  - `2d730dff6`: coremgr-cli (CLI-1, CLI-2, plus registering the `coremgr-cli.*` command-line arguments).
- **Numbering:** findings are grouped by crate with the prefixes `MQ` (general-mq), `CORE` (corelib), `AUTH`, `CMGR` (coremgr), `BRK` (broker), `DATA`, `CLI` (coremgr-cli), and `RTR` (router), and sorted by severity within each crate; AUTH-7 and CMGR-10 were added later and come last, out of severity order. A finding belongs to the crate where its root cause is.
- **Permission model:** per the maintainer, unit members can read data and send/receive data. Management operations (create, update, delete) are for the owner only. Admin and manager can do everything. Only admin or manager can add members, and the GUI has no member management yet. The details of member permissions are deferred to the unit member management work in 0.5.
- **rumqttd:** used only for demos and PoCs, so it is out of scope.

## Overview

### Not fixed (deferred, handled as needed, or not applicable)

| # | Severity | Verdict | Issue | Location | Status |
|---|---|---|---|---|---|
| MQ-2 | Medium | CONFIRMED (code trace) | The MQTT receiver has no backpressure, so the number of tasks grows without bound | general-mq | Deferred |
| RTR-1 | Medium | CONFIRMED (code trace) | The router management APIs have no role check. Any user can change the WAN/LAN/WiFi settings and read the PPPoE and WiFi passwords | router | As needed |
| RTR-2 | Medium | PLAUSIBLE (depends on the host firewall) | All router services bind to `[::]`, including the WAN interface, and the repo has no firewall configuration or guidance | router | As needed |
| CMGR-8 | — | Not applicable | The built-in rumqttd allows anonymous connections and has no ACL | coremgr | Won't fix (rumqttd is only for demos/PoCs) |
| CMGR-9 | — | Not applicable | Unit members can publish dldata or uldata through the APIs | coremgr | Won't fix (allowed by design) |

### Fixed

| # | Severity | Issue | Location | Commit |
|---|---|---|---|---|
| CMGR-1 | Critical | The broker APIs return a `hostUri` containing the MQ admin credentials to unit members | coremgr, broker | `c3d32c0ed` |
| CMGR-2 | Critical | coremgr does not enforce the MQ hosts allowlist, so the admin credentials are sent to hosts chosen by tenants | coremgr | `a07542aa1` |
| DATA-1 | Critical | The unit check of the data service can be bypassed with an empty `?unit=`, so any logged-in user can read all tenants' data | data | `a5cda738a` |
| MQ-1 | High | An MQTT message over 10 KiB drops the whole shared connection, a cross-tenant DoS | general-mq | `d1e5329f2` |
| AUTH-1 | High | Reflected XSS on the login page: templates are not HTML-escaped | auth | `1c7a4999c` |
| AUTH-2 | High | Injecting `allow=yes` into the login page `state` skips the consent page and obtains the victim's token | auth | `1c7a4999c` |
| BRK-1 | High | The range check of the range APIs truncates integers, so any logged-in user can hang the broker and exhaust its memory | broker | `c3d32c0ed` |
| CMGR-4 | High | Tenant isolation on EMQX depends on EMQX's security settings. Before 6.3, the defaults effectively allow all access | coremgr, docs | `a07542aa1` (docs) |
| AUTH-3 | Medium-high | Reflected and stored XSS on the grant page | auth | `1c7a4999c` |
| AUTH-4 | Medium-high | Disabled users can keep using their tokens and refresh them indefinitely | auth | `1c7a4999c` |
| CMGR-3 | Medium | Unit members can reset the MQ password of an application/network and take over its MQ account (originally "High"; only admin/manager can add members) | coremgr | `a07542aa1` |
| CMGR-10 | Medium | Unit members can wipe the MQ resources of an application, a network, or even a whole unit through coremgr's DELETE APIs (originally "High", for the same reason as CMGR-3) | coremgr, broker | `a07542aa1` |
| CMGR-5 | Medium | Without logging in, anyone can send huge request bodies and exhaust coremgr's memory | coremgr | `a07542aa1` |
| CORE-1 | Low-medium | Security credentials generated by `random_id_sha()` have too little entropy | corelib, auth | `1a78a3c3f` |
| AUTH-5 | Low-medium | SQLite login sessions never expire, and denying does not delete the session | auth | `1c7a4999c` |
| MQ-3 | Low | When an MQTT URI uses an IPv6 address, the host is replaced with `localhost` | general-mq | `d1e5329f2` |
| MQ-4 | Low | When a URI is malformed, the error message contains the plaintext password | general-mq | `d1e5329f2` |
| CORE-2 | Low | Weak password hashing (PBKDF2 with only 10000 rounds) | corelib | `1a78a3c3f` |
| CORE-3 | Low | The access log records `session_id` | corelib | `1a78a3c3f` |
| CORE-4 | Low | The log4j format does not escape control characters | corelib | `1a78a3c3f` |
| AUTH-6 | Low | Login has no attempt limit, and response times reveal whether an account exists | auth | `1c7a4999c` (partial: no attempt limit) |
| AUTH-7 | Low | Login sessions are not bound to a client and can be used to authorize another client | auth | `1c7a4999c` |
| BRK-2 | Low | dldata-result does not check whether the data belongs to the reporting network | broker | `c3d32c0ed` |
| CMGR-6 | Low | coremgr has no size limit when publishing data through the REST APIs | coremgr, docs | `a07542aa1` |
| CMGR-7 | Low | The MQ resource cleanup is not awaited, so it never runs | coremgr | `a07542aa1` |
| CLI-1 | Low | The token storage file usually has permission `0644`, so other local users can read it | coremgr-cli | `2d730dff6` |
| CLI-2 | Low | The password can only be passed with the `--password` command-line argument | coremgr-cli | `2d730dff6` |

"Critical" is higher than "High".

**Current status:** every item that needs a fix has been fixed. MQ-2 and the AUTH-6 attempt limit are deferred to 0.5, RTR-* are handled as needed, and CMGR-8 and CMGR-9 are not applicable. See "Results and follow-ups" at the end of the report.

## Threat model

- All data from the network is untrusted, including AMQP/MQTT packets, payloads, topics, queue names, and HTTP requests.
- Logged-in low-privilege users try to cross tenant (unit) boundaries. Examples are the owner of a network or application in some unit, or a developer with the DEV role who can create clients.
- Configuration values (URIs, credentials, file paths) may come from user-editable configuration files.
- The services must stay available: panics, unbounded memory growth, and resource exhaustion all count as vulnerabilities.

---

# 1. general-mq

## MQ-1. [High] An MQTT message over 10 KiB drops the whole connection (cross-tenant DoS)

**Verdict: CONFIRMED (PoC)　Status: fixed (`d1e5329f2`)**

### Defect

`create_event_loop()` creates the connection with `RumqttOption::new()` but never calls `set_max_packet_size()`. Both directions therefore use `rumqttc`'s default limit of only 10 KiB (rumqttc `src/lib.rs:516-517`).

- **Inbound:** when a received packet exceeds the limit, `rumqttc` returns `PayloadSizeLimitExceeded` while parsing the fixed header. general-mq swallows the error, then disconnects and reconnects.
- **Outbound:** `AsyncClient::publish()` does not check the size, so `send_msg()` returns `Ok(())`. The event loop only hits `OutgoingPacketTooLarge` when writing the packet. The whole connection drops, and the messages queued on it are lost.

An MQTT connection is shared by multiple queues. When it drops, every queue on it has to reconnect and resubscribe.

### Trigger path in sylvia-iot

1. coremgr's `transfer_host_uri()` (`sylvia-iot-coremgr/src/routes/v1/mod.rs:549`) fills every MQTT host URI with the same EMQX admin credentials. All URIs for the same MQTT broker are therefore identical strings.
2. The broker's connection pool uses the full URI string as the key, so **all tenants on the same MQTT broker share one MQTT connection**. AMQP separates tenants by vhost, each with its own connection, so it is not affected.
3. **Inbound attack:** tenant A publishes a message larger than 10 KiB to its own `uldata` topic. The EMQX default limit is 1 MB, so EMQX forwards it to the broker as usual, and the broker's shared connection drops. Sending large messages continuously keeps the connection down.
4. **Outbound attack:** an application sends a large downlink. When the broker forwards it to the network, the broker drops its own connection.

Even without malice, wrapping data in JSON and converting it to hex doubles its size, so raw data of about 5 KB already exceeds the limit.

### PoC

Environment: Docker Hub was unreachable at the time, so `rabbitmq:4.4.0-management-alpine` with the `rabbitmq_mqtt` plugin was used instead.

**Inbound:** two receiving queues on one shared general-mq connection, and a `rumqttc` client without the 10 KiB limit sending 11 KiB:

```
[shared] conn status: Connected
--- tenant-a publisher (non-general-mq client) sends 11 KiB to its own topic
[shared] conn status: Disconnected
raw publisher: 11264 bytes to tenant-a.uldata acked by broker
  queue tenant-a.uldata status: Connecting
  queue tenant-b.uldata status: Connecting
```

A subscriber with the default 10 KiB limit confirmed the cause of the disconnection: `MqttState(Deserialization(PayloadSizeLimitExceeded(11283)))`.

**Outbound:** `send_msg` returned `Ok(())`, and the connection dropped right after.

### Fix

**general-mq**
- `MqttConnectionOptions` has a new `max_packet_size`, 1 MiB by default, applied to both directions.
- `MqttQueue::send_msg()` computes the packet size before sending and returns `Error::PacketTooLarge` when it is too large, instead of letting the connection drop.
- A new synchronous `Queue::check_msg_size()` lets callers check before `task::spawn`.

**SDK and broker: two layers of checks, with all APIs staying synchronous**
- **Layer 1:** the SDK's `send_uldata()`, `send_dldata()`, `send_dldata_result()`, and the broker's `send_uldata()` and `send_dldata()` call `check_msg_size()` before spawning. Oversized messages now return an error synchronously instead of being silently dropped.
- **Layer 2:** the broker checks received messages against `MAX_DATA_MSG_SIZE = 1 MiB - 16 KiB`.
  - The size is computed after normalizing and re-serializing, because numbers like `1e15` in `extension` become longer when re-serialized.
  - Oversized dldata: a dldata-resp with error code `err_param` and message `"data too large"`.
  - Oversized uldata: logged as a warning and dropped.
  - The reserved 16 KiB is for the fields the broker adds when forwarding: about 300 B are fixed, and the rest is a buffer for code, profile, and network addr, which have no length limit.

**Documentation**
- The size limit is documented in `sylvia-iot-broker/doc/message.md`.
- It is also added to `dev/application.md` and `dev/network.md` in both the English and Chinese books.

**Tests**
- The general-mq, SDK, and broker integration tests all pass.

**Residual risk**
- Fields like code and profile have no length limit, so 16 KiB is enough in practice but not guaranteed.
- If the data channel hits `PacketTooLarge` when sending, the code still nacks. When the application or network side is AMQP, the message is redelivered forever.

### Compatibility

Per the maintainer's decision, this fix adds the `pub` field `MqttConnectionOptions::max_packet_size` and `Error::PacketTooLarge`.

- Neither the struct nor the enum has `#[non_exhaustive]`, so this is a **breaking change** for external users:
  - External code that builds the options with a struct literal without `..Default::default()` fails to compile.
  - External code with an exhaustive `match` on `Error` also fails to compile.
- The crates in this repo all use `..Default::default()`, so they are not affected.
- The version number for the crates.io release needs a separate decision.

## MQ-2. [Medium] The MQTT receiver has no backpressure, so the number of tasks grows without bound

**Verdict: CONFIRMED (code trace; memory exhaustion not tested)　Status: deferred**

### Defect

- `on_publish()` calls `task::spawn` for every received PUBLISH, with no limit.
- `rumqttc` sends PUBACK automatically by default, so the broker keeps sending messages without waiting for the client to finish.
- MQTT's `ack()` and `nack()` do nothing.

### Trigger path

A tenant publishes small messages at a high rate. If the handler takes longer to process a message (it queries the DB for routing) than the interval between messages, tasks keep piling up and eventually exhaust memory.

AMQP does not have this problem, because `prefetch` limits the number of unacked messages.

Also, MQTT acks before processing, so `reliable: true` cannot provide at-least-once delivery.

### Fix direction

- Use `set_manual_acks(true)` and send PUBACK only in `ack()`. This solves both the backpressure and the reliability problems.
- But it changes behavior: without calling `ack()`, no new messages arrive.
- Waiting on a semaphore inside `on_publish()` is not recommended, because it blocks the whole connection's event loop.

## MQ-3. [Low] When an MQTT URI uses an IPv6 address, the host is replaced with `localhost`

**Verdict: CONFIRMED (tested the `url` crate behavior)　Status: fixed (`d1e5329f2`)**

### Defect

`url.domain()` returns `None` for IPv6, and the code falls back to `"localhost"`.

For example, with `mqtt://user:pass@[2001:db8::5]:1883`, the client actually connects to `localhost` and sends the credentials in plaintext.

### Fix

Use `url.host()` and handle each type:

- **MQTT:** wrap IPv6 addresses in brackets for TCP.
- **`mqtts` with IPv6:** return an error in `new()`. rumqttc uses the same string as the TCP address and the TLS server name; the former needs brackets and the latter must not have them.
- **AMQP:** `AMQPUri` has the same IPv6 problem, so it is fixed too by taking the host from `Url`.

### Tests

Added `connect_ipv6`.

## MQ-4. [Low] When a URI is malformed, the error message contains the plaintext password

**Verdict: CONFIRMED (test)　Status: fixed (`d1e5329f2`)**

### Defect

For cannot-be-a-base URIs (for example `mqtt:user:secret@host`), the error message includes the original URI.

- The MQTT problem is in general-mq's own `uri.rs`.
- The AMQP problem is in the third-party `amq-protocol-uri` (lib.rs:189).

### Fix

- MQTT now returns only `Invalid URL`.
- AMQP now checks for cannot-be-a-base with `Url` first. Matching error strings was not used because it is too fragile.

### Tests

`new_wrong_opts` checks that the error message does not contain the password.

## Other general-mq fixes (done at the same time)

- Connection timeout fixes for MQTT and AMQP: rumqttc's timeout is in seconds, and AMQP now has a `time::timeout`.
- Fixed the failing doctest in `lib.rs`: shell commands now use ```` ```sh ```` blocks.

## Not vulnerabilities, but worth noting (deferred)

- **AMQP `reliable` does not wait for publisher confirms:** when the broker rejects or cannot route a message, `send_msg()` still returns `Ok(())`.
- **MQTT cannot match a PUBACK to a specific `send_msg()`:** `rumqttc`'s `publish()` does not return the packet id.
- **The RabbitMQ MQTT plugin is incompatible:** it swaps `.` and `/` in topics, which does not match general-mq's queue names. sylvia-iot uses EMQX or rumqttd, so it is not affected.

## Parts of general-mq checked with no issues found

- **Topic and queue name injection:** the name regex does not allow `+`, `#`, `/`, or `$`.
- **TLS:** both AMQP and MQTT verify the server certificate.
- **Mutex:** no guard is held across `.await`, and the lock order is consistent.
- **Panics:** data received from the network cannot reach any panicking path. There is no `unsafe`.

## Interface issues (to handle in 0.5)

- **Options structs cannot be extended:** all fields are `pub` and there is no `#[non_exhaustive]`. Adding only `#[non_exhaustive]` would stop external code from using even `..Default::default()`, so it must come with a builder.
- **`Error` cannot be extended:** no `#[non_exhaustive]`.
- **The implementation does not match the documented semantics:**
  - MQTT's `ack()` does nothing.
  - `send_msg()` returning `Ok` does not mean the message was delivered.

---

# 2. corelib

## CORE-1. [Low-medium] Security credentials generated by `random_id_sha()` have too little entropy

**Verdict: CONFIRMED (entropy) / online brute force is currently impractical　Status: fixed (`1a78a3c3f`)**

### Defect

`random_id_sha()` generates `sha256("{millisecond timestamp}-{len alphanumeric characters}")` (`sylvia-iot-corelib/src/strings.rs:79`).

SHA-256 adds no entropy, and the timestamp is largely predictable, so the only real entropy is the random part.

| Use | Location | Random length | Entropy | Lifetime |
|---|---|---|---|---|
| login session_id | `auth/src/routes/oauth2/api.rs:201` | 4 | about 2^24 | 60 seconds |
| authorization code | `auth/src/routes/oauth2/primitive.rs:44` | 4 | about 2^24 | 30 seconds |
| access token | `primitive.rs:128` | 8 | about 2^48 | 1 hour |
| refresh token | `primitive.rs:126` | 8 | about 2^48 | 14 days |

### Trigger path

1. Guessing a valid session_id lets the attacker call `POST /authorize` with their own client and obtain an authorization code for the victim's account.
2. Even with the exact millisecond timestamp, this takes about 14.8 million requests within 60 seconds.
3. Each second of timestamp uncertainty multiplies the number of requests by 1000.

### Fix

- auth's authorization codes, access tokens, refresh tokens, and login session IDs now all use `randomstring(64)`.
  - `rand::rng()`, used by `randomstring()`, is ChaCha12, a CSPRNG.
  - Each character has 62 possibilities, so 64 characters give about 381 bits.
- `random_id_sha()` is kept, but its rustdoc now says not to use it for security credentials.

## CORE-2. [Low] Weak password hashing

**Verdict: CONFIRMED　Status: fixed (`1a78a3c3f`)**

### Defect

- PBKDF2-HMAC-SHA256 runs only 10000 rounds (`strings.rs:14`). OWASP recommends 600000 rounds.
- The salt is only 8 characters (`auth/src/routes/v1/user/api.rs:40`).

### Impact

It only matters after a database leak: offline cracking is about 60 times faster.

### Fix

- **New format:** `$pbkdf2-sha256$i=600000$<hex>`. Hashes without the prefix are the old format (10000 rounds) and still verify.
- **New functions:** corelib adds `password_verify()` and `password_need_rehash()`. When a user with an old-format hash logs in successfully, auth rehashes the password and writes it back to the database.
- **Switched to aws-lc-rs:** the original `pbkdf2` crate is a generic implementation that takes about 3.5 seconds per hash in debug builds; aws-lc-rs takes about 0.33 seconds. Every crate depending on corelib already compiles aws-lc-rs, so it adds no build cost.
- **No blocked workers:** auth now hashes inside `spawn_blocking`.
- Hashing even when the account does not exist, and limiting concurrent hashes, are handled in AUTH-6 (`1c7a4999c`).

## CORE-3. [Low] The access log records `session_id`

**Verdict: CONFIRMED　Status: fixed (`1a78a3c3f`)**

### Defect

`LoggerMiddleware` logs the full URI, including the query string (`logger.rs:139,146-148`).

### Trigger path

1. After login, the browser GETs `/auth/oauth2/authorize?…&session_id=…`.
2. The URI matches the log filter `/auth/oauth2/`, so it is logged.
3. Anyone who can read the log can authorize on the victim's behalf with that session_id within 60 seconds.

### Fix

- When the path contains `/oauth2/`, the query keeps only the parameter names, without values. For example `?response_type&session_id`.
- Only the path is checked, not the prefix, so it also works when auth is mounted elsewhere, and it covers the built-in `/auth/oauth2/redirect?code=…` too.
- `FILTER_ONLY` changed from `"/auth/oauth2/"` to `"/oauth2/"`.
- Queries of `/api/` are kept in full, for debugging information such as limit and offset.

## CORE-4. [Low] The log4j format does not escape control characters

**Verdict: CONFIRMED　Status: fixed (`1a78a3c3f`)**

### Defect

`logger.rs:214` only replaces `\n`; `\r` and ESC (`\x1b`) are written as is.

### Trigger path

1. Send `GET /auth/oauth2/login?state=%1b[…`.
2. When parsing fails, `auth/src/routes/oauth2/api.rs:108` logs the raw `state` with `warn!`.
3. When an administrator views the log in a terminal, the terminal executes the ESC sequences. This can forge or hide log content.

The default JSON format escapes control characters, so only the log4j format is affected.

### Fix

The log4j format now uses `str::escape_debug()`:
- Control characters, bidirectional text controls (such as `U+202E`), zero-width spaces, and BOMs are escaped, and the result is always a single line.
- Chinese characters, accented letters, and emoji are kept as is.

## Parts of corelib checked with no issues found

- **`StatusCode::from_u16().unwrap()` in `ErrResp::Custom`:** every caller passes an `ErrReq` constant, so it cannot panic.
- **`escape_regex_str()`:** covers all PCRE special characters outside character classes.
  - With NUL in the input, `CString` fails and the "contains" condition is skipped. The query is still limited to the original unit, so there is no privilege escalation.
- **`is_uri()` accepts `javascript:`:** redirect_uri only appears in a 302 Location header (browsers ignore `javascript:`) and in a hidden input on the grant page. So it is not an attack by itself.
- **`hex_addr_to_u128()` accepts a leading `+`:** the result is always normalized by `u128_to_addr()` afterwards.
- **Others:**
  - Regular expressions use Rust's regex crate, which runs in linear time, so there is no ReDoS.
  - `Json` parse errors are returned as JSON, so they cannot cause XSS.
  - The port range of `server_config` is checked.
  - `role.rs` and `version.rs` have no issues.

---

# 3. auth

## AUTH-1. [High] Reflected XSS on the login page: templates are not HTML-escaped

**Verdict: CONFIRMED (code trace; not tested in a browser)　Status: fixed (`1c7a4999c`)**

### Defect

- tera 2.4.0 only autoescapes templates whose names end with `.html`, `.htm`, or `.xml`. This is decided by `set_templates_auto_escape()` in `finalize_templates()`, confirmed in the source.
- auth registers its templates as `login` and `grant` (`sylvia-iot-auth/src/routes/oauth2/mod.rs:20,30`, `api.rs:45-46`), so **neither template is escaped**.
- Custom template files from the configuration are affected too, because the registered names stay the same.
- `get_login()` puts the raw string of the URL parameter `state` directly into `value="{{state}}"` (`api.rs:125`, `template.rs:40`).

### Trigger path

1. The attacker prepares the inner string: `response_type=code&client_id=<any public client>&redirect_uri=<a URI registered for that client>&state="><script>…</script>`.
2. The attacker URL-encodes it and puts it in `GET /auth/oauth2/login?state=…`.
3. `serde_urlencoded::from_str::<GetAuthRequest>` parses it successfully, because `"`, `<`, and `>` are ordinary characters in form encoding.
4. `check_auth_params()` only checks that the client, redirect_uri, and scope are valid. These values can be taken from any app's login link, so the check passes.
5. The raw string is rendered into the HTML, `"` breaks out of the attribute, and the script runs on auth's domain.

### Impact

- No login is required; the victim only needs to open the link.
- The script runs on the real login page and can read and exfiltrate the account and password on submit. The domain is the official one, so the victim can hardly notice.

### Fix

- The templates are renamed `login.html` and `grant.html`, so tera escapes them automatically. Custom templates are covered too.
- Test: in the grant page output of `GET /authorize`, `"><script>` in `state` is escaped to `&quot;&gt;&lt;script&gt;`.

## AUTH-2. [High] Injecting `allow=yes` into the login page `state` skips the consent page and obtains the victim's token

**Verdict: CONFIRMED (code trace; not tested)　Status: fixed (`1c7a4999c`)**

### Defect

- `GET /authorize` parses its parameters with `Query::<AuthorizationRequest>`, and that struct has an `allow` field (`request.rs`). A GET request with `allow=yes` is therefore **authorized directly** without showing the grant page (`authorize()` in `api.rs`).
- `get_login()` and `post_login()` parse `state` with `GetAuthRequest` for validation, but extra unknown fields are not rejected. After validation, the code still uses the **raw string**:
  - `get_login()` puts the raw string into a hidden input on the login page.
  - `post_login()` appends the raw string directly to the redirect URL: `/authorize?{state}&session_id=…` (`api.rs:210-213`).
- Login sessions are not bound to a client. A session created in any client's login flow can authorize any client.

### Trigger path

1. The attacker has the DEV role and registers their own client EVIL, with a redirect_uri pointing to their own server and arbitrary scopes.
2. The attacker makes the victim open `GET /auth/oauth2/login?state=<URL-encoded response_type=code&client_id=EVIL&redirect_uri=<EVIL's URI>&allow=yes>`.
   - The URL is on the official domain, and the login page does not show which client it is for.
3. The victim logs in. `post_login()` redirects to `/authorize?response_type=code&client_id=EVIL&…&allow=yes&session_id=S`.
4. `authorize()` validates the client and redirect_uri, and `allowed()` is `Some(true)`, so it issues an authorization code and redirects to the attacker's server.
5. The attacker exchanges it with their own client for the victim's access token and refresh token.

### Impact

- The default `api_scopes` is empty, so the token has the **full API permissions** of the victim's roles. If the victim is an admin, the attacker gets admin permissions.
- The other services validate the token through tokeninfo, so it works on coremgr, broker, and data.
- **Fixing AUTH-1 (template escaping) does not fix this.** Escaping only affects how the HTML is displayed; when the form is submitted, `state` is still the same raw string.

### Fix

- `AuthorizationRequest` always sets `allow` to `None` for GET, so only the grant page's POST can authorize.
- A new `check_login_state()` parses and validates `state`, then re-serializes it with `GetAuthRequest`. `get_login()` and `post_login()` both use the re-serialized string, so unknown fields such as `allow` are dropped.
- Login sessions are bound to a client; see AUTH-7.
- Tests: neither the login page nor `post_login()`'s redirect URL contains `allow=`, and a GET with `allow` still returns the grant page (200).

## AUTH-3. [Medium-high] Reflected and stored XSS on the grant page

**Verdict: CONFIRMED (code trace)　Status: fixed (`1c7a4999c`)**

### Defect

`client_name`, `scope`, and `state` on the grant page are not escaped (`api.rs:319,327,331`, `template.rs:93-103`).

### Trigger path

**a. The OAuth `state` parameter (reflected)**
1. Go through the normal `/oauth2/auth?…&state="><script>…` entry. This step encodes correctly, so the login page is safe.
2. After the victim logs in, `post_login()` redirects to `/authorize?{state}&session_id=…`.
3. After GET decoding, `state` is the raw string again and is output unescaped on the grant page.

**b. The client name (stored)**
1. A client's `name` is not validated at all. A user with the DEV role can register a client whose name contains a script.
2. When a victim goes through the authorization flow for this client, the script runs.
3. The script can read the `session_id` in the page and click consent on the victim's behalf.

### Fix

- After the template rename, `client_name`, `scope`, and `state` on the grant page are all escaped (same as AUTH-1).
- No character restriction was added to the client `name`; escaping is enough.

## AUTH-4. [Medium-high] Disabled users can keep using their tokens and refresh them indefinitely

**Verdict: CONFIRMED (code trace)　Status: fixed (`1c7a4999c`)**

### Defect

- **Disabling does not revoke tokens:** `patch_admin_user()` only sets `disabled_at` for `disable: true`. `remove_tokens()` is only called when the password changes (`routes/v1/user/api.rs:384`).
- **Validation does not check the status:** the middleware queries the user but only checks roles and scopes, not `disabled_at` or `expired_at` (`routes/oauth2/middleware.rs:313-353`).
- **Other services cannot see the status either:** tokeninfo does not return the disabled status (`routes/v1/auth/api.rs`).
- **Refresh does not check the user:** `recover_refresh()` and `issue()` never look up the user, and each refresh issues a new refresh token valid for 14 days (`routes/oauth2/primitive.rs`).

### Trigger path

1. An admin disables account X, for example a former employee or a stolen account.
2. X's existing access token still passes the middleware and tokeninfo, so every service still works.
3. By calling `/oauth2/refresh` regularly, X keeps access **indefinitely**.

`expired_at` behaves the same: after an account expires it can no longer log in, but tokens already issued are not affected.

### Fix

- `patch_admin_user()` calls `remove_tokens()` when the password changes or the user is disabled (`disabled_at` set).
- After looking up the user, the middleware checks `disabled_at` and `expired_at` and returns 401 if either applies. tokeninfo goes through this middleware too.
- `recover_refresh()` looks up the user and returns `invalid_grant` when the user does not exist, is disabled, or has expired.
- Tests: when disabled or expired, tokeninfo returns 401 and refresh returns `invalid_grant`, and both work again after re-enabling; after disabling through the API, old tokens stay invalid after re-enabling (revoked).

## AUTH-5. [Low-medium] SQLite login sessions never expire, and denying does not delete the session

**Verdict: CONFIRMED (code trace)　Status: fixed (`1c7a4999c`)**

### Defect

- **No expiry check:** SQLite's `login_session.get()` does not check `expires_at` (`models/sqlite/login_session.rs:52-74`), and neither does `authorize()`. SQLite has no TTL mechanism, so a session is only deleted when it is used.
- **Denying does not delete:** when the user clicks "deny", the code goes through `pending.deny()` and the session is not deleted. Sessions whose grant page was shown but never submitted also stay.
- MongoDB has a TTL index on `createdAt` that cleans up after about 2–3 minutes, so it is not affected.

### Impact

On SQLite deployments, the time window of these two issues grows from 60 seconds to **forever**:

- CORE-3: anyone who can read the access log can get a `session_id`.
- CORE-1: a `session_id` has only about 2^24 of entropy, and unused sessions keep accumulating.

### Fix

- `authorize()` deletes the session right after taking it, then checks `expires_at` and rejects expired ones.
- Clicking "deny" also deletes the session first.
- SQLite's `login_session.add()` deletes expired sessions first, so no separate schedule is needed.
- **Compatibility:** SQLite's `login_session` table has two new `NOT NULL` columns (see AUTH-7), and `init()` only runs `CREATE TABLE IF NOT EXISTS`. Existing SQLite databases need the `login_session` table dropped manually.

## AUTH-6. [Low] Login has no attempt limit, and response times reveal whether an account exists

**Verdict: CONFIRMED (code trace)　Status: partially fixed (`1c7a4999c`; no attempt limit)**

### Defect

- `post_login()` has no attempt limit or lockout, so passwords can be brute-forced online.
- When the account does not exist, it returns an error immediately; when it exists, it runs PBKDF2 first (`routes/oauth2/api.rs`). The response times differ, so timing reveals whether an account exists.
- **CORE-2 makes the gap larger:** with rounds raised from 10000 to 600000, the time difference grows from a few milliseconds to around a hundred milliseconds, making account detection easier.
- **CPU usage:** hashing already runs in `spawn_blocking`, so tokio workers are not blocked. But a flood of login requests makes the blocking pool run many hashes in parallel and saturates the CPU.

### Fix

- New `libs/password.rs`:
  - `hash()` and `verify()` run in `spawn_blocking`, with a `Semaphore` limiting concurrent hashes to the number of CPUs.
  - When the account does not exist, a dummy hash is verified once so the response time is the same.
- `post_login()` verifies the password first and checks disabled/expired afterwards, so these states cannot be seen from the response time either.
- **Not handled:** attempt limits per account or IP.

## AUTH-7. [Low] Login sessions are not bound to a client and can be used to authorize another client

**Verdict: CONFIRMED (code trace)　Status: fixed (`1c7a4999c`)**

### Defect

- A login session only records `user_id`. `authorize()` only checks that the session exists, not which client's login flow created it.

### Trigger path

1. The attacker obtains a `session_id` created in the victim's login flow for client A, for example from the access log (CORE-3) or via XSS on the grant page (AUTH-3).
2. Within the session lifetime, the attacker calls `POST /authorize` for their own client EVIL with this `session_id` and `allow=yes`.
3. The attacker gets the victim's authorization for EVIL.

### Fix

- `LoginSession` has new `client_id` and `redirect_uri` fields, recorded in `post_login()`.
- `authorize()` compares the request's `client_id` and `redirect_uri` with the session and rejects mismatches.
- Tests: a session from a public client cannot be used for a private client, and a session can only be used once.

## Parts of auth checked with no issues found

- **Client APIs:** non-admins can only query, update, and delete their own clients; accessing someone else's returns 404.
- **User API permissions:** managers cannot modify admins, cannot assign the admin or service roles, and cannot disable admins or managers. Only admins can delete users, and not themselves.
- **SQL:** `sql_builder::quote()` doubles single quotes, which is the correct escaping for SQLite. LIKE escapes `\`, `%`, and `_`, and sort fields are fixed enums. MongoDB queries all use typed `doc!`.
- **Basic auth parsing:** the `"Basic "` prefix is checked first, so slicing `[6..]` is safe.
- **Redirects:**
  - The `Location` header value is validated by `HeaderValue`, so CRLF cannot be injected.
  - `post_login()`'s redirect URL always starts with `scope_path`, so it cannot redirect to an external domain.
  - Other redirects must use a redirect_uri registered for the client.
- **Authorization codes:** their lifetime is checked, and they are deleted when taken. In theory two concurrent requests could both get a token, but only if the attacker already has the code.

---

# 4. coremgr

CMGR-1's root cause is in coremgr (it writes the credentials into `hostUri`), but the leak happens through the broker APIs. CMGR-4 involves both coremgr's ACLs and the documentation.

## CMGR-1. [Critical] The broker APIs return a `hostUri` containing the MQ admin credentials to unit members

**Verdict: CONFIRMED (code trace; not tested)　Status: fixed (`c3d32c0ed`, broker side)**

### Defect

1. When coremgr creates or updates an application or network, `transfer_host_uri()` (`sylvia-iot-coremgr/src/routes/v1/mod.rs:549-567`) fills these credentials into `hostUri` before sending it to the broker to store in the database:
   - The RabbitMQ **admin** account and password.
   - The EMQX API key and secret. This key is also created as an MQTT superuser (`mod.rs:430-438`).
2. The broker's `application_transform()` and `network_transform()` put `host_uri` into the response as is (`sylvia-iot-broker/src/routes/v1/application/api.rs:974`, `network/api.rs:1031`).
3. The broker's read permission is `only_owner=false`, so unit members can read it (`check_application()` and `check_network()` in `broker/src/routes/v1/lib.rs`).
4. coremgr's own responses remove the credentials with `trunc_host_uri()`, but the broker APIs are mounted on the same public HTTP service as coremgr in both `sylvia-iot-core` (`coremgr/src/bin/sylvia-iot-core.rs:104`) and `sylvia-router` (`sylvia-router/src/bin/sylvia-router.rs:123`).

### Trigger path

1. Any unit member calls `GET /broker/api/v1/application/list` or `/network/list` with their own token.
2. The `hostUri` in the response contains `amqp://<admin account>:<admin password>@host/...`.
3. With the admin credentials, the attacker can:
   - Log in to the RabbitMQ management UI, or connect to EMQX directly as a superuser.
   - Read and publish data of **all tenants**.
   - Delete other tenants' vhosts or users.

### Fix

- The broker's `application_transform()` and `network_transform()` remove the account and password from `hostUri` with `hide_host_uri_credentials()`, for both GET and list, including for admins. The path (the AMQP vhost) is kept.
- PATCH uses `restore_host_uri_credentials()`: when the submitted `hostUri` has no credentials and the other parts (scheme, host, port, path) match the DB, the DB credentials are reused and treated as unchanged. When the host or vhost differs, they are not reused, so the admin credentials are never sent to a host or vhost chosen by the user.
- coremgr is not affected: it only reads the scheme, host, and port from the broker response, and builds the full URI with credentials from its own configuration when changing hosts.
- Tests: GET and list for 3 roles, and 6 PATCH cases.

### Notes

- Per the maintainer, real deployments put a reverse proxy in front and do not expose the broker APIs directly. This fix makes sure the broker APIs do not leak the admin credentials even if accessed directly.
- **Not handled:** in the long run, the credentials the broker uses to connect to the MQ should live in the broker's own configuration, not in every record.

## CMGR-2. [Critical] The MQ hosts allowlist is not enforced, so the admin credentials are sent to hosts chosen by tenants

**Verdict: CONFIRMED (code trace; no PoC)　Status: fixed (`a07542aa1`)**

### Defect

- The configuration has `coremgr.mq.rabbitmq.hosts` and `coremgr.mq.emqx.hosts` (`sylvia-iot-coremgr/src/libs/config.rs:59-60,72-73`), documented as "available hosts", but **the code never uses them**.
- When creating or updating an application or network, `create_queue_rsc()` (`routes/v1/mod.rs:365`) calls the MQ management APIs on the host the tenant specified in `hostUri`:
  - RabbitMQ: `libs/mq/rabbitmq.rs:110-113` uses `http://{hostname}:15672` with the **admin's Basic auth**.
  - EMQX: the api_key and secret are sent the same way.
- `transfer_host_uri()` (`routes/v1/mod.rs:549`) also fills the admin credentials into the URI.

### Trigger path

1. A unit owner creates or updates an application/network with `POST` or `PATCH`, pointing `hostUri` at a host they control.
2. coremgr sends the MQ admin credentials to this host over plaintext HTTP.
3. With these credentials, the attacker can manage the whole MQ and affect all tenants.

### Fix recommendation

- Check in `POST` and `PATCH` that the host of `hostUri` is in the `hosts` allowlist.
- What to do when the allowlist is not configured (no restriction, or only the default host) needs a separate decision.

### Fix

- Each `hosts` entry is `{name, host, external, active}`:
  - `host` is the host coremgr connects to internally, `external` is the host the unit's clients connect to, and `name` is only for display.
  - `rabbitmq.hosts` applies to amqp/amqps, and `emqx.hosts` to mqtt/mqtts. rumqttd is not supported.
- The APIs take and return the `external` host in `hostUri`. coremgr maps it to the internal `host` before passing it to the broker and the MQ management APIs, and any host not in the list gets `err_param`.
- A host with `active: false` can no longer be used to create an application/network or as the target of a host change, but existing resources are kept.
- New `GET /coremgr/api/v1/mq/hosts`: lists `type`, `name`, `external`, and `active` without exposing the internal hosts.
- The configuration is validated at startup: `name` must not be empty; `host` and `external` must be host names only (no scheme, port, or path), are lowercased, and must be unique; IPv6 addresses get brackets. An invalid configuration stops the service from starting, including invalid JSON in CLI arguments or environment variables.
- An empty allowlist means no restriction, with a warning log. Per the maintainer's decision, this is the operator's choice.
- Known limitation: the port and scheme are shared between internal and external; only the host name is mapped.
- Together with CMGR-3's owner check, unit members can no longer trigger it.

### Notes: member permission is enough to trigger it

- `post_application()` calls `get_unit_inner()`, which only requires read permission.
- `patch_application()` calls `get_application_inner()`, which also only requires read permission.

Both call `create_queue_rsc()` **before** the broker checks write permission, so unit members can trigger CMGR-2 without being the owner.

## CMGR-3. [Medium] Unit members can reset the MQ password of an application/network and take over its MQ account

**Verdict: CONFIRMED (code trace, across coremgr and broker)　Status: fixed (`a07542aa1`)**

### Defect

When the body of `patch_application()` contains only `password`, `ttl`, or `length`:

1. It only calls `get_application_inner()`, the broker's GET, which only requires member permission.
2. It **never calls** the broker's PATCH (which is where the owner check is).
3. It runs `rabbitmq::put_user()` or `emqx::put_user()` directly with the admin credentials, changing the MQ password to the value in the request (`sylvia-iot-coremgr/src/routes/v1/application.rs:680-720`).

`patch_network()` works the same way (`network.rs:700-750`).

### Trigger path

1. A unit member sends `PATCH /coremgr/api/v1/application/{id}` with the body `{"data":{"password":"attacker"}}`.
2. The password of the MQ account `application.<unit>.<code>` is changed.
3. The attacker connects to the MQ with the new password and can:
   - Read this application's uplink data.
   - Send downlinks to devices.
4. The original application, still using the old password, can no longer connect.

### Notes

Per the maintainer, management APIs are for the unit owner only. Resetting the MQ password is a management operation, so this finding holds. How members should be managed is not planned yet.

### Fix recommendation

- Changing the password, ttl, or length must pass the broker's owner check first, for example by calling the broker's PATCH first or adding a permission check API.
- Every call to `create_queue_rsc()` must come after the permission check.

### Fix

- coremgr adds `check_unit_owner()`: admin, manager, and the unit owner can manage MQ resources; public networks (without a unit) are for admin and manager only.
- application/network POST and PATCH check this before creating MQ resources or changing passwords. Non-owner members get 400 `err_param` (message `the user is not the unit owner`). When the unit does not exist or the user is not a member, the responses are unchanged (POST returns `err_broker_unit_not_exist`, PATCH returns 404).
- Tests: the member's POST uses a host without an MQ, confirming the error is returned before any resource is created; the PATCH tests include public networks.

### Severity adjustment and deferred items

- **Severity lowered from "High" to "Medium":** the broker only lets admin and manager change `memberIds` (`get_updates()` in `sylvia-iot-broker/src/routes/v1/unit/api.rs`), and neither owners nor the GUI can add members. So the precondition is "a member added by an admin abuses their permission". Once that holds, the attacker can still take over the MQ account, so it is not lowered to "Low".
- **Member permission semantics are deferred to the unit member management work in 0.5:** non-owners get `err_param` for now, marked `TODO: member management` in the code. The inconsistent broker write API responses to non-owners (POST returns `*_not_exist`, PATCH returns 404, DELETE returns 204 without deleting anything) will be handled then as well.

## CMGR-4. [High] Tenant isolation on EMQX depends on EMQX's security settings; before 6.3, the defaults effectively allow all access

**Verdict: PLAUSIBLE (supported by the comments in EMQX's official configuration; not tested in legacy mode)　Status: fixed (`a07542aa1`, docs)**

### Notes: findings from an EMQX 6.3.0 container (2026-11-01, read-only queries)

- `authorization.no_match` is `deny`. The authorization sources are `file` then `built_in_database`.
- The last rule of EMQX's default `acl.conf` is `{allow, {security_profile, legacy}}`. The official comments say:
  - With `EMQX_SECURITY_PROFILE=legacy` (**the default before EMQX 6.3**), this rule applies and effectively allows all access.
  - With `hardened` (the default from 6.3), this rule does not apply, and `no_match` is used instead.
- The comments also say production deployments should change the last rule to `{deny, all}` and set `no_match = deny`.
- So the impact of this issue is:
  - EMQX 6.3 or later with the default hardened profile: safe.
  - EMQX before 6.3, or explicitly set to legacy: any tenant can access every topic.
- The user documentation `quick.md` pinned **`EMQX_VER=6.0.1`**, which is in the affected range.

It can be tested without authentication: EMQX authorization decides based on the username the client presents. But reproducing the vulnerable case requires switching the container to legacy mode, so it was not tested on the maintainer's container.

### Original analysis

### Defect

- The EMQX ACLs coremgr creates for each application or network contain only **allow** rules, no deny rules (`sylvia-iot-coremgr/src/libs/mq/emqx.rs:238-283`).
- So "tenants cannot access other tenants' topics" depends entirely on EMQX's global setting `authorization.no_match = deny`, with no more permissive ACL source.
- The CI's `.github/workflows/files/emqx.conf:58` sets `no_match = deny`.
- But the user documentation (`documentation/book/src/guide/quick.md`, `configuration.md`) never mentions this setting. quick.md still used EMQX 4 environment variables.
- The default in EMQX 5/6 is `no_match = allow`. The last rule of the default `acl.conf` is `{allow, all}`, which only forbids subscribing to a literal `#`.

### Trigger path (to be confirmed by PoC)

1. An operator deploys EMQX following the documentation without changing `no_match`.
2. A tenant connects to EMQX with their own MQ account and subscribes to `+`. sylvia-iot topics are all single-level names (separated by `.`, without `/`), so `+` matches every topic.
3. The tenant receives all tenants' uldata and dldata. They can also publish to other tenants' topics and impersonate other networks or applications.

### Fix recommendation

- coremgr adds a deny-all rule (`topic: "#"`, `action: "all"`, `permission: "deny"`) at the end of each user's rules, so it no longer depends on the global setting.
- Document that EMQX must set `authorization.no_match = deny`, and update to the EMQX 5/6 configuration style.

### Fix (documentation only)

- `quick.md` (English and Chinese) now uses `EMQX_VER=6.3.3` and recommends 6.3.0 or later.
- The coremgr details in `configuration.md` (English and Chinese):
  - Recommend EMQX 6.3.0 or later with the default hardened security profile.
  - For earlier versions or legacy mode, `authorization.no_match = deny` must be set, and the last rule of `acl.conf` changed to `{deny, all}`.
- The CHANGELOG includes this recommendation.

### Why "coremgr adds a deny-all rule" was not adopted

EMQX evaluates authorization sources in order, and `file` (`acl.conf`) comes before `built_in_database`. In legacy mode, the `{allow, ...}` in `acl.conf` matches first, and the deny rule coremgr writes into the built-in database is never reached. Changing only the code has no effect.

## CMGR-5. [Medium] Without logging in, anyone can send huge request bodies and exhaust coremgr's memory

**Verdict: CONFIRMED (code trace; not tested)　Status: fixed (`a07542aa1`)**

### Defect

- `api_bridge()` reads the whole body into memory with `body::to_bytes(body, usize::MAX)` before forwarding it to auth or the broker (`routes/v1/mod.rs:77`).
- This happens **before** any authentication, and affects every directly forwarded API, such as `PATCH /coremgr/api/v1/user` and `POST /auth/logout`.
- When the data channel is configured, the outer `LogMiddleware` does the same for all DELETE, PATCH, POST, and PUT requests (`routes/middleware.rs:409`).
- axum's default 2 MB limit only applies to extractors, not to this way of reading. The bin only has a 60-second `TimeoutLayer` and no body size limit.

### Trigger path

Without a token, open many connections at once, each sending a body of several GB. Memory is exhausted within 60 seconds.

### Fix recommendation

- Give `to_bytes()` a reasonable limit, such as 2 MB.
- Or add `RequestBodyLimitLayer` in the bin.

### Fix

- `to_bytes()` in `api_bridge()` and `LogMiddleware` now uses `BODY_LIMIT` (2 MiB), the same as axum's default extractor limit.
- Oversized bodies get 400 `err_param`, consistent with corelib's `Json` extractor (axum itself returns 413, but corelib turns every extractor error into `err_param`).
- Tests: the middleware accepts 2 MiB and rejects 2 MiB + 1; the bridged APIs are tested with `POST /auth/logout`.

## CMGR-6. [Low] No size limit when publishing data through the REST APIs

**Verdict: CONFIRMED (PoC, EMQX 6.3.0, 2026-11-01)　Status: fixed (`a07542aa1`)**

### PoC results

Method: mimic coremgr's `emqx::publish_message()` by calling `POST /api/v5/publish` (base64, QoS 0), with an MQTT 3.1.1 QoS 1 subscriber receiving. The test script was one-off and is not in the repo.

| Payload size | REST response | Subscriber |
|---|---|---|
| 512 KiB | 200 | Received |
| 1 MiB - 1 KiB | 200 | Received |
| 1 MiB + 1 KiB | **400 `packet_too_large`** | Not received |
| 1.5 MiB | **400 `packet_too_large`** | Not received |

EMQX **does** check `mqtt.max_packet_size` (default 1 MB) when publishing from the REST API, so the original concern that "the REST API can bypass the size limit" does not hold. As long as EMQX's `max_packet_size` ≤ 1 MiB, oversized messages cannot break the broker's shared connection.

### Remaining risk

- If an operator raises EMQX's `max_packet_size` above 1 MiB, oversized messages are forwarded to the broker and make the shared connection reconnect.
- The documentation still needs to say EMQX's `max_packet_size` must be ≤ 1 MiB, in the coremgr details of `configuration.md`.

### Original analysis (kept for the record)

### Defect

`post_application_dldata()` (`routes/v1/application.rs:958`) and `post_network_uldata()` (`routes/v1/network.rs:948`) only check that `data` is valid hex before publishing through the EMQX or RabbitMQ REST API. axum allows bodies of about 2 MB by default.

### To verify

Messages published from EMQX's REST API might not be limited by the inbound MQTT `max_packet_size`. Need to confirm whether EMQX forwards messages over 1 MiB to MQTT 3.1.1 subscribers.

### Impact

If it forwards them:

- Before the fix: triggers MQ-1, a cross-tenant disconnection.
- After the fix: the inbound limit is 1 MiB. Packets over 1 MiB still make the shared connection reconnect, because rumqttc disconnects when it receives a packet over the inbound limit. So it is still a cross-tenant DoS, only with the threshold raised from 10 KiB to 1 MiB.

### Fix recommendation

- coremgr checks the payload length before publishing, with the same limit as the broker's `MAX_DATA_MSG_SIZE`.
- Document that EMQX's `mqtt.max_packet_size` must be ≤ 1MB, in the coremgr details of `configuration.md`.

### Fix

- `POST /application/{id}/dldata` and `POST /network/{id}/uldata` return 400 `err_param` (`data too large, ...`) when the JSON payload exceeds the broker's `MAX_DATA_MSG_SIZE` (1 MiB - 16 KiB).
- `configuration.md` (English and Chinese) documents that EMQX's `mqtt.max_packet_size` must be ≤ 1 MiB.

## CMGR-7. [Low] The MQ resource cleanup is not awaited, so it never runs

**Verdict: CONFIRMED (code trace)　Status: fixed (`a07542aa1`)**

### Defect

- `clear_queue_rsc()` is an async function, but these 12 places are written as `let _ = clear_queue_rsc(...)`. The future is created and dropped, so it never runs:
  - `routes/v1/mod.rs:389,396,411,450,457,543`
  - `application.rs:238,247,272`
  - `network.rs:252,261,286`
- Also, `post_application()` and `post_network()` never call the cleanup when the broker returns non-200.

### Impact

When creation or update fails, unused users, vhosts, and ACLs are left on the MQ. Combined with CMGR-2, they are also left on the host the attacker chose.

### Fix recommendation

- Add `.await`.
- Also clean up when the broker returns non-200.

### Fix

- All 12 places now have `.await`.
- `post_application()` and `post_network()` also remove the newly created resources when the broker returns non-200.

### Notes

- The maintainer asked whether the missing `.await` might have been intentional, to avoid API timeouts from a slow cleanup. But a future that is not awaited never runs at all; it was not cleaning up in the background. The cleanup is only 2–3 MQ management API calls, fewer than the creation already awaited in the same request (up to 9), and only happens on error paths, so `.await` is kept.
- coremgr's reqwest client has no timeout, so a stuck MQ management API keeps the request hanging. This is already on the TODO list as "(0.5) (HTTP) client timeout". When doing it, use `connect_timeout` plus `read_timeout`, not `timeout()`, or the CSV/list streaming proxies will be cut off.

## CMGR-8. [Not applicable] The built-in rumqttd allows anonymous connections and has no ACL

**Verdict: not applicable　Status: won't fix**

> Per the maintainer: rumqttd is only for demos and PoCs and must not be used in production; sylvia-iot takes no responsibility for it. Only the all-in-one bin (`sylvia-iot-core`) embeds rumqttd, and the individual service bins deliberately do not. Production deployments only consider proper MQTT brokers such as EMQX. The content below is kept for the record only.

### Defect

- rumqttd's listeners bind to `0.0.0.0` with `auth: None` and `external_auth: None` (`sylvia-iot-coremgr/src/libs/mq/rumqttd.rs:45-61,89-96`).
- In rumqttd mode, coremgr's `create_queue_rsc()` and `clear_queue_rsc()` do nothing (`routes/v1/mod.rs:462,521`), so no accounts or ACLs are created.
- The console also binds to `0.0.0.0:18083`.

### Trigger path

Anyone who can reach port 1883 can connect to rumqttd anonymously and then:

- Subscribe to `#` and read all tenants' uplinks and downlinks.
- Publish to `broker.network.<unit>.<code>.uldata` and send fake uplinks as any network.
- Publish to an application's dldata topic and command devices as any application.

### Fix recommendation

- If rumqttd is only meant for single-tenant edge deployments (such as sylvia-router), at least bind to `127.0.0.1` by default and document that it must not be used for multiple tenants.
- Otherwise, enable rumqttd's username/password authentication.

## CMGR-9. [Not applicable] Unit members can publish dldata or uldata through the APIs

**Verdict: not applicable　Status: won't fix**

> Per the maintainer: unit members can already send data directly through the MQ, or indirectly through the uldata/dldata APIs, and this is allowed by design. Only management APIs (create, update, delete, including resetting MQ passwords) are limited to the unit owner. The content below is kept for the record only.

### Defect

`post_application_dldata()` (`application.rs:958`) and `post_network_uldata()` (`network.rs:948`) only check read permission (`get_*_inner()`) before publishing through the MQ REST API with the admin credentials.

### Trigger path

1. When the owner creates an application, coremgr returns the MQ password only to the owner, so normally only the owner can send data as this application.
2. But a unit member with only read permission can use these two APIs to:
   - Send downlinks to the unit's devices as the application.
   - Send fake uplinks as the network.

If "members can send data" is allowed by design, this finding can be downgraded. This needed the maintainer's confirmation.

### Fix recommendation

Use the owner permission check instead.

## CMGR-10. [Medium] Unit members can wipe the MQ resources of an application, a network, or even a whole unit through the DELETE APIs

**Verdict: CONFIRMED (code trace, across coremgr and broker)　Status: fixed (`a07542aa1`)**

> Found while implementing CMGR-3 in the coremgr round (2026-11-21). Line numbers refer to `c3d32c0ed`.

### Defect

The processing order of `delete_application()` (`sylvia-iot-coremgr/src/routes/v1/application.rs:738-780`) and `delete_network()` (`network.rs:765-810`):

1. Call `get_*_inner()` with the user's token. This is the broker's GET, which only requires member permission.
2. Forward the broker's DELETE, and run `clear_queue_rsc()` whenever the response `is_success()`.
3. The broker's DELETE handles non-owners like this: `check_application(..., true)` finds nothing, so it **returns 204 directly and deletes nothing** (`sylvia-iot-broker/src/routes/v1/application/api.rs:787`, and the same for networks).
4. coremgr treats the 204 as success and deletes the application's or network's MQ account, vhost, and ACL. The broker's data is still there, but the connection account is gone.

`delete_unit()` (`unit.rs:201-245`) is worse: **before** forwarding to the broker, it lists all applications and networks of the unit with the member's token (`delete_application_resources()`, `delete_network_resources()`, `unit.rs:234-241`) and deletes their MQ resources one by one. Only then does the broker return 204.

### Trigger path

1. A unit member sends `DELETE /coremgr/api/v1/unit/{unitId}`.
2. The MQ accounts of all applications and networks in the unit are deleted.
3. All devices and applications are disconnected. The broker's data is unchanged, and recovery requires the owner to reset passwords or recreate them one by one.

To delete a single application or network, `DELETE /coremgr/api/v1/application/{id}` or `/network/{id}` is enough.

### Fix recommendation

- Before clearing MQ resources, confirm with `check_unit_owner()` that the user is the owner, an admin, or a manager.
- Requests from non-owners are only forwarded to the broker, which decides the response; coremgr does not touch the MQ resources.
- The broker's write APIs respond inconsistently to non-owners: POST returns `*_not_exist`, PATCH returns 404, and DELETE returns 204 without deleting anything. The silent 204 of DELETE is one of the root causes of this finding. This is discussed together with member permission semantics (see member management in 0.5) and not handled in this finding.

### Fix

- `delete_application()`, `delete_network()`, and `delete_unit()` check with `check_unit_owner()` before clearing MQ resources.
- Requests from non-owners are still forwarded to the broker, which decides the response (currently 204), and coremgr does not touch the MQ resources. If the broker's semantics change later, coremgr follows automatically.
- Tests: after a member sends DELETE for an application, network, or unit, the test logs in to RabbitMQ with that account to confirm the resources still exist; then the owner deletes and gets 204. For public networks, the member gets 404 and a manager can delete.

### Severity adjustment and deferred items

Severity lowered from "High" to "Medium" for the same reason as CMGR-3: only admin or manager can add members, through the API. Member permission semantics are deferred to the unit member management work in 0.5.

## Parts of coremgr checked with no issues found

- **Delete order:** when deleting an application or network, the broker deletes first (including the owner check), and the MQ resources are cleared only on success.
- **MQ account names:** the format is `{type}.{unit_code}.{code}`. code cannot contain `.`, so different applications or networks cannot produce the same name.
- **Management API URLs:** the username used in the URL comes from the code, which is restricted by a regex. The host name problem is covered by CMGR-2.
- **Operation log:** `data.password` is cleared before sending to the data channel.

---

# 5. broker

## BRK-1. [High] The range check of the range APIs truncates integers, so any logged-in user can hang the broker and exhaust its memory

**Verdict: CONFIRMED (code trace; not tested)　Status: fixed (`c3d32c0ed`)**

### Defect

The range check is written as `(end_addr - start_addr) as usize >= BULK_MAX`. Casting `u128` to `usize` drops the high bits on 64-bit systems, so a difference of `2^64 + 5` becomes `5` and passes the check.

The same code appears in 4 places:

- `sylvia-iot-broker/src/routes/v1/device/api.rs:717` (`post_device_range`)
- `device/api.rs:854` (`post_device_range_del`)
- `device_route/api.rs:616` (`post_device_route_range`)
- `device_route/api.rs:758` (`post_device_route_range_del`)

The following `for addr in start_addr..end_addr` loop pushes a string into a `Vec` on each iteration, with no `.await` in between. It therefore occupies a tokio worker until memory is exhausted.

### Trigger path

1. Any logged-in user can create their own unit with `POST /broker/api/v1/unit` (`unit.post` has no role restriction), then create a network.
2. Call `POST /broker/api/v1/device/range` with:
   - `startAddr` = `00000000000000000000000000000000`.
   - `endAddr` = `00000000000000010000000000000005`.
3. After the check passes, the loop has to run more than 2^64 times. The broker's CPU is occupied and its memory keeps growing until exhausted. The APIs and data forwarding of all tenants are affected.

Also, when `endAddr` is all `f`, `end_addr += 1` overflows: debug builds panic, and release builds wrap around to 0.

### Fix

- `BULK_MAX` is now `u128`, the range check compares `end_addr - start_addr >= BULK_MAX` directly, and the bulk APIs' count check uses `len() as u128`.
- `end_addr += 1` is removed and the loops use `start_addr..=end_addr`, so an end address of all `f` does not overflow.
- Tests: all 4 range APIs have cases with more than 1024 addresses and with 2^64+5; there are also tests creating and deleting `ff…fe` to `ff…ff`.

## BRK-2. [Low] dldata-result does not check whether the data belongs to the reporting network

**Verdict: PLAUSIBLE (the missing check is confirmed, but a dataId cannot be guessed in practice)　Status: fixed (`c3d32c0ed`)**

### Defect

After `on_dldata_result()` gets the dldata buffer by `dataId`, it does not check that the buffer's `network_id` is the reporting network (`sylvia-iot-broker/src/routes/v1/network/api.rs:1778`).

### Trigger path

1. Network B sends a dldata-result with a `dataId` that belongs to network A.
2. The broker forwards the forged result to the application and deletes the buffer.

A `dataId` is a millisecond timestamp plus 12 random alphanumeric characters (about 2^71 combinations), so it cannot be guessed in practice unless leaked.

### Fix

- `on_dldata_result()` compares `dldata.network_id` with the reporting network. On a mismatch it logs a warning and ignores the result: it is not forwarded to the application, and the buffer is not deleted.
- The record is still sent to the data channel for auditing.
- Test: in the routing tests, the public network reports with the manager's dataId and the application receives nothing; afterwards, the manager network's report still works.

## Parts of the broker checked with no issues found

- **auth middleware:** every request queries tokeninfo from auth, with no caching. The role and scope checks match auth's.
- **Permission matrix:** for application, network, device, device-route, network-route, and dldata-buffer:
  - Reads use `only_owner=false`.
  - Create, update, and delete use `only_owner=true`.
  - Only admin/manager can change a unit's owner and members.
  - Non-admins must specify `unit` for lists.
  - Public networks are for admin/manager only.
- **Unit consistency:**
  - A device's `check_network()` requires a private network to belong to the same unit.
  - `post_device_route()` and `post_network_route()` both check that the units match.
  - The bulk and range device route APIs do not check that the application and network are in the same unit. But a non-admin must own both units, so it does not cross tenants; it is only a data consistency issue.
- **dldata authorization:** an application can only send to devices in its own unit. With a deviceId, the query uses `unit_id`; with a network code plus addr, it uses `unit_code`.
- **SQL:** every value goes through `quote()`, including each item of `IN` lists. Sort fields are fixed enums. MongoDB queries all use `doc!`.

---

# 6. data

## DATA-1. [Critical] The unit check of the data service can be bypassed with an empty `?unit=`, so any logged-in user can read all tenants' data

**Verdict: CONFIRMED (code trace; not tested)　Status: fixed (`a5cda738a`)**

### Defect

`get_unit_cond()` (`sylvia-iot-data/src/routes/v1/mod.rs`) only checks whether the user is admin/manager when the `unit` parameter is **missing entirely** (`None`).

When `unit` is an empty string, it reaches `0 => Ok(None)` and returns a condition that does not filter by unit, without any role check:

```rust
match query_unit {
    None => { if !ADMIN && !MANAGER { return Err("missing `unit`") } Ok(None) }
    Some(unit_id) => match unit_id.len() {
        0 => Ok(None),          // ← non-admins reach here too
        _ => { /* check unit read permission with the broker */ }
    },
}
```

The broker's equivalent checks (for example the list in `device/api.rs`) reject both `None` and the empty string; the data service only rejects `None`.

### Scope

The `/count` and `/list` APIs of the 4 groups `application-uldata`, `application-dldata`, `network-uldata`, and `network-dldata` all use `get_unit_cond()`. These routes only require a valid token, **with no role restriction**.

### Trigger path

1. Any logged-in user, such as a newly registered normal user, calls `GET /data/api/v1/network-uldata/list?unit=&limit=0`.
2. `serde_urlencoded` parses `unit=` as `Some("")`, so `unit_cond` is `None` and the query does not filter by `unit_id`.
3. The response contains the data records of **all tenants**, including:
   - Devices' uplink and downlink payloads (`data`).
   - `extension`.
   - The IDs and codes of units, networks, and devices.

### Fix recommendation

Apply the same check to the empty string as to `None`: return an error for non-admins.

The broker's approach can be followed: check the role first, then handle `None` and the empty string.

### Fix

- `get_unit_cond()` first treats an empty string as a missing `unit` with `query_unit.filter(|u| u.len() > 0)`, then checks the role: only admin and manager can query all units, and normal users get 400 `err_param` (`missing unit`). This matches api.md's "`unit`: required for normal user" and "manager acts as admin to view all data".
- With a unit ID, nothing changes: the broker still checks read permission with the user's token, and members can read.
- Tests: the count and list APIs of the 4 uldata/dldata groups each reject `unit=` from the owner and a member; the existing tests where a manager queries everything with `unit=` still pass.

## Parts of data checked with no issues found

- **coremgr operation log:** non-admins are always forced to query only their own `user_id`, with no way around it.
- **Unit read permission:** with a normal unit ID, the broker's `GET /unit/{id}` is called with the user's own token, so members can read and non-members are rejected, as the permission model requires.
- **The `unit` parameter is put into a URL path:**
  - `unit` goes into the broker URL without encoding, for example `../application/x` or `mine?x`. After URL normalization, it does hit a different API.
  - But the database filter uses the raw string with an exact match, so nothing is found and there is no privilege escalation.
- **The data channel receiver:** time parsing is all handled with `match`, with no panicking path.
  - Who can publish to the data channel depends on MQ isolation. For EMQX, see CMGR-4.
- **SQL:** string values all go through `quote()`, times are integers only, and sort fields are fixed enums.

---

# 7. sdk

No issues found.

## Parts of the sdk checked with no issues found

- **MQ receivers:** for dldata, ctrl, uldata, dldata-resp, and dldata-result from the broker, hex and times are parsed with `match`, with no panicking path. `unwrap()` is only used on locks.
- **Sending:** `send_*()` now has the synchronous `check_msg_size()` check (see MQ-1).
- **HTTP client:** gets tokens with client credentials. On 401 it retries only once, not forever. It uses reqwest's default TLS verification.
- **auth middleware:** queries tokeninfo from auth every time, with no caching.

---

# 8. coremgr-cli

Both findings only affect the machine running the CLI.

## CLI-1. [Low] The token storage file usually has permission `0644`, so other local users can read it

**Verdict: CONFIRMED (code trace)　Status: fixed (`2d730dff6`)**

### Defect

`write_storage()` writes `~/.sylvia-iot-coremgr-cli.json` with `fs::write()` (`sylvia-iot-coremgr-cli/src/libs/cli/config.rs:23-26`). This file stores the access token and the refresh token (valid for 14 days).

`fs::write()` creates files with permission `0666` minus the umask. The common umask is `022`, so the file ends up `0644`.

### Trigger path

On a shared machine, other local users can read this file and keep exchanging the refresh token for new access tokens, impersonating the CLI user.

### Fix recommendation

On Unix, create the file with `OpenOptions` and `mode(0o600)`.

### Fix

- `write_storage()` now uses `write_private_file()`: on Unix it creates the file with `OpenOptions` and `mode(0o600)`, then calls `set_permissions(0o600)` before writing. `mode` only applies to new files, so `0644` files left by older versions are fixed on the next write. Non-Unix platforms keep `fs::write()`.
- Verification: with umask 022, new files are `600`, and an existing `644` file becomes `600` after writing. coremgr-cli has no tests and CI only builds it, so there is no coverage report.

## CLI-2. [Low] The password can only be passed with the `--password` command-line argument

**Verdict: CONFIRMED (code trace)　Status: fixed (`2d730dff6`)**

### Defect

`--password` of the `login` subcommand is required, with no other input method such as an interactive prompt or an environment variable (`sylvia-iot-coremgr-cli/src/libs/cli/login.rs:64-70`).

### Trigger path

The password ends up in the shell history. While the command runs, other local users can also see it with `ps`.

### Fix recommendation

- Make `--password` optional.
- When it is not given, read it interactively without echo (for example with `rpassword`).

### Fix

- `--password` is now optional. When it is not given, `rpassword::prompt_password()` reads it from the terminal without echo. Added the `rpassword` 7.5.5 dependency.
- `-p` still works, so `login -a admin -p admin` in the documentation and existing scripts are not affected.

## Other coremgr-cli fixes (done at the same time)

- **The `coremgr-cli.*` command-line arguments were not registered:** `read_args()` reads arguments such as `--coremgr-cli.auth` (also listed in the documentation), but they were never registered with clap, since the first commit. Without `-f`, debug builds panicked on clap's debug check, and in release builds the command-line arguments had no effect, leaving only environment variables. `reg_args()` now registers them.
- **Removed the unused `serde_with` dependency:** it has been there since the first commit, but coremgr-cli never used it. It was probably copied along with auth's dependency list.

## Parts of coremgr-cli checked with no issues found

- **Login flow:** after taking the code from the server's Location header, the token request always goes to the configured auth address, not wherever the server's Location points.
- **CSV output:** the file name comes from the user's own arguments.
- **URL settings:** the auth, coremgr, and data URLs are all checked with `is_uri()`.

---

# 9. router (handled as needed)

Per the maintainer, the router is an IP-sharing router project, unrelated to IoT itself and never deployed on K8S, so whether to fix the issues in this section is decided as needed.

## RTR-1. [Medium] The router management APIs have no role check; any user can change the WAN/LAN/WiFi settings and read the PPPoE and WiFi passwords

**Verdict: CONFIRMED (code trace)　Status: as needed**

### Defect

All routes in `routes/v1/net/mod.rs` and `routes/v1/sys/mod.rs` only use the SDK's `AuthService`, which only checks that the token is valid, with no role or scope check.

### Trigger path

Any user with a valid token, not necessarily an admin, can:

- `GET /router/api/v1/net/wan`: `conf.pppoe` in the response contains the PPPoE **account and password**.
- `GET /router/api/v1/net/wlan`: the response contains the WiFi **password**.
- `PUT /router/api/v1/net/wan/{id}`, `PUT /lan`, `PUT /wlan`, `PUT /wwan`: change the network settings directly, for example changing the PPPoE credentials to take the whole router offline, or changing the LAN DNS and gateway.

### Fix recommendation

- Add an admin role check to these routes, like auth and the broker.
- Or at least restrict PUT and the GETs that include passwords to admins.

## RTR-2. [Medium] All router services bind to `[::]`, including the WAN interface, and the repo has no firewall configuration or guidance

**Verdict: PLAUSIBLE (confirmed in code; the actual impact depends on the host firewall)　Status: as needed**

### Defect

`bin/sylvia-router.rs` binds both the HTTP and HTTPS services to `[::]`, which means all interfaces (dual-stack). This bin includes auth, broker, coremgr, data, and the router management APIs.

Neither the code nor the documentation in the repo configures a firewall (iptables, nftables, ufw), and Ubuntu does not enable a firewall by default.

### Trigger path

With the default deployment, the WAN side (the internet) can reach directly:

- The OAuth2 login page: AUTH-1 and AUTH-2 could be attacked from outside before they were fixed (now fixed).
- The management APIs: combined with RTR-1, any account whose password leaks can change the router's settings from outside.
- The broker and data APIs: issues such as DATA-1 and BRK-1 could also be triggered from outside before they were fixed (all now fixed).

### Fix recommendation

- Bind only to the LAN address by default.
- Or document that the WAN side must block the service ports with a firewall.

## Parts of the router checked with no issues found

- **Command injection:** `nmcli` and `systemctl` arguments are passed one by one with `Command::args()`, without a shell, so there is no shell injection. Each value is the "value" after an nmcli key, so no extra properties can be injected.
- **dhcpd configuration file:** `dhcpStart` and `dhcpEnd` are parsed as `Ipv4Addr`, `address` as `Ipv4Net`, and `leaseTime` is a `usize`, so newlines or extra settings cannot be injected into `/etc/dhcp/dhcpd.conf`.
- **Static IP:** the address, gateway, and DNS are all parsed as IPv4 addresses, excluding special addresses such as broadcast and loopback.
- **WiFi:** the channel is limited to 1–11. The SSID and password are passed as nmcli arguments and are not parsed as commands.

## Appendix: PPPoE often drops and redials on Ubuntu 22.04 (not a security issue, not tested)

The router itself has **no** code that operates the network periodically; it only deletes and recreates the WAN connection when an API is called. So the drops are not actively caused by the router's code, and are more likely due to NetworkManager (NM) settings or the system environment.

`set_wan_conf()` creates the PPPoE connection with `nmcli c add type pppoe con-name <wan_id> ifname <physical interface> username … password …` (`libs/network.rs`), with no other parameters. The hypotheses, ordered by likelihood:

1. **Other auto-connecting ethernet profiles on the same interface.**
   - Situation: `ifname` names the physical interface directly, which is NM's legacy PPPoE mode. PPPoE is then tied to the interface's own lifecycle.
   - Cause: `set_wan_conf()` only deletes the profile named `wan_id`. If the system also has an ethernet profile such as `Wired connection 1` or netplan's `netplan-<interface>` with autoconnect on, NM may activate that profile (running DHCP) when PPPoE drops briefly or the interface carrier changes, pushing PPPoE out and redialing later.
   - Check: `nmcli -f NAME,TYPE,DEVICE,AUTOCONNECT c show`.
2. **LCP echo timeouts.**
   - Situation: NM's PPPoE sends LCP echoes periodically by default and declares the link down after consecutive missing replies. If the ISP's equipment replies slowly or the line drops packets occasionally, pppd disconnects and redials.
   - Check: look for messages such as `No response to N echo-requests` and `LCP terminated` in `journalctl -u NetworkManager -b`.
   - Mitigation: tune `ppp.lcp-echo-failure` and `ppp.lcp-echo-interval`.
3. **netplan and systemd-networkd also manage the interface.**
   - Situation: Ubuntu Server 22.04 uses netplan with networkd by default. If `/etc/netplan/*.yaml` still configures the WAN interface (for example `dhcp4: true`), networkd and NM may operate the same interface and interfere with each other.
   - Check: `networkctl list` to see whether the WAN interface is `configured`, then check the netplan configuration.
4. **Scheduled disconnection by the ISP, or MTU.**
   - Scheduled redials by the ISP: some providers drop sessions periodically, which is normal. A fixed interval between drops suggests this.
   - MTU: the PPPoE MTU must be ≤ 1492. A wrong setting usually means "connected but some websites hang", and rarely causes drops.

### Code changes to consider

- Use NM's newer style: `pppoe.parent <physical interface>` with a separate `ifname` (for example `ppp-<wan_id>`), decoupling PPPoE from other profiles on the interface.
- When creating PPPoE, turn off autoconnect of other profiles on the same interface.
- Set `ppp.lcp-echo-*` and `ppp.mtu` explicitly.
- `put_wan` lowercases the PPPoE account (`routes/v1/net/api.rs:539`). This is unrelated to the intermittent drops, because a wrong account would never connect at all, but it is a problem if the ISP's accounts are case-sensitive.

---

# Results and follow-ups

This round of security fixes was split into 8 commits by crate, all on branch `20261122-security-fixes`: general-mq → corelib → coverage CI → auth → broker → coremgr → data → coremgr-cli.

**Done:** MQ-1, MQ-3, MQ-4 (`d1e5329f2`); CORE-1 to CORE-4 (`1a78a3c3f`); AUTH-1 to AUTH-5, AUTH-7, and part of AUTH-6 (`1c7a4999c`); BRK-1, BRK-2, and the broker side of CMGR-1 (`c3d32c0ed`); CMGR-2 to CMGR-7 and CMGR-10 (`a07542aa1`); DATA-1 (`a5cda738a`); CLI-1 and CLI-2 (`2d730dff6`). In addition, `034e00b1b` fixed the CI coverage report pages.

**Deferred to 0.5:**
- **MQ-2:** MQTT manual ack and backpressure.
- **AMQP publisher confirms:** `send_msg()` waits for the broker's confirmation (see "Not vulnerabilities, but worth noting" under general-mq).
- **The login attempt limit of AUTH-6.**
- **Unit member management:** the error code for non-owners (currently `err_param`, marked `TODO: member management` in the code), and consistent broker write API responses to non-owners. See CMGR-3 and CMGR-10.
- **HTTP client timeout:** use `connect_timeout` plus `read_timeout`, not `timeout()`. See CMGR-7.

**Handled as needed:** router's RTR-1 and RTR-2.

**Won't fix:** CMGR-8 (rumqttd is only for demos/PoCs) and CMGR-9 (members sending and receiving data is allowed by design).
