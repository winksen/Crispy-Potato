//! Restrict destinations both before requests and in the socket's DNS resolver.
use reqwest::{
    Url,
    dns::{Addrs, Name, Resolve, Resolving},
};
use std::{
    fmt,
    net::{IpAddr, SocketAddr},
};

#[derive(Clone)]
pub(crate) enum Policy {
    Source,
    Links,
    #[cfg(test)]
    LocalTest(Url),
}

#[derive(Debug)]
pub(crate) struct Denied;
impl fmt::Display for Denied {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("egress destination refused")
    }
}
impl std::error::Error for Denied {}

impl Policy {
    pub(crate) fn validate(&self, url: &Url) -> Result<(), crate::http::Error> {
        use crate::http::Error;
        if !matches!(url.scheme(), "http" | "https")
            || !url.username().is_empty()
            || url.password().is_some()
        {
            return Err(Error::InvalidUrl);
        }
        #[cfg(test)]
        if let Self::LocalTest(origin) = self
            && url.origin() == origin.origin()
        {
            return Ok(());
        }
        if matches!(self, Self::Source)
            && (url.scheme() != "https"
                || url.port_or_known_default() != Some(443)
                || !crate::limits::QUOTAS
                    .iter()
                    .any(|q| Some(q.1) == url.host_str()))
        {
            return Err(Error::Blocked);
        }
        match url.host() {
            Some(url::Host::Ipv4(ip)) if !public_ip(IpAddr::V4(ip)) => Err(Error::Blocked),
            Some(url::Host::Ipv6(ip)) if !public_ip(IpAddr::V6(ip)) => Err(Error::Blocked),
            Some(url::Host::Domain(name))
                if name.eq_ignore_ascii_case("localhost")
                    || name.trim_end_matches('.').ends_with(".localhost")
                    || name == "localhost." =>
            {
                Err(Error::Blocked)
            }
            None => Err(Error::InvalidUrl),
            _ => Ok(()),
        }
    }
}

// Conservative public unicast policy. Reserved, documentation and transition
// networks are refused along with private, link-local and multicast addresses.
pub(crate) fn public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let [a, b, c, _] = ip.octets();
            !(matches!(a, 0 | 10 | 127 | 224..=255)
                || (a == 100 && (64..=127).contains(&b))
                || (a == 169 && b == 254)
                || (a == 172 && (16..=31).contains(&b))
                || (a == 192
                    && (b == 168 || (b == 0 && matches!(c, 0 | 2)) || (b == 88 && c == 99)))
                || (a == 198 && (matches!(b, 18 | 19) || (b == 51 && c == 100)))
                || (a == 203 && b == 0 && c == 113))
        }
        IpAddr::V6(ip) => {
            let s = ip.segments();
            (s[0] & 0xe000) == 0x2000
                && !(s[0] == 0x2001 && (s[1] < 0x0200 || s[1] == 0x0db8))
                && s[0] != 0x2002
                && !(s[0] == 0x3fff && s[1] < 0x1000)
        }
    }
}

fn checked(addrs: Vec<SocketAddr>) -> Result<Addrs, Box<dyn std::error::Error + Send + Sync>> {
    if addrs.is_empty() || addrs.iter().any(|address| !public_ip(address.ip())) {
        return Err(Box::new(Denied));
    }
    Ok(Box::new(addrs.into_iter()))
}

pub(crate) struct PublicResolver;
impl Resolve for PublicResolver {
    fn resolve(&self, name: Name) -> Resolving {
        Box::pin(async move {
            let addresses = tokio::net::lookup_host((name.as_str(), 0)).await?.collect();
            checked(addresses)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_nonpublic_literal_and_dns_addresses() {
        for address in [
            "0.0.0.0",
            "10.0.0.1",
            "127.0.0.1",
            "100.64.0.1",
            "169.254.169.254",
            "172.31.0.1",
            "192.168.0.1",
            "192.0.2.1",
            "198.18.0.1",
            "224.0.0.1",
            "255.255.255.255",
            "::",
            "::1",
            "::ffff:127.0.0.1",
            "fc00::1",
            "fe80::1",
            "ff02::1",
            "2001:db8::1",
            "2002:7f00:1::",
        ] {
            let ip: IpAddr = address.parse().unwrap();
            assert!(!public_ip(ip), "{address}");
            assert!(checked(vec![SocketAddr::new(ip, 443)]).is_err());
        }
        for address in ["1.1.1.1", "8.8.8.8", "2606:4700:4700::1111"] {
            assert!(public_ip(address.parse().unwrap()));
        }
        assert!(
            checked(vec![
                "1.1.1.1:443".parse().unwrap(),
                "127.0.0.1:443".parse().unwrap()
            ])
            .is_err()
        );
    }

    #[tokio::test]
    async fn actual_resolver_refuses_localhost() {
        assert!(
            PublicResolver
                .resolve("localhost".parse().unwrap())
                .await
                .is_err()
        );
    }
}
