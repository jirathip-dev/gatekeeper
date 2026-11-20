use gatekeeper_engine::cache::mode;
use gatekeeper_model::{CacheConfig, CacheMode, Event, Off, PullRequests, Tier};

fn config(pull_requests: &str) -> CacheConfig {
    toml::from_str(&format!("pull_requests = \"{pull_requests}\"")).expect("config")
}

fn holds(event: &Event, read: &[Tier], write: Option<Tier>, at: &str) {
    match event {
        Event::PullRequest {
            number,
            fork: false,
        } => {
            assert_ne!(write, Some(Tier::Integration), "{at}");
            assert!(
                write.is_none_or(|t| t == Tier::PullRequest(*number)),
                "{at}"
            );
            let own = |t: &Tier| *t == Tier::Integration || *t == Tier::PullRequest(*number);
            assert!(read.iter().all(own), "{at}");
        }
        Event::Push { branch } if branch == "staging" => {
            assert!(read.is_empty(), "{at}");
            assert_eq!(write, Some(Tier::Integration), "{at}");
        }
        Event::Lane => assert!(read.iter().chain(&write).all(|t| *t == Tier::Host), "{at}"),
        _ => panic!("{at}: this run must not use the cache"),
    }
}

#[test]
fn no_pull_request_writes_the_integration_tier_and_no_push_reads() {
    let events = [
        Event::Lane,
        Event::PullRequest {
            number: 41,
            fork: false,
        },
        Event::PullRequest {
            number: 41,
            fork: true,
        },
        Event::Push {
            branch: "staging".into(),
        },
        Event::Push {
            branch: "main".into(),
        },
        Event::Other,
    ];
    for prs in ["own", "read"] {
        let cfg = config(prs);
        assert_eq!(
            cfg.pull_requests,
            if prs == "own" {
                PullRequests::Own
            } else {
                PullRequests::Read
            }
        );
        for event in &events {
            for full in [false, true] {
                for no_cache in [false, true] {
                    let got = mode(Some(&cfg), event, full, no_cache, "staging");
                    let at = format!("{prs} {event:?} full={full} no_cache={no_cache}: {got:?}");
                    let CacheMode::Use { read, write } = &got else {
                        continue;
                    };
                    assert!(!no_cache, "{at}");
                    assert!(!full || read.is_empty(), "{at}");
                    holds(event, read, *write, &at);
                }
            }
        }
    }
}

#[test]
fn a_pull_request_reads_integration_then_its_own_and_unconfigured_is_off() {
    let lane = mode(None, &Event::Lane, false, false, "staging");
    assert_eq!(lane, CacheMode::Off(Off::NotConfigured));
    let pr = Event::PullRequest {
        number: 41,
        fork: false,
    };
    assert_eq!(
        mode(Some(&config("own")), &pr, false, false, "staging"),
        CacheMode::Use {
            read: vec![Tier::Integration, Tier::PullRequest(41)],
            write: Some(Tier::PullRequest(41))
        }
    );
    assert_eq!(
        mode(Some(&config("read")), &pr, true, false, "staging"),
        CacheMode::Use {
            read: Vec::new(),
            write: None
        }
    );
}
