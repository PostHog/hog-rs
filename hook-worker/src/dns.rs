use std::error::Error as StdError;
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::{fmt, io};

use futures::FutureExt;
use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use tokio::task::spawn_blocking;

pub struct NoPublicIPError;

impl std::error::Error for NoPublicIPError {}
impl fmt::Display for NoPublicIPError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "No public IPv4 found for specified host")
    }
}
impl fmt::Debug for NoPublicIPError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "No public IPv4 found for specified host")
    }
}

/// Internal reqwest type, copied here as part of Resolving
pub(crate) type BoxError = Box<dyn StdError + Send + Sync>;

/// Returns [`true`] if the address appears to be a globally reachable IPv4.
///
/// Trimmed down version of the unstable IpAddr::is_global, move to it when it's stable.
fn is_global_ipv4(addr: &SocketAddr) -> bool {
    match addr.ip() {
        IpAddr::V4(ip) => {
            !(ip.octets()[0] == 0 // "This network"
            || ip.is_private()
            || ip.is_loopback()
            || ip.is_link_local()
            || ip.is_broadcast())
        }
        IpAddr::V6(_) => false, // Our network does not currently support ipv6, let's ignore for now
    }
}

/// DNS resolver using the stdlib resolver, but filtering results to only pass public IPv4 results.
///
/// Private and broadcast addresses are filtered out, so are IPv6 results for now (as our infra
/// does not currently support IPv6 routing anyway).
/// This is adapted from the GaiResolver in hyper and reqwest.
pub struct PublicIPv4Resolver {}

impl Resolve for PublicIPv4Resolver {
    fn resolve(&self, name: Name) -> Resolving {
        // Closure to call the system's resolver (blocking call) through the ToSocketAddrs trait.
        let resolve_host = move || (name.as_str(), 0).to_socket_addrs();

        // Execute the blocking call in a separate worker thread then process its result asynchronously.
        // spawn_blocking returns a JoinHandle that implements Future<Result<(closure result), JoinError>>.
        let future_result = spawn_blocking(resolve_host).map(|result| match result {
            Ok(Ok(all_addrs)) => {
                // Resolution succeeded, filter the results
                let filtered_addr: Vec<SocketAddr> = all_addrs.filter(is_global_ipv4).collect();
                if filtered_addr.is_empty() {
                    // No public IPs found, error out with PermissionDenied
                    let err: BoxError = Box::new(NoPublicIPError);
                    Err(err)
                } else {
                    // Pass remaining IPs in a boxed iterator for request to use.
                    let addrs: Addrs = Box::new(filtered_addr.into_iter());
                    Ok(addrs)
                }
            }
            Ok(Err(err)) => {
                // Resolution failed, pass error through in a Box
                let err: BoxError = Box::new(err);
                Err(err)
            }
            Err(join_err) => {
                // The tokio task failed, error handled copied from hyper's GaiResolver
                if join_err.is_cancelled() {
                    let err: BoxError =
                        Box::new(io::Error::new(io::ErrorKind::Interrupted, join_err));
                    Err(err)
                } else {
                    panic!("background task failed: {:?}", join_err)
                }
            }
        });

        // Box the Future to satisfy the Resolving interface.
        Box::pin(future_result)
    }
}
