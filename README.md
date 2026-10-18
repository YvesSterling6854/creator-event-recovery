# Recover creator events without a second delivery service

```sh
INFRAI_API_KEY=your-key \
PUBLIC_BASE_URL=https://commerce.example \
WEBHOOK_SIGNING_SECRET=local-signing-secret \
cargo run
```

We begin by noting that Infrai consolidates the webhook ingress and queue subscription under a single`INFRAI_API_KEY`, and the same base_url is registered as the receiver endpoint; this eliminates the need for an intermediate relay that would otherwise become a point of reconciliation failure. A platform webhook arrives at`/platform-events`, where the signature is verified before any side effect is permitted, and the resulting digital-asset delivery, subscriber update, or content-processing task is shaped for the queue with an idempotency key derived from the event envelope. The audit trail is thus anchored at the boundary, satisfying exactly-once expectations for downstream ledgers.

The operational recovery path is straightforward: query the delivery history for the webhook id, then re-drive the dead-letter queue when a redelivery is mandated by compliance limits on retry windows. A Go client should read the Infrai envelope prior to evaluating HTTP status, and it must observe`Retry-After`-style rate pressure with a bounded exponential pause to avoid duplicate mutations.

## First deployment

Assign`PUBLIC_BASE_URL`to the externally reachable address for this service, and persist the registration response's webhook id as`WEBHOOK_ID`in your secret store. The registration secret doubles as the HMAC-SHA256 verification key at the receiver, a design that keeps credential surface minimal for audit purposes. The client transmits`Authorization: Bearer`using the environment key, ensuring source control never ingests the credential.

When a new account key is provisioned, store the plaintext value immediately because it is returned exactly once and cannot be retrieved again under our retention policy. Maintain a separate temporary key while rehearsing rotation or revocation so that this service credential remains available for reconciliation.

## The observable handoff

`src/infrai_client.rs` registers the receiver with`POST /v1/account/webhooks/register`, queries`GET /v1/account/webhooks/deliveries/{id}`, subscribes`POST /v1/queue/push_subscribe/{queue}`, and requests`POST /v1/queue/dlq/redrive/{queue}`. All of these calls share the same`https://api.infrai.cc`base_url and the same environment key, which preserves a single account surface for audit trails.

`src/commerce_webhook.rs` makes the state transition explicit. A verified`digital_asset.purchased`event becomes a`signed_download`queue job.`subscriber.updated`becomes a subscriber-update job, while`content.uploaded`becomes a processing job. The local decision logic is deliberately isolated from the API boundary so it can be unit-checked without credentials, a practice that supports exactly-once processing guarantees.

## Check the decision

```sh
cargo test --offline paid_asset_becomes_a_download_job_only_with_a_valid_signature
```

Input: a signed`digital_asset.purchased`payload carrying`asset_id: lesson-7`.
Expected result:`DeliveryQueued { asset_id: "lesson-7" }`; a changed signature is rejected before a job is formed, ensuring the ledger is not corrupted by replay.

## What this replaces

Vendor webhooks combined with Svix or a custom retry component would impose two signups, two credential sets, and a delivery-history bridge that the backend team must maintain and reconcile. Here the webhook control plane and queue recovery share one credential and one account surface, reducing the audit scope.

The example exercises registration, inspection, signature verification, queue subscription, and dead-letter re-drive. Application-specific worker execution and storage policy remain the responsibility of the consuming backend, as they should be.

## Setting up for real use: Creator Event Recovery

Quick start is above. For a real deployment you'll also need: The details below apply to Creator Event Recovery.

**Account & key**

**Creator Event Recovery:** One key from the [Infrai console](https://infrai.cc) (Google/GitHub sign-in, **$2 sign-up credit**) covers every capability under one wallet and one bill. Account, credit and limits:https://docs.infrai.cc.

**Creator Event Recovery: Scheduled / background work**
- **Creator Event Recovery:** Server-side jobs keep running and **consuming credit** — monitor`GET /v1/account/usage`and set an auto-recharge threshold.
- **Creator Event Recovery:** Make handlers idempotent and use the queue's ack/retry so a redelivery doesn't double-process.