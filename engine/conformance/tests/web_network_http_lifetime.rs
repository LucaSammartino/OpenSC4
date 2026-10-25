//! web-network: http owner conformance vectors.
use fc4_core::www::http::*;

mod tests {
    use super::*;
    #[derive(Default)]
    struct Fake {
        effects: Vec<Effect>,
        observed: Vec<(Job, Vec<u32>)>,
    }
    impl Host for Fake {
        fn effect(&mut self, effect: Effect) {
            self.effects.push(effect);
        }
        fn notify(&mut self, client: &mut Client, job: Job, _: bool) {
            self.observed.push((
                job,
                client
                    .requests
                    .values()
                    .map(|r| r.refs)
                    .chain(client.resources.values().map(|r| r.refs))
                    .collect(),
            ));
        }
    }
    #[test]
    fn method_prefix_and_wrapper_boundaries() {
        assert_eq!(method_id(b"GETaway", 1), 2);
        assert_eq!(method_id(b"GETaway", u32::MAX), 0);
        assert_eq!(method_id(b"anything", 0), 1);
        for id in 1..=10 {
            assert_eq!(method_id(method_name(id).unwrap(), u32::MAX), id);
        }
        let mut client = Client::new(None);
        let mut host = Fake::default();
        assert_eq!(client.submit(&mut host, 8, b"url", None, 0, &[], 0), Ok(0));
        assert!(host.effects.is_empty());
        assert_eq!(client.submit(&mut host, 5, b"url", None, 0, &[], 0), Ok(0));
        assert!(client.requests.is_empty());
        assert_eq!(client.admit(&mut host, None, 0), Ok(0));
        assert_eq!(client.submit(&mut host, 5, b"url", Some(b"data"), 0, &[], 0), Ok(1));
        assert_eq!(client.requests.values().next().unwrap().body, b"data");
    }
    #[test]
    fn admission_wrap_raw_cancel_leaks_and_first_match_wins() {
        let mut client = Client::new(None);
        let mut host = Fake::default();
        client.counter = u32::MAX;
        assert_eq!(client.submit(&mut host, 2, b"url", None, 0, &[], 8), Ok(0));
        let job = client.queue[0];
        assert_eq!(job.0[7], 8);
        assert_eq!(client.requests[&job.0[5]].refs, 1);
        client.append(&mut host, job);
        let worker = client.active.unwrap();
        client.workers.get_mut(&worker).unwrap().current = Some(job);
        assert!(client.cancel(&mut host, 0));
        assert_eq!(client.queue.len(), 1);
        assert!(!client.workers[&worker].stop);
        assert_eq!(client.requests[&job.0[5]].refs, 1);
        assert!(client.cancel(&mut host, 0));
        assert!(client.cancel(&mut host, 0));
        assert!(client.workers[&worker].stop);
        assert!(client.active.is_none());
        assert!(!client.cancel(&mut host, 99));
    }
    fn resources(client: &mut Client) -> Job {
        client.requests.insert(
            50,
            Request {
                refs: 2,
                method: b"GET".to_vec(),
                url: b"url".to_vec(),
                redirects: 3,
                ..Request::default()
            },
        );
        client.resources.insert(
            51,
            Resource {
                refs: 2,
                ..Resource::default()
            },
        );
        Job([9, 0, 0, 1, 0, 50, 51, 0])
    }
    #[test]
    fn cancellation_callback_before_release_and_stop_leaks() {
        for stop in [false, true] {
            let mut client = Client::new(None);
            client.target = Some(1);
            let mut host = Fake::default();
            let job = resources(&mut client);
            client.workers.insert(
                1,
                Worker {
                    running: true,
                    cancel: true,
                    stop,
                    current: Some(job),
                    socket: 0,
                },
            );
            client.final_boundary(&mut host, 1).unwrap();
            assert!(!client.workers[&1].cancel);
            assert!(client.workers[&1].current.is_none());
            if stop {
                assert!(host.observed.is_empty());
                assert_eq!(client.requests[&50].refs, 2);
            } else {
                assert_eq!(host.observed[0].1, [2, 2]);
                assert_eq!(host.observed[0].0.status(), 7);
                assert_eq!(client.requests[&50].refs, 1);
                assert_eq!(client.resources[&51].strings[1], b"url");
                assert_eq!(host.observed[0].0 .0[4], 100);
                assert_eq!(
                    host.effects
                        .iter()
                        .filter(|e| matches!(e, Effect::ReleaseRequest(_) | Effect::ReleaseResource(_)))
                        .cloned()
                        .collect::<Vec<_>>(),
                    [Effect::ReleaseRequest(50), Effect::ReleaseResource(51)]
                );
            }
        }
    }
    #[test]
    fn shutdown_replacement_then_raw_drain_no_join() {
        let mut client = Client::new(None);
        let mut host = Fake::default();
        client.submit(&mut host, 2, b"url", None, 0, &[], 0).unwrap();
        let old = client.active.unwrap();
        assert!(client.shutdown(&mut host));
        assert_eq!(client.queue.len(), 1);
        client.initialized = true;
        client.workers.get_mut(&old).unwrap().socket = 88;
        assert!(client.shutdown(&mut host));
        let replacement = client.active.unwrap();
        assert_ne!(replacement, old);
        assert!(client.queue.is_empty());
        assert!(client.workers[&old].stop);
        assert!(host.effects.contains(&Effect::CloseSocket(88)));
        host.effects.clear();
        assert_eq!(client.worker_exit(&mut host, old), Ok(()));
        assert_eq!(
            host.effects,
            [
                Effect::Lock,
                Effect::CloseSocket(88),
                Effect::DestroySocket(88),
                Effect::FreeWorker(old),
                Effect::Unlock,
                Effect::ReleaseClient
            ]
        );
        assert!(client.active.is_none());
        assert!(client.workers.contains_key(&replacement));
    }
    #[test]
    fn zero_and_one_request_ref_delete() {
        let mut client = Client::new(None);
        let mut host = Fake::default();
        client.requests.insert(
            1,
            Request {
                refs: 2,
                ..Request::default()
            },
        );
        assert_eq!(client.release_request(&mut host, 1), Ok(1));
        assert_eq!(client.release_request(&mut host, 1), Ok(0));
        client.requests.insert(2, Request::default());
        assert_eq!(client.release_request(&mut host, 2), Ok(0));
        assert_eq!(client.timeout, 15);
        assert_eq!(client.suffix, b".tmp");
        assert_eq!(client.lookup, None);
    }
}
