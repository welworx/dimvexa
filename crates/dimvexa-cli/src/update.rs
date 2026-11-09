// SPDX-License-Identifier: MIT OR Apache-2.0
//! `dimvexa update`: replace the running binary with the latest GitHub release (ADR-015).
//!
//! The workspace has no dependencies (ADR-003), so there is no HTTP client here: the download
//! runs the system `curl`, the unpacking the system `tar`, and the SHA-256 check is a small
//! implementation in this file. The release workflow (`.github/workflows/release.yml`) names the
//! assets, and this module reads those names.

// Items are `pub` so the parent module (and nothing else) uses them; the module is private, and
// the two visibility lints disagree about `pub(crate)` here.
#![allow(unreachable_pub, clippy::redundant_pub_crate)]

use std::ffi::{OsStr, OsString};
use std::fmt::Write as _;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

/// The GitHub releases page of the project.
const RELEASES: &str = "https://github.com/welworx/dimvexa/releases";

/// Exit code of `update --check` when a newer release exists (0 means up to date, 1 an error).
pub const EXIT_UPDATE_AVAILABLE: u8 = 3;

/// The release target triple of this build, or `None` when no binary is published for it.
const fn release_target() -> Option<&'static str> {
    if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        Some("aarch64-apple-darwin")
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        Some("x86_64-apple-darwin")
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        Some("x86_64-unknown-linux-musl")
    } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
        Some("aarch64-unknown-linux-musl")
    } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        Some("x86_64-pc-windows-msvc")
    } else {
        None
    }
}

/// Parses `MAJOR.MINOR.PATCH` (an optional leading `v`; a `-pre` or `+build` suffix is ignored).
fn parse_version(s: &str) -> Option<(u64, u64, u64)> {
    let s = s.strip_prefix('v').unwrap_or(s);
    let core = s.split(['-', '+']).next()?;
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

/// Takes the tag out of the URL GitHub redirects `releases/latest` to
/// (`.../releases/tag/v0.3.0`). Only a plain version tag is accepted, because the tag is put
/// into the download URLs.
fn parse_release_tag(url: &str) -> Option<String> {
    let (_, rest) = url.split_once("/releases/tag/")?;
    let tag = rest.split(['?', '#']).next()?.trim_end_matches('/');
    parse_version(tag)?;
    let plain = tag
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+'));
    plain.then(|| tag.to_string())
}

/// Why an installation must not be replaced by this command, as the advice to print; `None`
/// when it may be.
fn managed_elsewhere(exe: &Path, cargo_home: Option<&Path>) -> Option<&'static str> {
    let text = exe.to_string_lossy().replace('\\', "/");
    let brew = text.contains("/Cellar/")
        || text.starts_with("/opt/homebrew/")
        || text.starts_with("/usr/local/Homebrew/")
        || text.starts_with("/home/linuxbrew/");
    if brew {
        return Some("this dimvexa was installed with Homebrew; run: brew upgrade dimvexa");
    }
    let in_cargo_home =
        cargo_home.is_some_and(|home| exe.parent() == Some(home.join("bin").as_path()));
    if in_cargo_home || text.contains("/.cargo/bin/") {
        return Some(
            "this dimvexa was installed with cargo; run: cargo install dimvexa-cli --force",
        );
    }
    None
}

// ---------------------------------------------------------------------------------------------
// SHA-256 (FIPS 180-4), written without indexing so it follows the workspace lints.

