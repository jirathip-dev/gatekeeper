func load() -> Int { 1 }

func run() -> Int {
    let stored: Any = load()
    return stored as! Int
}
