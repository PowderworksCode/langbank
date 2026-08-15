//! Reading an upstream, over the network.
//!
//! Nothing is vendored. A tarball is read straight out of memory rather than
//! unpacked, because these run in CI where the checkout is throwaway and on a
//! developer's machine where it is not.

use std::io::Read;

use crate::report::Result;

const TIMEOUT: u64 = 120;

pub fn text(url: &str) -> Result<String> {
    let mut body = String::new();
    ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(TIMEOUT))
        .build()
        .get(url)
        .call()?
        .into_reader()
        .read_to_string(&mut body)?;
    Ok(body)
}

// Used by the tarball readers as they land; the sources that need them are
// ported one at a time.
#[allow(dead_code)]
pub fn bytes(url: &str) -> Result<Vec<u8>> {
    let mut body = Vec::new();
    ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(TIMEOUT))
        .build()
        .get(url)
        .call()?
        .into_reader()
        .read_to_end(&mut body)?;
    Ok(body)
}

/// Every file in a gzipped tarball whose path matches, as (path, contents).
#[allow(dead_code)]
pub fn tarball(url: &str, matches: impl Fn(&str) -> bool) -> Result<Vec<(String, String)>> {
    let raw = bytes(url)?;
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(raw.as_slice()));
    let mut out = Vec::new();
    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.to_string_lossy().into_owned();
        if !matches(&path) {
            continue;
        }
        let mut contents = String::new();
        if entry.read_to_string(&mut contents).is_ok() {
            out.push((path, contents));
        }
    }
    out.sort();
    Ok(out)
}
