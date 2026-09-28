use std::sync::Arc;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};

use crate::config::Config;
use futures::TryStreamExt;
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::{Attachment, MultiPart, SinglePart, header::ContentType},
    transport::smtp::authentication::Credentials,
};

const SMTP: &str = "smtp.gmail.com";
const IMAP: (&str, u16) = ("imap.gmail.com", 993);

fn keychain(config: &Config) -> String {
    format!("hunt-gmail{}", config.instance())
}

pub struct Mailbox {
    address: String,
    password: String,
}

pub struct Incoming {
    pub uid: u32,
    pub message_id: String,
    pub sender: String,
    pub subject: String,
    pub text: String,
    pub at: DateTime<Utc>,
}

impl Mailbox {
    pub fn open(config: &Config, address: &str) -> Result<Mailbox> {
        let password = keyring::Entry::new(&keychain(config), address)?
            .get_password()
            .context("no Gmail app password in the Keychain yet; add it in Settings")?;
        Ok(Mailbox { address: address.into(), password })
    }

    pub fn remember(config: &Config, address: &str, password: &str) -> Result<()> {
        Ok(keyring::Entry::new(&keychain(config), address)?.set_password(password)?)
    }

    pub async fn send(
        &self,
        to: &str,
        subject: &str,
        body: &str,
        message_id: &str,
        attachments: Vec<(String, Vec<u8>)>,
    ) -> Result<()> {
        let mut parts = MultiPart::mixed().singlepart(SinglePart::plain(body.to_string()));
        for (name, bytes) in attachments {
            parts = parts.singlepart(Attachment::new(name).body(bytes, ContentType::parse("application/pdf")?));
        }
        let message = Message::builder()
            .from(self.address.parse()?)
            .to(to.parse().with_context(|| format!("{to} is not an email address"))?)
            .subject(subject)
            .message_id(Some(message_id.into()))
            .multipart(parts)?;
        AsyncSmtpTransport::<Tokio1Executor>::relay(SMTP)?
            .credentials(Credentials::new(self.address.clone(), self.password.clone()))
            .build()
            .send(message)
            .await
            .context("Gmail refused the message")?;
        Ok(())
    }

    /// Inbox messages with a UID above `after`, oldest first.
    pub async fn since(&self, after: u32) -> Result<Vec<Incoming>> {
        let mut roots = tokio_rustls::rustls::RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let tls = tokio_rustls::TlsConnector::from(Arc::new(
            tokio_rustls::rustls::ClientConfig::builder().with_root_certificates(roots).with_no_client_auth(),
        ));
        let tcp = tokio::net::TcpStream::connect(IMAP).await?;
        let stream = tls.connect(IMAP.0.try_into()?, tcp).await?;
        let mut session = async_imap::Client::new(stream)
            .login(&self.address, &self.password)
            .await
            .map_err(|(err, _)| err)
            .context("Gmail refused the login; check the app password")?;
        session.select("INBOX").await?;

        let mut messages = vec![];
        let fetched: Vec<_> =
            session.uid_fetch(format!("{}:*", after + 1), "(UID BODY.PEEK[])").await?.try_collect().await?;
        for fetch in fetched {
            let (Some(uid), Some(raw)) = (fetch.uid, fetch.body()) else { continue };
            if uid <= after {
                continue;
            }
            let Some(mail) = mail_parser::MessageParser::default().parse(raw) else { continue };
            messages.push(Incoming {
                uid,
                message_id: mail.message_id().unwrap_or_default().to_string(),
                sender: mail.from().and_then(|f| f.first()).and_then(|a| a.address()).unwrap_or_default().to_string(),
                subject: mail.subject().unwrap_or_default().to_string(),
                text: mail.body_text(0).unwrap_or_default().chars().take(4000).collect(),
                at: mail.date().and_then(|d| DateTime::from_timestamp(d.to_timestamp(), 0)).unwrap_or_else(Utc::now),
            });
        }
        session.logout().await.ok();
        Ok(messages)
    }
}
