use std::collections::HashSet;
use std::net::{SocketAddr, ToSocketAddrs};

#[cfg(feature = "testnet")]
const REGIONS_URL: &str = "https://raw.githubusercontent.com/jito-foundation/bam-plugins/refs/heads/regions/data/testnet-regions.txt";
#[cfg(feature = "testnet")]
const DNS_SUFFIX: &str = "testnet.bam.jito.wtf";
const PORT: u16 = 5012;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    #[cfg(not(feature = "tokio"))]
    Ureq(ureq::Error),
    #[cfg(feature = "tokio")]
    Reqwest(reqwest::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "io: {e}"),
            #[cfg(not(feature = "tokio"))]
            Error::Ureq(e) => write!(f, "http: {e}"),
            #[cfg(feature = "tokio")]
            Error::Reqwest(e) => write!(f, "http: {e}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            #[cfg(not(feature = "tokio"))]
            Error::Ureq(e) => Some(e),
            #[cfg(feature = "tokio")]
            Error::Reqwest(e) => Some(e),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

#[cfg(not(feature = "tokio"))]
impl From<ureq::Error> for Error {
    fn from(e: ureq::Error) -> Self {
        Error::Ureq(e)
    }
}

#[cfg(feature = "tokio")]
impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::Reqwest(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub struct JitoMakerClient {
    targets: HashSet<SocketAddr>,
    regions_url: String,
}

impl JitoMakerClient {
    pub fn new() -> Self {
        Self {
            targets: HashSet::new(),
            regions_url: REGIONS_URL.to_string(),
        }
    }

    pub fn with_regions_url(mut self, url: &str) -> Self {
        self.regions_url = url.to_string();
        self
    }

    pub fn set_targets(&mut self, addrs: &[SocketAddr]) {
        for &addr in addrs {
            self.targets.insert(addr);
        }
    }
}

#[cfg(not(feature = "tokio"))]
impl JitoMakerClient {
    pub fn send_wire_transaction(&self, transaction_bytes: &[u8]) -> crate::Result<()> {
        let socket = std::net::UdpSocket::bind("0.0.0.0:0")?;
        for addr in &self.targets {
            socket.send_to(transaction_bytes, addr).ok();
        }
        Ok(())
    }

    pub fn sync_targets(&mut self) -> crate::Result<()> {
        let body: String = ureq::get(&self.regions_url).call()?.into_string()?;

        for region in body.lines() {
            let region = region.trim();
            if region.is_empty() {
                continue;
            }
            let hostname = format!("{}.{}:{}", region, DNS_SUFFIX, PORT);
            if let Ok(addrs) = hostname.to_socket_addrs() {
                for addr in addrs {
                    self.targets.insert(addr);
                }
            }
        }
        Ok(())
    }
}

#[cfg(feature = "tokio")]
impl JitoMakerClient {
    pub async fn send_wire_transaction(&self, transaction_bytes: &[u8]) -> crate::Result<()> {
        let socket = tokio::net::UdpSocket::bind("0.0.0.0:0").await?;
        for addr in &self.targets {
            socket.send_to(transaction_bytes, addr).await.ok();
        }
        Ok(())
    }

    pub async fn sync_targets(&mut self) -> crate::Result<()> {
        let body = reqwest::get(&self.regions_url).await?.text().await?;

        for region in body.lines() {
            let region = region.trim();
            if region.is_empty() {
                continue;
            }
            let hostname = format!("{}.{}:{}", region, DNS_SUFFIX, PORT);
            if let Ok(addrs) = hostname.to_socket_addrs() {
                for addr in addrs {
                    self.targets.insert(addr);
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(feature = "tokio"))]
    #[test]
    fn sync_targets_fetches_and_parses_regions() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/regions.txt")
            .with_status(200)
            .with_body("ams\nslc\n")
            .create();

        let mut client =
            JitoMakerClient::new().with_regions_url(&format!("{}/regions.txt", server.url()));

        let result = client.sync_targets();
        assert!(result.is_ok());
        mock.assert();
    }

    #[cfg(not(feature = "tokio"))]
    #[test]
    fn sync_targets_handles_empty_lines() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/regions.txt")
            .with_status(200)
            .with_body("\n  \nams\n\n")
            .create();

        let mut client =
            JitoMakerClient::new().with_regions_url(&format!("{}/regions.txt", server.url()));

        let result = client.sync_targets();
        assert!(result.is_ok());
        mock.assert();
    }

    #[cfg(not(feature = "tokio"))]
    #[test]
    fn sync_targets_returns_error_on_server_failure() {
        let mut server = mockito::Server::new();
        let mock = server.mock("GET", "/regions.txt").with_status(500).create();

        let mut client =
            JitoMakerClient::new().with_regions_url(&format!("{}/regions.txt", server.url()));

        let result = client.sync_targets();
        assert!(result.is_err());
        mock.assert();
    }

    #[cfg(feature = "tokio")]
    #[tokio::test]
    async fn sync_targets_async_fetches_and_parses_regions() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/regions.txt")
            .with_status(200)
            .with_body("ams\nslc\n")
            .create_async()
            .await;

        let mut client =
            JitoMakerClient::new().with_regions_url(&format!("{}/regions.txt", server.url()));

        let result = client.sync_targets().await;
        assert!(result.is_ok());
        mock.assert_async().await;
    }

    #[cfg(feature = "tokio")]
    #[tokio::test]
    async fn sync_targets_async_handles_empty_lines() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/regions.txt")
            .with_status(200)
            .with_body("\n  \nams\n\n")
            .create_async()
            .await;

        let mut client =
            JitoMakerClient::new().with_regions_url(&format!("{}/regions.txt", server.url()));

        let result = client.sync_targets().await;
        assert!(result.is_ok());
        mock.assert_async().await;
    }
}
