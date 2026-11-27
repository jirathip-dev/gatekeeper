A lane's check never asserts a time: perf, soak, bench and wall-clock limits are CI's.
A gate that measures time gives `needs = ["quiet-host"]`: a lane's host skips it with `CI only on a lane's host: needs a quiet host`, and CI runs it.
A test inside an ordinary gate reads `GATEKEEPER_HOST`, `lane` or `ci`: on `lane` it asserts the outcome and prints the time, on `ci` it asserts the limit too.
A gate's `run` that reads `CI` or `GITHUB_ACTIONS` to skip itself is a config error: use `needs = ["quiet-host"]`.
