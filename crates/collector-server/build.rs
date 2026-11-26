//! Makes sure the web assets this server embeds were built from the current
//! `web/src`. `web/build.mjs` writes a fingerprint of its inputs to
//! `web/dist/source-digest`; this script recomputes it. When they differ and Node
//! with the web dependencies is available, it rebuilds `web/dist` (under a second).
//! Otherwise it stops the build and says how to fix it, so a server never ships
//! old screens by accident. Builds that only have `web/dist` (the container build)
//! skip the check; CI checks the committed assets match `web/src`.
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let web = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../web");
    let digest_file = web.join("dist/source-digest");
    println!("cargo:rerun-if-changed={}", digest_file.display());
    println!("cargo:rerun-if-env-changed=COLLECTOR_SKIP_WEB_BUILD");
    if !web.join("src").is_dir() {
        return;
    }
    for input in ["src", "build.mjs", "package-lock.json"] {
        println!("cargo:rerun-if-changed={}", web.join(input).display());
    }
    let expected = source_digest(&web);
    if recorded(&digest_file) == expected {
        return;
    }
    let skip = std::env::var_os("COLLECTOR_SKIP_WEB_BUILD").is_some();
    if !skip && web.join("node_modules/esbuild").is_dir() {
        let node = std::env::var_os("NODE").unwrap_or_else(|| "node".into());
        let status = Command::new(node)
            .arg("build.mjs")
            .current_dir(&web)
            .status();
        match status {
            Ok(status) if status.success() && recorded(&digest_file) == expected => return,
            Ok(status) => panic!(
                "web/build.mjs failed ({status}); fix it and run `npm --prefix web run build`"
            ),
            Err(error) => println!("cargo:warning=could not run node to rebuild web/dist: {error}"),
        }
    }
    panic!(
        "web/dist is out of date with web/src, so this server would serve old screens. \
         Run `npm --prefix web ci && npm --prefix web run build` and build again."
    );
}

fn recorded(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_default()
        .trim()
        .to_owned()
}

/// Same fingerprint as `web/source-digest.mjs`: FNV-1a 64 over each file's
/// relative path, a zero byte, its contents and a zero byte, in byte order of path.
fn source_digest(web: &Path) -> String {
    let mut files = vec!["build.mjs".to_owned(), "package-lock.json".to_owned()];
    collect(web, Path::new("src"), &mut files);
    files.sort();
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut add = |bytes: &[u8]| {
        for byte in bytes {
            hash = (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3);
        }
    };
    for file in files {
        let contents =
            fs::read(web.join(&file)).unwrap_or_else(|error| panic!("read web/{file}: {error}"));
        add(file.as_bytes());
        add(&[0]);
        add(&contents);
        add(&[0]);
    }
    format!("{hash:016x}")
}

fn collect(web: &Path, directory: &Path, files: &mut Vec<String>) {
    const EXTENSIONS: [&str; 8] = ["ts", "tsx", "css", "js", "mjs", "json", "html", "svg"];
    let entries = fs::read_dir(web.join(directory))
        .unwrap_or_else(|error| panic!("read {}: {error}", directory.display()));
    for entry in entries.map(|entry| entry.expect("read web/src entry")) {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let relative: PathBuf = directory.join(&name);
        if entry.file_type().expect("file type").is_dir() {
            collect(web, &relative, files);
        } else if relative
            .extension()
            .is_some_and(|ext| EXTENSIONS.iter().any(|known| ext == *known))
        {
            files.push(relative.to_string_lossy().replace('\\', "/"));
        }
    }
}
