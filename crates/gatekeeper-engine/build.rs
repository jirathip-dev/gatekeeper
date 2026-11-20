use std::fmt::Write as _;
use std::hash::{DefaultHasher, Hasher};
use std::path::{Path, PathBuf};
use std::{env, fs, io};

fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            walk(&path, out)?;
        } else {
            out.push(path);
        }
    }
    Ok(())
}

fn main() -> io::Result<()> {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").map_err(io::Error::other)?);
    let packs = manifest.join("../../packs").canonicalize()?;
    println!("cargo::rerun-if-changed={}", packs.display());
    let mut files = Vec::new();
    walk(&packs, &mut files)?;
    files.sort();
    let mut src = String::from("pub static FILES: &[(&str, &[u8])] = &[\n");
    for file in files {
        let rel = file.strip_prefix(&packs).map_err(io::Error::other)?;
        let rel = rel.to_string_lossy().replace('\\', "/");
        writeln!(
            src,
            "    ({rel:?}, include_bytes!({:?})),",
            file.display().to_string()
        )
        .map_err(io::Error::other)?;
    }
    src.push_str("];\n");
    let mut sources = Vec::new();
    for dir in [
        "../../packs",
        "../gatekeeper-engine/src",
        "../gatekeeper-model/src",
        "../gatekeeper/src",
    ] {
        let dir = manifest.join(dir).canonicalize()?;
        println!("cargo::rerun-if-changed={}", dir.display());
        walk(&dir, &mut sources)?;
    }
    sources.push(manifest.join("../../Cargo.lock").canonicalize()?);
    sources.sort();
    let mut hash = DefaultHasher::new();
    for file in &sources {
        hash.write(
            file.strip_prefix(packs.join(".."))
                .unwrap_or(file)
                .to_string_lossy()
                .as_bytes(),
        );
        hash.write(&fs::read(file)?);
    }
    println!("cargo::rustc-env=GATEKEEPER_SOURCE={:016x}", hash.finish());
    let out = PathBuf::from(env::var("OUT_DIR").map_err(io::Error::other)?);
    fs::write(out.join("packs.rs"), src)
}