#[rustfmt::skip]
const K: [u32; 64] = [
    0x428a_2f98, 0x7137_4491, 0xb5c0_fbcf, 0xe9b5_dba5,
    0x3956_c25b, 0x59f1_11f1, 0x923f_82a4, 0xab1c_5ed5,
    0xd807_aa98, 0x1283_5b01, 0x2431_85be, 0x550c_7dc3,
    0x72be_5d74, 0x80de_b1fe, 0x9bdc_06a7, 0xc19b_f174,
    0xe49b_69c1, 0xefbe_4786, 0x0fc1_9dc6, 0x240c_a1cc,
    0x2de9_2c6f, 0x4a74_84aa, 0x5cb0_a9dc, 0x76f9_88da,
    0x983e_5152, 0xa831_c66d, 0xb003_27c8, 0xbf59_7fc7,
    0xc6e0_0bf3, 0xd5a7_9147, 0x06ca_6351, 0x1429_2967,
    0x27b7_0a85, 0x2e1b_2138, 0x4d2c_6dfc, 0x5338_0d13,
    0x650a_7354, 0x766a_0abb, 0x81c2_c92e, 0x9272_2c85,
    0xa2bf_e8a1, 0xa81a_664b, 0xc24b_8b70, 0xc76c_51a3,
    0xd192_e819, 0xd699_0624, 0xf40e_3585, 0x106a_a070,
    0x19a4_c116, 0x1e37_6c08, 0x2748_774c, 0x34b0_bcb5,
    0x391c_0cb3, 0x4ed8_aa4a, 0x5b9c_ca4f, 0x682e_6ff3,
    0x748f_82ee, 0x78a5_636f, 0x84c8_7814, 0x8cc7_0208,
    0x90be_fffa, 0xa450_6ceb, 0xbef9_a3f7, 0xc671_78f2,
];

const fn small_sigma0(x: u32) -> u32 {
    x.rotate_right(7) ^ x.rotate_right(18) ^ (x >> 3)
}

const fn small_sigma1(x: u32) -> u32 {
    x.rotate_right(17) ^ x.rotate_right(19) ^ (x >> 10)
}

/// SHA-256 of `data`.
fn sha256(data: &[u8]) -> [u8; 32] {
    let mut state: [u32; 8] = [
        0x6a09_e667,
        0xbb67_ae85,
        0x3c6e_f372,
        0xa54f_f53a,
        0x510e_527f,
        0x9b05_688c,
        0x1f83_d9ab,
        0x5be0_cd19,
    ];
    let mut msg = Vec::with_capacity(data.len().saturating_add(72));
    msg.extend_from_slice(data);
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    let bits = u64::try_from(data.len()).map_or(0, |n| n.wrapping_mul(8));
    msg.extend_from_slice(&bits.to_be_bytes());

    for block in msg.chunks_exact(64) {
        let mut win = [0_u32; 16];
        for (slot, bytes) in win.iter_mut().zip(block.chunks_exact(4)) {
            *slot = u32::from_be_bytes(<[u8; 4]>::try_from(bytes).unwrap_or([0; 4]));
        }
        let [s0, s1, s2, s3, s4, s5, s6, s7] = state;
        let (mut va, mut vb, mut vc, mut vd) = (s0, s1, s2, s3);
        let (mut ve, mut vf, mut vg, mut vh) = (s4, s5, s6, s7);
        for k in K {
            // `win` is a window of the message schedule: win[0] is the word of this round.
            let [w0, w1, _, _, _, _, _, _, _, w9, _, _, _, _, w14, _] = win;
            let big1 = ve.rotate_right(6) ^ ve.rotate_right(11) ^ ve.rotate_right(25);
            let choose = (ve & vf) ^ (!ve & vg);
            let t1 = vh
                .wrapping_add(big1)
                .wrapping_add(choose)
                .wrapping_add(k)
                .wrapping_add(w0);
            let big0 = va.rotate_right(2) ^ va.rotate_right(13) ^ va.rotate_right(22);
            let majority = (va & vb) ^ (va & vc) ^ (vb & vc);
            let t2 = big0.wrapping_add(majority);
            vh = vg;
            vg = vf;
            vf = ve;
            ve = vd.wrapping_add(t1);
            vd = vc;
            vc = vb;
            vb = va;
            va = t1.wrapping_add(t2);
            let next = small_sigma1(w14)
                .wrapping_add(w9)
                .wrapping_add(small_sigma0(w1))
                .wrapping_add(w0);
            win.rotate_left(1);
            if let Some(last) = win.last_mut() {
                *last = next;
            }
        }
        state = [
            s0.wrapping_add(va),
            s1.wrapping_add(vb),
            s2.wrapping_add(vc),
            s3.wrapping_add(vd),
            s4.wrapping_add(ve),
            s5.wrapping_add(vf),
            s6.wrapping_add(vg),
            s7.wrapping_add(vh),
        ];
    }

    let mut out = [0_u8; 32];
    for (slot, byte) in out
        .iter_mut()
        .zip(state.iter().flat_map(|x| x.to_be_bytes()))
    {
        *slot = byte;
    }
    out
}

