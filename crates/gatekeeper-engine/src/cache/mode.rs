use gatekeeper_model::{CacheConfig, CacheMode, Event, Off, PullRequests, Tier};

/// Which tiers a run reads and writes: no pull request writes the integration tier, no push reads.
#[must_use]
pub fn mode(
    config: Option<&CacheConfig>,
    event: &Event,
    full: bool,
    no_cache: bool,
    integration: &str,
) -> CacheMode {
    let Some(config) = config else {
        return CacheMode::Off(Off::NotConfigured);
    };
    if no_cache {
        return CacheMode::Off(Off::Flag);
    }
    let reads = |tiers: Vec<Tier>| if full { Vec::new() } else { tiers };
    match event {
        Event::Lane => CacheMode::Use {
            read: reads(vec![Tier::Host]),
            write: Some(Tier::Host),
        },
        Event::PullRequest { fork: true, .. } => CacheMode::Off(Off::Fork),
        Event::PullRequest { number, .. } => {
            let own = Tier::PullRequest(*number);
            match config.pull_requests {
                PullRequests::Own => CacheMode::Use {
                    read: reads(vec![Tier::Integration, own]),
                    write: Some(own),
                },
                PullRequests::Read => CacheMode::Use {
                    read: reads(vec![Tier::Integration]),
                    write: None,
                },
            }
        }
        Event::Push { branch } if branch == integration => CacheMode::Use {
            read: Vec::new(),
            write: Some(Tier::Integration),
        },
        Event::Push { .. } | Event::Other => CacheMode::Off(Off::NotAPullRequest),
    }
}
