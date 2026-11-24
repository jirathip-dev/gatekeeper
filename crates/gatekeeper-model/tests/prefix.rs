use gatekeeper_model::Prefix;

fn claim(prefix: &str, span: &str) -> Option<String> {
    let p = Prefix::try_from(prefix.to_owned()).ok()?;
    p.claim(span).map(str::to_owned)
}

#[test]
fn a_prefix_claims_the_word_after_it() {
    let cases = [
        ("tool {name}", "tool spawn --dry-run <f>", Some("spawn")),
        ("tool {name}", "tool queue", Some("queue")),
        ("--{name}", "--dry-run", Some("dry-run")),
        (
            "[limits] {name}",
            "[limits] stall_minutes = 30",
            Some("stall_minutes"),
        ),
        ("core/{name}", "core/pr-body.", Some("pr-body")),
        ("tool {name}", "tool 2fa", Some("2fa")),
        ("tool {name}", "tool <verb>", None),
        ("tool {name}", "tool …", None),
        ("tool {name}", "tool *", None),
        ("tool {name}", "tool --flag", None),
        ("tool {name}", "tool ", None),
        ("tool {name}", "cargo build --locked", None),
        ("tool {name}", "./tool spawn", None),
    ];
    for (prefix, span, want) in cases {
        assert_eq!(claim(prefix, span).as_deref(), want, "{prefix} on {span}");
    }
}