fn sha256_hex(data: &[u8]) -> String {
    let mut s = String::with_capacity(64);
    for b in sha256(data) {
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// Checks `data` against the first line of a `sha256sum` file (`<hex>  <name>`, the name may
/// carry a `*`). The name must be `name`, so a checksum of another file is not accepted.
fn verify_checksum(data: &[u8], sums: &str, name: &str) -> Result<(), String> {
    let mut fields = sums.split_whitespace();
    let (Some(hash), Some(file)) = (fields.next(), fields.next()) else {
        return Err(format!("the checksum file for {name} is malformed"));
    };
    if fields.next().is_some() || file.trim_start_matches('*') != name {
        return Err(format!("the checksum file does not describe {name}"));
    }
    if sha256_hex(data) == hash.to_ascii_lowercase() {
        Ok(())
    } else {
        Err(format!(
            "checksum mismatch for {name}; nothing was installed"
        ))
    }
}

// ---------------------------------------------------------------------------------------------
// Network and files.

/// What the update needs from the network; the real one runs `curl`, the tests use a folder.
pub trait Net {
    /// The tag of the latest release, for example `v0.3.0`.
    fn latest_tag(&self) -> Result<String, String>;
    /// Downloads the asset `name` of release `tag` to `dest`.
    fn download(&self, tag: &str, name: &str, dest: &Path) -> Result<(), String>;
}

/// The network through the system `curl` (HTTPS only, also after redirects).
pub struct Curl;

impl Curl {
    fn run(args: &[&OsStr]) -> Result<Vec<u8>, String> {
        let out = Command::new("curl")
            .args(["--proto", "=https", "--proto-redir", "=https", "--tlsv1.2"])
            .args(["--fail", "--silent", "--show-error", "--location"])
            .args(["--max-time", "300"])
            .args(args)
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    "'dimvexa update' needs curl, which was not found on PATH".to_string()
                } else {
                    format!("could not run curl: {e}")
                }
            })?;
        if out.status.success() {
            Ok(out.stdout)
        } else {
            let msg = String::from_utf8_lossy(&out.stderr);
            Err(format!("curl failed: {}", msg.trim()))
        }
    }
}

impl Net for Curl {
    fn latest_tag(&self) -> Result<String, String> {
        let null = if cfg!(windows) { "NUL" } else { "/dev/null" };
        let url = format!("{RELEASES}/latest");
        let out = Self::run(&[
            OsStr::new("--head"),
            OsStr::new("--output"),
            OsStr::new(null),
            OsStr::new("--write-out"),
            OsStr::new("%{url_effective}"),
            OsStr::new(&url),
        ])?;
        let landed = String::from_utf8_lossy(&out).into_owned();
        parse_release_tag(&landed)
            .ok_or_else(|| format!("no release found (curl ended at {landed})"))
    }

    fn download(&self, tag: &str, name: &str, dest: &Path) -> Result<(), String> {
        let url = format!("{RELEASES}/download/{tag}/{name}");
        Self::run(&[OsStr::new("--output"), dest.as_os_str(), OsStr::new(&url)]).map(|_| ())
    }
}

