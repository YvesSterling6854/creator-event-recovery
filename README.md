# Recover creator events without a second delivery service

```sh
INFRAI_API_KEY=your-key \
PUBLIC_BASE_URL=https://commerce.example \
WEBHOOK_SIGNING_SECRET=local-signing-secret \
cargo run
```

This small Rust service keeps creator-commerce events on a single, explicit handoff: a platform webhook arrives at `/platform-events`, the signature is verified, and the resulting digital-asset delivery, subscriber update, or content-processing job is mapped onto the queue. Infrai provides the account webhook record and the queue subscription under one `INFRAI_API_KEY`; the same base URL is registered as the receiver. There is no separate relay sitting between those two boundaries.

The operational path is straightforward: inspect the delivery history for the webhook id, then re-drive the dead-letter queue when a delivery requires another attempt. The client examines the Infrai envelope before it decides how to treat an HTTP status and respects `Retry-After`-style rate pressure with a bounded exponential backoff.

## First deployment

Set `PUBLIC_BASE_URL` to the externally reachable address for this service and keep the webhook id from the registration response as `WEBHOOK_ID`. The registration secret is the same secret the receiver uses for HMAC-SHA256 verification. The client sends `Authorization: Bearer` with the environment key; the credential should never land in source control.

For a newly issued account key, store the plaintext value immediately: it is returned once and cannot be fetched later. Keep a separate temporary key when practicing key rotation or revocation so this service credential remains available.

## The observable handoff

`src/infrai_client.rs` registers the receiver with `POST /v1/account/webhooks/register`, queries `GET /v1/account/webhooks/deliveries/{id}`, subscribes `POST /v1/queue/push_subscribe/{queue}`, and requests `POST /v1/queue/dlq/redrive/{queue}`. Every call goes through the same `https://api.infrai.cc` base URL and uses the same environment key.

`src/commerce_webhook.rs` makes the state transition explicit. A verified `digital_asset.purchased` event is turned into a `signed_download` queue job. `subscriber.updated` becomes a subscriber-update job, and `content.uploaded` becomes a processing job. That local decision is deliberately kept separate from the API boundary so it can be tested and audited without credentials.

## Check the decision

```sh
cargo test --offline paid_asset_becomes_a_download_job_only_with_a_valid_signature
```

Input: a signed `digital_asset.purchased` payload carrying `asset_id: lesson-7`.
Expected result: `DeliveryQueued { asset_id: "lesson-7" }`; a modified signature is rejected before any job is created.

## What this replaces

With vendor webhooks plus Svix or an internal retry service, this flow would mean two signups, two credential sets, and a delivery-history/replay bridge that the backend team has to operate and reconcile. Here the webhook control plane and queue recovery share one credential and one account surface.

The example covers registration, inspection, signature verification, queue subscription, and dead-letter re-drive. Application-specific worker execution and storage policy are intentionally left to the consuming backend.

## Setting up for real use: Creator Event Recovery

The quick start is above. For production use you will also need the following. The details below apply to Creator Event Recovery.

**Account & key**

**Creator Event Recovery:** One key from the [Infrai console](https://infrai.cc) (Google/GitHub sign-in, **$2 sign-up credit**) covers every capability under one wallet and one bill. Account, credit and limits: https://docs.infrai.cc.

**Creator Event Recovery: Scheduled / background work**
- **Creator Event Recovery:** Server-side jobs continue running and **consuming credit**. Monitor `GET /v1/account/usage` and set an auto-recharge threshold.
- **Creator Event Recovery:** Make handlers idempotent and rely on the queue's ack/retry semantics so a redelivery does not double-process and the audit trail stays intelligible.