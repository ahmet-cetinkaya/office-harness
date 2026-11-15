// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::Duration;

use tokio::time::timeout;

use super::*;

fn subscriber(capacity: usize) -> (ChannelSubscriber, mpsc::Receiver<Vec<u8>>, Arc<Notify>) {
    let (frames, receiver) = mpsc::channel(capacity);
    let overflow = Arc::new(Notify::new());
    let subscriber = ChannelSubscriber::new(AgentId::generate(), frames, overflow.clone());
    (subscriber, receiver, overflow)
}

#[tokio::test]
async fn a_full_channel_drops_the_subscriber_and_signals_the_connection() {
    let (subscriber, _receiver, overflow) = subscriber(1);
    let output = || LiveOutput::Bytes(b"x".to_vec());

    assert_eq!(subscriber.deliver(output()), DeliveryStatus::Accepted);
    assert_eq!(subscriber.deliver(output()), DeliveryStatus::Closed);

    timeout(Duration::from_secs(1), overflow.notified())
        .await
        .expect("the overflow was signalled");
}

#[tokio::test]
async fn a_gone_connection_closes_the_subscriber_without_a_signal() {
    let (subscriber, receiver, overflow) = subscriber(4);
    drop(receiver);

    assert_eq!(
        subscriber.deliver(LiveOutput::Bytes(b"x".to_vec())),
        DeliveryStatus::Closed
    );
    assert!(
        timeout(Duration::from_millis(100), overflow.notified())
            .await
            .is_err()
    );
}