/// A scratch folder that is removed when it goes out of scope.
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Result<Self, String> {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.subsec_nanos());
        // `create_dir` (not `create_dir_all`) refuses a folder someone else planted; the counter
        // keeps names apart within one process, a clash with another process is retried.
        for _ in 0..16 {
            let n = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir()
                .join(format!("dimvexa-update-{}-{nanos}-{n}", std::process::id()));
            match std::fs::create_dir(&path) {
                Ok(()) => return Ok(Self(path)),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(e) => return Err(format!("cannot create {}: {e}", path.display())),
            }
        }
        Err("cannot create a scratch folder".to_string())
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Unpacks a `.tar.gz` with the system `tar`.
fn untar(archive: &Path, into: &Path) -> Result<(), String> {
    let out = Command::new("tar")
        .arg("-xzf")
        .arg(archive)
        .arg("-C")
        .arg(into)
        .output()
        .map_err(|e| format!("could not run tar: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!(
            "tar failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    }
}

/// Installs `new` over `target`: a copy next to it (so the last step is a rename on one
/// filesystem), then `target` is renamed to `.old` and the copy to `target`. Renaming a running
/// executable away is allowed on Windows too, where deleting it is not. A failure puts `target`
/// back.
fn replace_binary(new: &Path, target: &Path) -> Result<(), String> {
    let dir = target
        .parent()
        .ok_or_else(|| format!("{} has no folder", target.display()))?;
    let staged = dir.join(format!(".dimvexa-update-{}", std::process::id()));
    let denied = |e: std::io::Error| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            format!(
                "no permission to write in {}; run the update with enough rights",
                dir.display()
            )
        } else {
            format!("cannot install into {}: {e}", dir.display())
        }
    };
    if let Err(e) = std::fs::copy(new, &staged) {
        let _ = std::fs::remove_file(&staged);
        return Err(denied(e));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        if let Err(e) = std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755)) {
            let _ = std::fs::remove_file(&staged);
            return Err(format!("cannot make the new binary executable: {e}"));
        }
    }
    let mut old_name = OsString::from(target);
    old_name.push(".old");
    let old = PathBuf::from(old_name);
    let _ = std::fs::remove_file(&old);
    if let Err(e) = std::fs::rename(target, &old) {
        let _ = std::fs::remove_file(&staged);
        return Err(denied(e));
    }
    if let Err(e) = std::fs::rename(&staged, target) {
        let _ = std::fs::remove_file(&staged);
        return match std::fs::rename(&old, target) {
            Ok(()) => Err(denied(e)),
            Err(back) => Err(format!(
                "install failed ({e}) and putting {} back failed ({back})",
                target.display()
            )),
        };
    }
    let _ = std::fs::remove_file(&old);
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// The command.

/// The facts the update needs about this process, passed in so the tests can choose them.
pub struct Env<'a> {
    /// The running version, for example `0.2.0`.
    pub current: &'a str,
    /// The release target triple, for example `x86_64-unknown-linux-musl`.
    pub target: &'a str,
    /// Whether the asset is a bare `dimvexa.exe` instead of a `.tar.gz`.
    pub windows: bool,
    /// The binary to replace.
    pub exe: &'a Path,
}

/// Runs `dimvexa update [--check] [--yes]` on the real system.
pub fn run(check: bool, yes: bool, out: &mut dyn Write, err: &mut dyn Write) -> u8 {
    let Some(target) = release_target() else {
        let _ = writeln!(
            err,
            "error: no prebuilt dimvexa is published for this platform; use: cargo install dimvexa-cli --force"
        );
        return 1;
    };
    let exe = match std::env::current_exe().and_then(std::fs::canonicalize) {
        Ok(p) => p,
        Err(e) => {
            let _ = writeln!(err, "error: cannot find the running binary: {e}");
            return 1;
        }
    };
    if !check
        && let Some(advice) = managed_elsewhere(
            &exe,
            std::env::var_os("CARGO_HOME").map(PathBuf::from).as_deref(),
        )
    {
        let _ = writeln!(err, "error: {advice}");
        return 1;
    }
    let env = Env {
        current: env!("CARGO_PKG_VERSION"),
        target,
        windows: cfg!(windows),
        exe: &exe,
    };
    let stdin = std::io::stdin();
    update_with(&Curl, &env, check, yes, &mut stdin.lock(), out, err)
}

/// The update flow against any [`Net`]. Returns the exit code: 0 done or up to date, 1 error,
/// [`EXIT_UPDATE_AVAILABLE`] for `--check` with a newer release.
pub fn update_with(
    net: &dyn Net,
    env: &Env<'_>,
    check: bool,
    yes: bool,
    input: &mut dyn BufRead,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    match flow(net, env, check, yes, input, out, err) {
        Ok(code) => code,
        Err(message) => {
            let _ = writeln!(err, "error: {message}");
            1
        }
    }
}

