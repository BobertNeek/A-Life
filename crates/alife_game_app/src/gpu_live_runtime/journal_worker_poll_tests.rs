//! Polling must not wait for a publisher that has sent its last message.
use super::*;
use std::time::Duration;

struct HeldPublicationWorker {
    release: Option<SyncSender<()>>,
    owner: Option<SleepJournalPublicationWorkerOwnerV1>,
}

impl Drop for HeldPublicationWorker {
    fn drop(&mut self) {
        // Disconnect before the owner joins, including on an assertion panic.
        self.release.take();
    }
}

fn assert_poll_does_not_wait_for_exit(send_result: bool) {
    let (sender, receiver) = mpsc::sync_channel(1);
    let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
    let (release, exit_receiver) = mpsc::sync_channel::<()>(1);
    let join_handle = thread::spawn(move || {
        if send_result {
            sender
                .send(SleepJournalPublicationWorkerFinalV1 {
                    result: Err(ScaffoldContractError::InvalidId.into()),
                    expected_base_digest: "held-publication-result".to_string(),
                    expected_base_generation: Some(7),
                    entry_count: 3,
                    worker_wall_ns: 0,
                })
                .unwrap();
        }
        drop(sender);
        let _ = ready_sender.send(());
        // Both a ready result and disconnection precede actual thread exit.
        let _ = exit_receiver.recv();
    });
    let mut held = HeldPublicationWorker {
        release: Some(release),
        owner: Some(SleepJournalPublicationWorkerOwnerV1 {
            receiver,
            join_handle: Some(join_handle),
            incremental_entry_count: 3,
        }),
    };
    ready_receiver
        .recv_timeout(Duration::from_secs(10))
        .unwrap();

    let mut owner = held.owner.take().unwrap();
    let (polled_sender, polled_receiver) = mpsc::sync_channel(1);
    let caller = thread::spawn(move || {
        let result = owner.poll();
        let _ = polled_sender.send(matches!(
            result,
            SleepJournalPublicationWorkerPollV1::Pending
        ));
        (owner, result)
    });
    // This is a synchronization deadline, not a latency or p99 benchmark.
    let returned_before_exit = polled_receiver.recv_timeout(Duration::from_secs(2));
    held.release.take();
    let (mut owner, _) = caller.join().unwrap();
    assert_eq!(
        returned_before_exit.ok(),
        Some(true),
        "poll must return Pending while worker exit is deliberately withheld"
    );

    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match owner.poll() {
            SleepJournalPublicationWorkerPollV1::Pending => {
                assert!(Instant::now() < deadline, "released worker did not exit");
                thread::yield_now();
            }
            SleepJournalPublicationWorkerPollV1::Ready(report) => {
                assert!(send_result);
                assert_eq!(report.expected_base_digest, "held-publication-result");
                assert_eq!(report.expected_base_generation, Some(7));
                assert_eq!(report.entry_count, 3);
                assert!(matches!(
                    report.result,
                    Err(GameAppShellError::Core(ScaffoldContractError::InvalidId))
                ));
                return;
            }
            SleepJournalPublicationWorkerPollV1::Panicked => {
                assert!(!send_result, "a sent result must survive until thread exit");
                return;
            }
        }
    }
}

#[test]
fn ready_publication_result_does_not_make_poll_join_a_live_worker() {
    assert_poll_does_not_wait_for_exit(true);
}

#[test]
fn disconnected_publication_channel_does_not_make_poll_join_a_live_worker() {
    assert_poll_does_not_wait_for_exit(false);
}
