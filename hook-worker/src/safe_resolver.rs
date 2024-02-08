use std::error::Error as StdError;
use std::fmt;
use std::net::SocketAddr;

use futures_util::future::FutureExt;
use hyper::client::connect::dns::{GaiResolver as HyperGaiResolver, Name};
use hyper::service::Service;
use reqwest::dns::{Addrs, Resolve, Resolving};

type BoxError = Box<dyn StdError + Send + Sync>;

/// SafeResolver is a copy of the Gai (GetAddrInfo) resolver from reqwest, because it is private. We
/// then add a validation step to ensure that the resolved addresses are not private, much like
/// `plugin-server`'s `raiseIfUserProvidedUrlUnsafe` function:
///     https://github.com/PostHog/posthog/blob/5c1867cfcf3138a1979e9356396cb999eda52855/plugin-server/src/utils/fetch.ts#L31-L63

#[derive(Debug)]
pub struct SafeResolver(HyperGaiResolver);

#[derive(Debug)]
struct InvalidUrlError;

impl fmt::Display for InvalidUrlError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "A custom error occurred")
    }
}

impl std::error::Error for InvalidUrlError {}

impl SafeResolver {
    pub fn new() -> Self {
        Self(HyperGaiResolver::new())
    }
}

impl Default for SafeResolver {
    fn default() -> Self {
        SafeResolver::new()
    }
}

fn validate_addr(addr: &SocketAddr) -> bool {
    match addr {
        SocketAddr::V4(ipv4) => {
            let ip = ipv4.ip();
            if ip.is_private()
                || ip.is_loopback()
                || ip.is_link_local()
                || ip.is_broadcast()
                || ip.is_multicast()
                || ip.is_unspecified()
                || ip.is_documentation()
            {
                return false;
            }

            true
        }
        SocketAddr::V6(ipv6) => {
            let ip = ipv6.ip();
            if ip.is_loopback() || ip.is_multicast() || ip.is_unspecified() {
                return false;
            }

            // TODO: is_unique_local, among others, are not available in stable Rust
            // https://github.com/rust-lang/rust/blob/07dca489ac2d933c78d3c5158e3f43beefeb02ce/library/core/src/net/ip_addr.rs#L1525-L1547

            true
        }
    }
}

impl Resolve for SafeResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let this = &mut self.0.clone();
        Box::pin(Service::<Name>::call(this, name).map(|result| {
            result
                .and_then(|addrs| {
                    let addrs: Vec<_> = addrs.collect();

                    if !addrs.iter().all(validate_addr) {
                        // If any address fails validation, return an Err
                        Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "Validation failed",
                        ))
                    } else {
                        Ok(Box::new(addrs.into_iter()) as Addrs)
                    }
                })
                .map_err(|err| -> BoxError { Box::new(err) })
        }))
    }
}
