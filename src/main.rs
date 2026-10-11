use std::{env, future::Future, pin::Pin, task::{Context, Poll, RawWaker, RawWakerVTable, Waker}};
use creator_event_recovery::{commerce_webhook::{decide_delivery, queue_payload}, infrai_client::Infrai};

async fn run() -> Result<(), String> {
    let base_url = env::var("PUBLIC_BASE_URL").map_err(|_| "set PUBLIC_BASE_URL".to_owned())?;
    let signing_secret = env::var("WEBHOOK_SIGNING_SECRET").map_err(|_| "set WEBHOOK_SIGNING_SECRET".to_owned())?;
    let infrai = Infrai::from_env().map_err(|e| e.to_string())?;
    let registration = infrai.register_webhook(&format!("{base_url}/platform-events"), &signing_secret).map_err(|e| e.to_string())?;
    let webhook_id = env::var("WEBHOOK_ID").unwrap_or_else(|_| "replace-with-registration-id".into());
    let deliveries = infrai.deliveries(&webhook_id).map_err(|e| e.to_string())?;
    let subscription = infrai.subscribe_queue("creator-commerce-events", &format!("{base_url}/platform-events")).map_err(|e| e.to_string())?;
    let redrive = infrai.redrive_failed("creator-commerce-events").map_err(|e| e.to_string())?;
    let sample = "{\"event\":\"digital_asset.purchased\",\"asset_id\":\"demo-asset\"}";
    let ignored = decide_delivery(sample, "signature-from-request", &signing_secret);
    println!("webhook={registration}\ndeliveries={deliveries}\nsubscription={subscription}\nredrive={redrive}\npayload-boundary={ignored:?}");
    println!("Workers enqueue: {}", queue_payload(&creator_event_recovery::commerce_webhook::DeliveryDecision::ContentProcessingQueued { asset_id: "demo-asset".into() }));
    Ok(())
}

fn block_on<F: Future>(mut future: F) -> F::Output { fn no_op(_: *const ()) {} fn clone(_: *const ()) -> RawWaker { raw() } fn raw() -> RawWaker { RawWaker::new(std::ptr::null(), &RawWakerVTable::new(clone, no_op, no_op, no_op)) } let waker = unsafe { Waker::from_raw(raw()) }; let mut context = Context::from_waker(&waker); let mut future = unsafe { Pin::new_unchecked(&mut future) }; loop { if let Poll::Ready(value) = future.as_mut().poll(&mut context) { return value; } } }
fn main() { if let Err(error) = block_on(run()) { eprintln!("{error}"); std::process::exit(1); } }
