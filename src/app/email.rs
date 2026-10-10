#[cfg(feature = "smtp")]
mod implementation {
    use std::env::{self, VarError};

    use lettre::transport::smtp::AsyncSmtpTransportBuilder;
    use lettre::{
        AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
        message::{Mailbox, SinglePart, header::ContentType},
        transport::smtp::authentication::Credentials,
    };

    use crate::{
        app::models::SiteData,
        error::{AppResult, ResultExt},
    };

    const MAX_SUBJECT_NAME_LENGTH: usize = 120;
    const MAX_COMPANY_LENGTH: usize = 120;
    const MAX_EMAIL_LENGTH: usize = 254;
    const MAX_MESSAGE_LENGTH: usize = 2_000;
    const DEFAULT_SMTP_PORT: u16 = 587;

    #[derive(Clone)]
    pub(crate) struct Mailer {
        transport: AsyncSmtpTransport<Tokio1Executor>,
        from: Mailbox,
        default_to: Mailbox,
    }

    pub(crate) struct CvContactPayload<'a> {
        pub full_name: &'a str,
        pub company: &'a str,
        pub email: &'a str,
        pub message: &'a str,
    }

    pub(crate) fn build_mailer(site_data: &SiteData) -> AppResult<Option<Mailer>> {
        let host = match env_var("SMTP_HOST")? {
            Some(value) => value,
            None => return Ok(None),
        };
        let port: u16 = match env_var("SMTP_PORT")? {
            Some(value) => value.parse().with_context(|| "parsing SMTP_PORT")?,
            None => DEFAULT_SMTP_PORT,
        };
        let username = match env_var("SMTP_USERNAME")? {
            Some(value) => value,
            None => return Ok(None),
        };
        let password = match env_var("SMTP_PASSWORD")? {
            Some(value) => value,
            None => return Ok(None),
        };
        let from = match env_var("SMTP_FROM")? {
            Some(value) => value,
            None => return Ok(None),
        };

        let credentials = Credentials::new(username, password);

        // `relay` = implicit TLS (typically 465). `starttls_relay` = STARTTLS (typically 587).
        // Using `relay` on 587 makes rustls speak TLS to a plaintext SMTP banner.
        let builder: AsyncSmtpTransportBuilder = if port == 465 {
            AsyncSmtpTransport::<Tokio1Executor>::relay(&host)
                .with_context(|| format!("building smtp transport for {host}"))?
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&host)
                .with_context(|| format!("building starttls smtp transport for {host}"))?
        };

        let transport = builder.port(port).credentials(credentials).build();

        let from_mailbox =
            from.parse::<Mailbox>().with_context(|| format!("parsing SMTP_FROM address {from}"))?;
        let cv_email = site_data.contact.emails.resolved_cv();
        let to_mailbox = cv_email
            .parse::<Mailbox>()
            .with_context(|| format!("parsing cv contact email {cv_email}"))?;

        Ok(Some(Mailer { transport, from: from_mailbox, default_to: to_mailbox }))
    }

    impl Mailer {
        pub(crate) async fn send_cv_contact(
            &self,
            payload: CvContactPayload<'_>,
            to_override: Option<&str>,
        ) -> AppResult<()> {
            let subject_name = sanitize_header_value(payload.full_name, MAX_SUBJECT_NAME_LENGTH);
            let subject = format!("Nuovo contatto dal CV - {subject_name}");

            let body = format!(
                "Nuovo messaggio inviato dal form CV.\n\nNome completo: {}\nAzienda: {}\nEmail: {}\n\nMessaggio:\n{}\n",
                sanitize_body_value(payload.full_name, MAX_SUBJECT_NAME_LENGTH),
                sanitize_body_value(payload.company, MAX_COMPANY_LENGTH),
                sanitize_body_value(payload.email, MAX_EMAIL_LENGTH),
                sanitize_message_body(payload.message, MAX_MESSAGE_LENGTH)
            );

            let reply_to = sanitize_header_value(payload.email, MAX_EMAIL_LENGTH)
                .parse::<Mailbox>()
                .with_context(|| "parsing contact form reply-to address")?;

            let to = match to_override {
                Some(value) => value
                    .parse::<Mailbox>()
                    .with_context(|| format!("parsing cv contact email {value}"))?,
                None => self.default_to.clone(),
            };

            let email = Message::builder()
                .from(self.from.clone())
                .to(to)
                .reply_to(reply_to)
                .subject(subject)
                .singlepart(SinglePart::builder().header(ContentType::TEXT_PLAIN).body(body))?;

            self.transport.send(email).await.with_context(|| "sending contact email").map(|_| ())
        }
    }

    fn env_var(key: &str) -> AppResult<Option<String>> {
        match env::var(key) {
            Ok(value) => {
                let trimmed = value.trim().to_string();
                if trimmed.is_empty() { Ok(None) } else { Ok(Some(trimmed)) }
            },
            Err(VarError::NotPresent) => Ok(None),
            Err(error) => Err(error).with_context(|| format!("reading {key} from environment")),
        }
    }

    fn sanitize_header_value(value: &str, max_len: usize) -> String {
        value
            .chars()
            .filter(|character| *character != '\r' && *character != '\n')
            .take(max_len)
            .collect()
    }

    fn sanitize_body_value(value: &str, max_len: usize) -> String {
        sanitize_header_value(value, max_len)
    }

    fn sanitize_message_body(value: &str, max_len: usize) -> String {
        value
            .replace("\r\n", "\n")
            .replace('\r', "\n")
            .chars()
            .filter(|character| *character != '\0')
            .take(max_len)
            .collect()
    }

    #[cfg(test)]
    mod tests {
        use super::{sanitize_header_value, sanitize_message_body};

        #[test]
        fn message_body_keeps_newlines() {
            let sanitized = sanitize_message_body("hello\r\nworld\n\nagain", 2_000);
            assert_eq!(sanitized, "hello\nworld\n\nagain");
        }

        #[test]
        fn header_values_strip_newlines() {
            let sanitized = sanitize_header_value("hello\nworld", 120);
            assert_eq!(sanitized, "helloworld");
        }
    }
}

#[cfg(not(feature = "smtp"))]
#[allow(dead_code)]
mod implementation {
    use crate::{app::models::SiteData, error::AppResult};

    #[derive(Clone)]
    pub(crate) struct Mailer;

    pub(crate) struct CvContactPayload<'a> {
        pub full_name: &'a str,
        pub company: &'a str,
        pub email: &'a str,
        pub message: &'a str,
    }

    pub(crate) fn build_mailer(_site_data: &SiteData) -> AppResult<Option<Mailer>> {
        Ok(None)
    }

    impl Mailer {
        pub(crate) async fn send_cv_contact(
            &self,
            _payload: CvContactPayload<'_>,
            _to_override: Option<&str>,
        ) -> AppResult<()> {
            Ok(())
        }
    }
}

pub(crate) use implementation::*;
