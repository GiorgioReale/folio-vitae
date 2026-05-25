#[cfg(feature = "smtp")]
mod implementation {
    use std::env::{self, VarError};

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
        to: Mailbox,
    }

    #[derive(Clone)]
    pub(crate) struct CvContactPayload {
        pub full_name: String,
        pub company: String,
        pub email: String,
        pub message: String,
    }

    pub(crate) fn build_mailer(site_data: &SiteData) -> AppResult<Option<Mailer>> {
        let host = match env_var("SMTP_HOST")? {
            Some(value) => value,
            None => return Ok(None),
        };
        let port: u16 = match env_var("SMTP_PORT")? {
            Some(value) => value.parse().with_context(|| "parsing SMTP_PORT".to_string())?,
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

        let transport = AsyncSmtpTransport::<Tokio1Executor>::relay(&host)
            .with_context(|| format!("building smtp transport for {host}"))?
            .port(port)
            .credentials(credentials)
            .build();

        let from_mailbox =
            from.parse::<Mailbox>().with_context(|| format!("parsing SMTP_FROM address {from}"))?;
        let cv_email = site_data.contact.emails.resolved_cv();
        let to_mailbox = cv_email
            .parse::<Mailbox>()
            .with_context(|| format!("parsing cv contact email {cv_email}"))?;

        Ok(Some(Mailer { transport, from: from_mailbox, to: to_mailbox }))
    }

    impl Mailer {
        pub(crate) async fn send_cv_contact(&self, payload: CvContactPayload) -> AppResult<()> {
            let subject_name = sanitize_header_value(&payload.full_name, MAX_SUBJECT_NAME_LENGTH);
            let subject = format!("Nuovo contatto dal CV - {subject_name}");

            let body = format!(
                "Nuovo messaggio inviato dal form CV.\n\nNome completo: {}\nAzienda: {}\nEmail: {}\n\nMessaggio:\n{}\n",
                sanitize_body_value(&payload.full_name, MAX_SUBJECT_NAME_LENGTH),
                sanitize_body_value(&payload.company, MAX_COMPANY_LENGTH),
                sanitize_body_value(&payload.email, MAX_EMAIL_LENGTH),
                sanitize_body_value(&payload.message, MAX_MESSAGE_LENGTH)
            );

            let email = Message::builder()
                .from(self.from.clone())
                .to(self.to.clone())
                .subject(subject)
                .singlepart(SinglePart::builder().header(ContentType::TEXT_PLAIN).body(body))?;

            self.transport
                .send(email)
                .await
                .with_context(|| "sending contact email".to_string())
                .map(|_| ())
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
        value
            .chars()
            .filter(|character| *character != '\r' && *character != '\n')
            .take(max_len)
            .collect()
    }
}

#[cfg(not(feature = "smtp"))]
#[allow(dead_code)]
mod implementation {
    use crate::{app::models::SiteData, error::AppResult};

    #[derive(Clone)]
    pub(crate) struct Mailer;

    #[derive(Clone)]
    pub(crate) struct CvContactPayload {
        pub full_name: String,
        pub company: String,
        pub email: String,
        pub message: String,
    }

    pub(crate) fn build_mailer(_site_data: &SiteData) -> AppResult<Option<Mailer>> {
        Ok(None)
    }

    impl Mailer {
        pub(crate) async fn send_cv_contact(&self, _payload: CvContactPayload) -> AppResult<()> {
            Ok(())
        }
    }
}

pub(crate) use implementation::*;
