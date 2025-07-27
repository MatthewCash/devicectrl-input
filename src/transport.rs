use std::{net::SocketAddr, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use devicectrl_common::{UpdateRequest, protocol::tcp::ServerBoundTcpMessage};
use futures::{SinkExt, TryStreamExt};
use serde::{Deserialize, de};
use serde_derive::Deserialize;
use socket2::{SockRef, TcpKeepalive};
use tokio::{net::TcpStream, select, sync::mpsc, time::sleep};
use tokio_rustls::{
    TlsConnector,
    rustls::{
        ClientConfig, RootCertStore,
        pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject},
    },
};
use tokio_util::codec::{Framed, LinesCodec};

#[derive(Clone, Debug, Deserialize)]
pub struct ServerConnectionConfig {
    pub server_addr: SocketAddr,
    pub server_domain: String,
    #[serde(
        rename = "server_ca_path",
        deserialize_with = "deserialize_file_path_bytes"
    )]
    pub server_ca_bytes: Vec<u8>,
    #[serde(rename = "cert_path", deserialize_with = "deserialize_file_path_bytes")]
    pub cert_bytes: Vec<u8>,
    #[serde(rename = "key_path", deserialize_with = "deserialize_file_path_bytes")]
    pub key_bytes: Vec<u8>,
}

fn deserialize_file_path_bytes<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: de::Deserializer<'de>,
{
    std::fs::read(String::deserialize(deserializer)?).map_err(de::Error::custom)
}

pub async fn connect_to_server(
    config: &ServerConnectionConfig,
    receiver: &mut mpsc::Receiver<UpdateRequest>,
    connector: &TlsConnector,
) -> Result<()> {
    let connection = TcpStream::connect(config.server_addr).await?;
    let sock_ref = SockRef::from(&connection);

    let keepalive = TcpKeepalive::new()
        .with_time(Duration::from_secs(5))
        .with_interval(Duration::from_secs(5));

    sock_ref.set_tcp_keepalive(&keepalive)?;

    let mut stream = Framed::new(
        connector
            .connect(config.server_domain.clone().try_into()?, connection)
            .await?,
        LinesCodec::new(),
    );

    log::info!("Connected to server at {}", config.server_addr);

    loop {
        select! {
            update = receiver.recv() => {
                log::debug!("sending update: {update:?}");
                stream
                    .send(serde_json::to_string(
                        &ServerBoundTcpMessage::UpdateRequest(update.context("update channel closed")?),
                    )?)
                    .await?;
            }
            data = stream.try_next() => {
                data.context("failed to recv message from server")?;
            }
        }
    }
}

pub async fn start_communication(
    config: ServerConnectionConfig,
    mut receiver: mpsc::Receiver<UpdateRequest>,
) -> Result<()> {
    let mut root_store = RootCertStore::empty();
    root_store.add(CertificateDer::from_pem_slice(&config.server_ca_bytes)?)?;

    let tls_config = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_client_auth_cert(
            vec![CertificateDer::from_pem_slice(&config.cert_bytes)?],
            PrivateKeyDer::from_pem_slice(&config.key_bytes)?,
        )?;

    let connector = TlsConnector::from(Arc::new(tls_config));

    loop {
        if let Err(err) = connect_to_server(&config, &mut receiver, &connector)
            .await
            .context("connection to server failed")
        {
            log::error!("{err:?}");
            sleep(Duration::from_secs(5)).await;
        }
    }
}
