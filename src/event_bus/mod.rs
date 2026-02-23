//! Event bus: bounded MPSC channel between platform capture and rule engine.
//!
//! Decouples the capture callback (producer) from the rule engine consumer,
//! absorbing brief bursts while preserving strict event ordering. The channel
//! is synchronous (`std::sync::mpsc::sync_channel`) so publishers never
//! allocate on the heap per-send. Uses `try_send` to avoid stalling the
//! capture thread when the channel is full; dropped events are logged.

use std::sync::mpsc;

use crate::platform::InputEvent;

/// Default channel capacity. Sized for keystroke bursts at human typing speeds.
pub const DEFAULT_CAPACITY: usize = 256;

// ---------------------------------------------------------------------------
// Publisher
// ---------------------------------------------------------------------------

/// Sending end of the event bus.
///
/// `Clone`able and `Send` so it can be moved into a capture callback or
/// shared across producer threads.
#[derive(Clone)]
pub struct EventPublisher {
    sender: mpsc::SyncSender<InputEvent>,
}

impl EventPublisher {
    /// Send an event to the bus.
    ///
    /// Uses `try_send` so the capture callback never blocks. Logs a warning
    /// and drops the event when the channel is at capacity.
    pub fn send(&self, event: InputEvent) {
        log::debug!("event_bus: publish {:?} {:?}", event.key, event.state);
        if let Err(e) = self.sender.try_send(event) {
            log::warn!("event_bus: dropped event ({})", e);
        }
    }
}

// ---------------------------------------------------------------------------
// Subscriber
// ---------------------------------------------------------------------------

/// Receiving end of the event bus.
///
/// Used by the rule engine to consume events. Implements `Iterator` for
/// ergonomic `for event in subscriber` loops; the iterator returns `None`
/// when all `EventPublisher` handles have been dropped.
pub struct EventSubscriber {
    receiver: mpsc::Receiver<InputEvent>,
}

impl EventSubscriber {
    /// Blocking receive. Returns `None` when all publishers have been dropped.
    pub fn recv(&self) -> Option<InputEvent> {
        match self.receiver.recv() {
            Ok(event) => {
                log::debug!("event_bus: deliver {:?} {:?}", event.key, event.state);
                Some(event)
            }
            Err(_) => {
                log::debug!("event_bus: channel closed, subscriber exiting");
                None
            }
        }
    }
}

impl Iterator for EventSubscriber {
    type Item = InputEvent;

    fn next(&mut self) -> Option<Self::Item> {
        self.recv()
    }
}

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

/// Create a linked publisher/subscriber pair with the given channel capacity.
pub fn new(capacity: usize) -> (EventPublisher, EventSubscriber) {
    let (sender, receiver) = mpsc::sync_channel(capacity);
    (EventPublisher { sender }, EventSubscriber { receiver })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;
    use crate::platform::{InputEvent, KeyCode, KeyState, Modifiers, WindowContext};

    fn make_event(key: KeyCode) -> InputEvent {
        InputEvent {
            key,
            state: KeyState::Down,
            modifiers: Modifiers::default(),
            window: WindowContext::default(),
        }
    }

    #[test]
    fn send_and_receive_single_event() {
        let (publisher, subscriber) = new(8);
        publisher.send(make_event(KeyCode::A));
        let received = subscriber.recv().unwrap();
        assert_eq!(received.key, KeyCode::A);
    }

    #[test]
    fn subscriber_returns_none_when_all_publishers_dropped() {
        let (publisher, subscriber) = new(8);
        drop(publisher);
        assert!(subscriber.recv().is_none());
    }

    #[test]
    fn events_are_ordered() {
        let (publisher, subscriber) = new(8);
        let keys = [KeyCode::A, KeyCode::B, KeyCode::C];
        for &key in &keys {
            publisher.send(make_event(key));
        }
        drop(publisher);
        let received: Vec<KeyCode> = subscriber.map(|e| e.key).collect();
        assert_eq!(received, keys);
    }

    #[test]
    fn full_channel_warns_and_does_not_block() {
        // Capacity 2; send 4 events; only the first 2 should be received.
        let (publisher, subscriber) = new(2);
        for _ in 0..4 {
            publisher.send(make_event(KeyCode::A));
        }
        drop(publisher);
        assert_eq!(subscriber.count(), 2);
    }

    #[test]
    fn clone_publisher_both_ends_deliver() {
        let (publisher, subscriber) = new(8);
        let publisher2 = publisher.clone();
        publisher.send(make_event(KeyCode::A));
        publisher2.send(make_event(KeyCode::B));
        drop(publisher);
        drop(publisher2);
        let received: Vec<KeyCode> = subscriber.map(|e| e.key).collect();
        assert_eq!(received.len(), 2);
    }

    /// Gate test: 10k events, no drops, throughput logged.
    #[test]
    fn throughput_10k_no_drops() {
        let _ = env_logger::try_init();
        const N: usize = 10_000;
        // Channel sized to N so all sends are non-blocking; proves the bus
        // can absorb a full burst without any drops.
        let (publisher, subscriber) = new(N);

        let start = Instant::now();

        let sender_thread = std::thread::spawn(move || {
            for _ in 0..N {
                publisher.send(make_event(KeyCode::A));
            }
            // publisher drops here, signalling subscriber to drain and exit
        });

        let received: Vec<InputEvent> = subscriber.collect();
        sender_thread.join().unwrap();

        let elapsed = start.elapsed();
        let throughput = N as f64 / elapsed.as_secs_f64();
        log::info!(
            "event_bus throughput: {:.0} events/s ({} events in {:.3}ms)",
            throughput,
            N,
            elapsed.as_secs_f64() * 1000.0
        );

        assert_eq!(received.len(), N, "expected no drops");
        // Hard gate: the bus must flush 10k events within the 33ms per-frame
        // budget so it cannot be a pipeline bottleneck.
        assert!(
            elapsed < std::time::Duration::from_millis(33),
            "event_bus throughput gate failed: {:.3}ms > 33ms",
            elapsed.as_secs_f64() * 1000.0
        );
    }
}
