Pin bump: drover files one when `./.gatekeeper/run pin --outdated` exits 1 (`pinned <old>, latest <new>`).
Run `./.gatekeeper/run pin latest`, then `./.gatekeeper/run check`.
Fix each new finding where the fix is in reach; otherwise run `./.gatekeeper/run baseline <pack>/<gate>` for it: a PR that moves `[pin]` may grow a baseline (`core/ratchet`).
Never turn a gate off in `[off]` without a reason line.