fn flow(
    net: &dyn Net,
    env: &Env<'_>,
    check: bool,
    yes: bool,
    input: &mut dyn BufRead,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<u8, String> {
    let tag = net.latest_tag()?;
    let latest = parse_version(&tag).ok_or_else(|| format!("unreadable release tag {tag}"))?;
    let current = parse_version(env.current)
        .ok_or_else(|| format!("unreadable running version {}", env.current))?;
    if latest <= current {
        let _ = writeln!(out, "up to date ({})", env.current);
        return Ok(0);
    }
    let latest_text = tag.strip_prefix('v').unwrap_or(&tag);
    if check {
        let _ = writeln!(out, "update available: {} -> {latest_text}", env.current);
        return Ok(EXIT_UPDATE_AVAILABLE);
    }
    let _ = writeln!(out, "{} -> {latest_text}", env.current);
    if !yes {
        let _ = write!(err, "Update to {tag}? [y/N] ");
        let _ = err.flush();
        let mut line = String::new();
        let read = input
            .read_line(&mut line)
            .map_err(|e| format!("cannot read the answer: {e}"))?;
        if read == 0 {
            return Err(
                "no answer on stdin; run 'dimvexa update -y' to skip the question".to_string(),
            );
        }
        if !matches!(line.trim().to_ascii_lowercase().as_str(), "y" | "yes") {
            let _ = writeln!(out, "aborted");
            return Ok(0);
        }
    }

    let (asset, binary_in_archive) = if env.windows {
        ("dimvexa.exe".to_string(), None)
    } else {
        let stem = format!("dimvexa-{latest_text}-{}", env.target);
        (format!("{stem}.tar.gz"), Some(format!("{stem}/dimvexa")))
    };
    let sums_name = format!("{asset}.sha256");

    let scratch = TempDir::new()?;
    let downloaded = scratch.0.join("asset");
    let sums_file = scratch.0.join("asset.sha256");
    net.download(&tag, &asset, &downloaded)?;
    net.download(&tag, &sums_name, &sums_file)?;
    let bytes = std::fs::read(&downloaded).map_err(|e| format!("cannot read the download: {e}"))?;
    let sums = std::fs::read_to_string(&sums_file)
        .map_err(|e| format!("cannot read the checksum file: {e}"))?;
    verify_checksum(&bytes, &sums, &asset)?;

    let new_binary = if let Some(inner) = binary_in_archive {
        let unpacked = scratch.0.join("unpacked");
        std::fs::create_dir(&unpacked).map_err(|e| format!("cannot create a folder: {e}"))?;
        untar(&downloaded, &unpacked)?;
        let path = unpacked.join(inner);
        if !path.is_file() {
            return Err(format!("{asset} does not contain the dimvexa binary"));
        }
        path
    } else {
        downloaded
    };
    replace_binary(&new_binary, env.exe)?;
    let _ = writeln!(out, "updated to {latest_text}");
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    // Test vectors of FIPS 180-4 (NIST, "SHA-256 example"): the empty message, "abc" and the
    // 448-bit message that needs a second block for the padding.
    #[test]
    fn sha256_known_vectors() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    #[test]
    fn sha256_million_a() {
        let data = vec![b'a'; 1_000_000];
        assert_eq!(
            sha256_hex(&data),
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
        );
    }

    #[test]
    fn versions() {
        assert_eq!(parse_version("v0.3.0"), Some((0, 3, 0)));
        assert_eq!(parse_version("0.2.0"), Some((0, 2, 0)));
        assert_eq!(parse_version("1.2.3-rc.1+x"), Some((1, 2, 3)));
        assert_eq!(parse_version("1.2"), None);
        assert_eq!(parse_version("1.2.3.4"), None);
        assert_eq!(parse_version("latest"), None);
        assert!(parse_version("0.10.0") > parse_version("0.9.9"));
    }

    #[test]
    fn release_tag_from_redirect_url() {
        let url = "https://github.com/welworx/dimvexa/releases/tag/v0.3.0";
        assert_eq!(parse_release_tag(url).as_deref(), Some("v0.3.0"));
        assert_eq!(
            parse_release_tag(&format!("{url}?x=1")).as_deref(),
            Some("v0.3.0")
        );
        // No release yet: GitHub stays on /releases.
        assert_eq!(
            parse_release_tag("https://github.com/welworx/dimvexa/releases"),
            None
        );
        // A tag with characters that do not belong in a URL path is refused.
        assert_eq!(
            parse_release_tag("https://github.com/x/y/releases/tag/v1.2.3%2F..%2Fz"),
            None
        );
    }

    #[test]
    fn package_managers_are_left_alone() {
        let some = |p: &str, home: Option<&str>| {
            managed_elsewhere(Path::new(p), home.map(Path::new)).is_some()
        };
        assert!(some("/opt/homebrew/Cellar/dimvexa/0.2.0/bin/dimvexa", None));
        assert!(some("/opt/homebrew/bin/dimvexa", None));
        assert!(some("/home/linuxbrew/.linuxbrew/bin/dimvexa", None));
        assert!(some("/home/u/.cargo/bin/dimvexa", None));
        // A CARGO_HOME elsewhere (the official rust Docker image uses /usr/local/cargo).
        assert!(some(
            "/usr/local/cargo/bin/dimvexa",
            Some("/usr/local/cargo")
        ));
        assert!(!some("/usr/local/cargo/bin/dimvexa", None));
        // A folder that merely has "homebrew" in its name is not Homebrew.
        assert!(!some("/home/u/src/homebrew/bin/dimvexa", None));
        assert!(!some("/home/u/.local/bin/dimvexa", None));
    }

    #[test]
    fn no_answer_on_stdin_is_an_error_not_an_abort() {
        let t = scratch("bin");
        let exe = t.0.join("bin").join("dimvexa");
        std::fs::write(&exe, b"old").unwrap();
        let net = windows_release("v0.3.0", b"new", true);
        let r = drive(&net, &env_for("0.2.0", &exe), false, false, "");
        assert_eq!(r.code, 1);
        assert!(r.err.contains("no answer on stdin"), "{}", r.err);
        assert_eq!(std::fs::read(&exe).unwrap(), b"old");
    }

    #[test]
    fn a_checksum_file_for_another_asset_is_refused() {
        let t = scratch("bin");
        let exe = t.0.join("bin").join("dimvexa");
        std::fs::write(&exe, b"old").unwrap();
        let mut net = windows_release("v0.3.0", b"new", true);
        net.files[1].1 = format!("{}  other.exe\n", sha256_hex(b"new")).into_bytes();
        let r = drive(&net, &env_for("0.2.0", &exe), false, true, "");
        assert_eq!(r.code, 1);
        assert!(r.err.contains("does not describe"), "{}", r.err);
        assert_eq!(std::fs::read(&exe).unwrap(), b"old");
    }

    #[test]
    fn a_failed_rename_puts_the_old_binary_back() {
        let t = scratch("bin");
        let exe = t.0.join("bin").join("dimvexa");
        std::fs::write(&exe, b"old").unwrap();
        // The new file is missing, so staging fails before anything is renamed.
        assert!(replace_binary(&t.0.join("missing"), &exe).is_err());
        assert_eq!(std::fs::read(&exe).unwrap(), b"old");
        let leftovers: Vec<_> = std::fs::read_dir(t.0.join("bin")).unwrap().collect();
        assert_eq!(leftovers.len(), 1, "{leftovers:?}");
    }

    #[test]
    fn checksum_checks_hash_and_name() {
        let good = format!("{}  dimvexa.exe\n", sha256_hex(b"abc"));
        assert!(verify_checksum(b"abc", &good, "dimvexa.exe").is_ok());
        assert!(verify_checksum(b"abd", &good, "dimvexa.exe").is_err());
        assert!(verify_checksum(b"abc", &good, "other.exe").is_err());
        let star = format!("{} *dimvexa.exe", sha256_hex(b"abc").to_uppercase());
        assert!(verify_checksum(b"abc", &star, "dimvexa.exe").is_ok());
        assert!(verify_checksum(b"abc", "", "dimvexa.exe").is_err());
    }

    /// A release served from memory: asset name -> content.
    struct Fake {
        tag: String,
        files: Vec<(String, Vec<u8>)>,
    }

    impl Net for Fake {
        fn latest_tag(&self) -> Result<String, String> {
            Ok(self.tag.clone())
        }
        fn download(&self, tag: &str, name: &str, dest: &Path) -> Result<(), String> {
            assert_eq!(tag, self.tag);
            let (_, bytes) = self
                .files
                .iter()
                .find(|(n, _)| n == name)
                .ok_or_else(|| format!("404 {name}"))?;
            std::fs::write(dest, bytes).map_err(|e| e.to_string())
        }
    }

    struct Run {
        code: u8,
        out: String,
        err: String,
    }

    fn drive(net: &dyn Net, env: &Env<'_>, check: bool, yes: bool, answer: &str) -> Run {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = update_with(
            net,
            env,
            check,
            yes,
            &mut Cursor::new(answer.to_string()),
            &mut out,
            &mut err,
        );
        Run {
            code,
            out: String::from_utf8(out).unwrap(),
            err: String::from_utf8(err).unwrap(),
        }
    }

    /// A fresh folder for one test.
    fn scratch(name: &str) -> TempDir {
        let t = TempDir::new().unwrap();
        let dir = t.0.join(name);
        std::fs::create_dir(&dir).unwrap();
        t
    }

    /// A Windows-style release (one bare file) for `tag`, with the right or a wrong checksum.
    fn windows_release(tag: &str, content: &[u8], good_sum: bool) -> Fake {
        let hash = if good_sum {
            sha256_hex(content)
        } else {
            "0".repeat(64)
        };
        Fake {
            tag: tag.to_string(),
            files: vec![
                ("dimvexa.exe".to_string(), content.to_vec()),
                (
                    "dimvexa.exe.sha256".to_string(),
                    format!("{hash}  dimvexa.exe\n").into_bytes(),
                ),
            ],
        }
    }

    fn env_for<'a>(current: &'a str, exe: &'a Path) -> Env<'a> {
        Env {
            current,
            target: "x86_64-pc-windows-msvc",
            windows: true,
            exe,
        }
    }

    #[test]
    fn up_to_date_and_check() {
        let t = scratch("bin");
        let exe = t.0.join("bin").join("dimvexa");
        std::fs::write(&exe, b"old").unwrap();
        let net = windows_release("v0.2.0", b"new", true);
        let r = drive(&net, &env_for("0.2.0", &exe), false, true, "");
        assert_eq!((r.code, r.out.as_str()), (0, "up to date (0.2.0)\n"));
        // A newer release is reported by --check without touching the binary.
        let net = windows_release("v0.3.0", b"new", true);
        let r = drive(&net, &env_for("0.2.0", &exe), true, false, "");
        assert_eq!(
            (r.code, r.out.as_str()),
            (EXIT_UPDATE_AVAILABLE, "update available: 0.2.0 -> 0.3.0\n")
        );
        assert_eq!(std::fs::read(&exe).unwrap(), b"old");
        // A release older than the running version is not a downgrade.
        let net = windows_release("v0.1.0", b"new", true);
        assert_eq!(
            drive(&net, &env_for("0.2.0", &exe), false, true, "").code,
            0
        );
        assert_eq!(std::fs::read(&exe).unwrap(), b"old");
    }

    #[test]
    fn installs_after_confirmation_only() {
        let t = scratch("bin");
        let exe = t.0.join("bin").join("dimvexa");
        std::fs::write(&exe, b"old").unwrap();
        let net = windows_release("v0.3.0", b"new binary", true);
        let r = drive(&net, &env_for("0.2.0", &exe), false, false, "n\n");
        assert_eq!((r.code, r.out.as_str()), (0, "0.2.0 -> 0.3.0\naborted\n"));
        assert!(r.err.contains("Update to v0.3.0? [y/N]"));
        assert_eq!(std::fs::read(&exe).unwrap(), b"old");
        let r = drive(&net, &env_for("0.2.0", &exe), false, false, "y\n");
        assert_eq!(r.code, 0, "{}", r.err);
        assert!(r.out.ends_with("updated to 0.3.0\n"));
        assert_eq!(std::fs::read(&exe).unwrap(), b"new binary");
        assert!(!exe.with_extension("old").exists());
        assert!(!t.0.join("bin").join("dimvexa.old").exists());
    }

    #[test]
    fn bad_checksum_installs_nothing() {
        let t = scratch("bin");
        let exe = t.0.join("bin").join("dimvexa");
        std::fs::write(&exe, b"old").unwrap();
        let net = windows_release("v0.3.0", b"new binary", false);
        let r = drive(&net, &env_for("0.2.0", &exe), false, true, "");
        assert_eq!(r.code, 1);
        assert!(r.err.contains("checksum mismatch"), "{}", r.err);
        assert_eq!(std::fs::read(&exe).unwrap(), b"old");
    }

    #[test]
    fn missing_asset_is_an_error() {
        let t = scratch("bin");
        let exe = t.0.join("bin").join("dimvexa");
        std::fs::write(&exe, b"old").unwrap();
        let net = Fake {
            tag: "v0.3.0".to_string(),
            files: Vec::new(),
        };
        let r = drive(&net, &env_for("0.2.0", &exe), false, true, "");
        assert_eq!(r.code, 1);
        assert!(r.err.contains("404"), "{}", r.err);
    }

    /// An archive without the binary installs nothing.
    #[cfg(unix)]
    #[test]
    fn an_archive_without_the_binary_is_refused() {
        let t = scratch("bin");
        let exe = t.0.join("bin").join("dimvexa");
        std::fs::write(&exe, b"old").unwrap();
        let stem = "dimvexa-0.3.0-x86_64-unknown-linux-musl";
        let pack = t.0.join("pack");
        std::fs::create_dir_all(pack.join(stem)).unwrap();
        std::fs::write(pack.join(stem).join("README.md"), b"no binary").unwrap();
        let made = Command::new("tar")
            .arg("-czf")
            .arg(pack.join("a.tar.gz"))
            .arg("-C")
            .arg(&pack)
            .arg(stem)
            .status()
            .unwrap();
        assert!(made.success());
        let tarball = std::fs::read(pack.join("a.tar.gz")).unwrap();
        let name = format!("{stem}.tar.gz");
        let sums = format!("{}  {name}\n", sha256_hex(&tarball)).into_bytes();
        let net = Fake {
            tag: "v0.3.0".to_string(),
            files: vec![(name.clone(), tarball), (format!("{name}.sha256"), sums)],
        };
        let env = Env {
            current: "0.2.0",
            target: "x86_64-unknown-linux-musl",
            windows: false,
            exe: &exe,
        };
        let r = drive(&net, &env, false, true, "");
        assert_eq!(r.code, 1);
        assert!(r.err.contains("does not contain"), "{}", r.err);
        assert_eq!(std::fs::read(&exe).unwrap(), b"old");
    }

    /// The Unix path: a real `.tar.gz` in the layout the release workflow builds.
    #[cfg(unix)]
    #[test]
    fn unpacks_and_installs_a_tarball() {
        use std::os::unix::fs::PermissionsExt as _;
        let t = scratch("bin");
        let exe = t.0.join("bin").join("dimvexa");
        std::fs::write(&exe, b"old").unwrap();
        let stem = "dimvexa-0.3.0-x86_64-unknown-linux-musl";
        let pack = t.0.join("pack");
        std::fs::create_dir_all(pack.join(stem)).unwrap();
        std::fs::write(pack.join(stem).join("dimvexa"), b"new binary").unwrap();
        let made = Command::new("tar")
            .arg("-czf")
            .arg(pack.join("a.tar.gz"))
            .arg("-C")
            .arg(&pack)
            .arg(stem)
            .status()
            .unwrap();
        assert!(made.success());
        let tarball = std::fs::read(pack.join("a.tar.gz")).unwrap();
        let name = format!("{stem}.tar.gz");
        let net = Fake {
            tag: "v0.3.0".to_string(),
            files: vec![
                (name.clone(), tarball.clone()),
                (
                    format!("{name}.sha256"),
                    format!("{}  {name}\n", sha256_hex(&tarball)).into_bytes(),
                ),
            ],
        };
        let env = Env {
            current: "0.2.0",
            target: "x86_64-unknown-linux-musl",
            windows: false,
            exe: &exe,
        };
        let r = drive(&net, &env, false, true, "");
        assert_eq!(r.code, 0, "{}", r.err);
        assert_eq!(std::fs::read(&exe).unwrap(), b"new binary");
        assert_eq!(
            std::fs::metadata(&exe).unwrap().permissions().mode() & 0o777,
            0o755
        );
    }
}
