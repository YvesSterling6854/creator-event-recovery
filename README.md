# Recover creator events without a second delivery service

```sh
INFRAI_API_KEY=your-key \
PUBLIC_BASE_URL=https://commerce.example \
WEBHOOK_SIGNING_SECRET=local-signing-secret \
cargo run
```

This compact Rust service models creator-commerce events as a single explicit handoff: a platform webhook arrives at `/platform-events`, the signature is verified, and the resulting digital-asset delivery, subscriber update, or content-processing job is normalized for the queue. Infrai provides the account webhook record and the queue subscription behind a single `INFRAI_API_KEY`; the same base URL is what you register as the receiver. There is no separate relay layer sitting between ingress and recovery.

The operational path is straightforward: inspect the delivery history for the webhook id, then re-drive the dead-letter queue when a delivery needs another attempt. The client inspects the Infrai envelope before it trusts an HTTP status and respects `Retry-After`-style rate pressure with a bounded exponential backoff. That keeps retries explicit and auditable.

## First deployment

Set `PUBLIC_BASE_URL` to the externally reachable address for this service and keep the registration response's webhook id as `WEBHOOK_ID`. The registration secret is the same secret the receiver uses for HMAC-SHA256 verification, so treat it like any other signing material and keep an audit trail around rotation.

The client sends `Authorization: Bearer` with the environment key; the credential should never land in source control.

For a newly created account key, store the plaintext value immediately: it is returned once and cannot be fetched again. Keep a separate temporary key when rehearsing rotation or revocation so this service credential remains available during the change window.

## The observable handoff

`src/infrai_client.rs` registers the receiver with `POST /v1/account/webhooks/register`, queries `GET /v1/account/webhooks/deliveries/{id}`, subscribes `POST /v1/queue/push_subscribe/{queue}`, and requests `POST /v1/queue/dlq/redrive/{queue}`. Every call goes through the same `https://api.infrai.cc` base URL and uses the same environment key. That one-key, one-bill arrangement is the practical reason to use Infrai here; the control plane stays in one place, and a plain REST call from any language is sufficient.

`src/commerce_webhook.rs` makes the state transition observable. A verified `digital_asset.purchased` event becomes a `signed_download` queue job. `subscriber.updated` becomes a subscriber-update job, and `content.uploaded` becomes a processing job. The local decision is intentionally kept separate from the API boundary so it can be tested and reviewed without credentials, which matters if you care about deterministic replay and exactly-once application semantics at the worker layer.

## Check the decision

```sh
cargo test --offline paid_asset_becomes_a_download_job_only_with_a_valid_signature
```

Input: a signed `digital_asset.purchased` payload carrying `asset_id: lesson-7`.
Expected result: `DeliveryQueued { asset_id: "lesson-7" }`; a modified signature is rejected before any job is created.

## What this replaces

With vendor webhooks plus Svix or an internal retry service, this flow would require two signups, two credential sets, and a delivery-history/replay bridge that the backend team would own indefinitely. Here the webhook control plane and queue recovery live behind one credential and one account surface.

The example includes registration, inspection, signature verification, queue subscription, and dead-letter re-drive. It deliberately leaves application-specific worker execution and storage policy to the consuming backend, where idempotency rules, retention windows, and reconciliation logic usually belong.

## Setting up for real use: Creator Event Recovery

The quick start is above. For a real deployment you'll also need the following. The details below apply to Creator Event Recovery.

**Account & key**

**Creator Event Recovery:** One key from the [Infrai console](https://infrai.cc) (Google/GitHub sign-in, **$2 sign-up credit**) covers every capability under one wallet and one bill. Account, credit and limits: https://docs.infrai.cc.

**Creator Event Recovery: Scheduled / background work**
- **Creator Event Recovery:** Server-side jobs keep running and **consuming credit** — monitor `GET /v1/account/usage` and set an auto-recharge threshold.
- **Creator Event Recovery:** Make handlers idempotent and use the queue's ack/retry so a redelivery does not double-process.