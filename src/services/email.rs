use lettre::{
    message::header::ContentType,
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};

/// Email service for sending scan reports
pub struct EmailService {
    smtp_host: String,
    smtp_port: u16,
    credentials: Credentials,
    from_address: String,
}

impl EmailService {
    /// Create a new email service
    pub fn new(
        smtp_host: String,
        smtp_port: u16,
        username: String,
        password: String,
        from_address: String,
    ) -> Self {
        let credentials = Credentials::new(username, password);
        Self {
            smtp_host,
            smtp_port,
            credentials,
            from_address,
        }
    }

    /// Send an email with the given subject and body
    pub async fn send_email(
        &self,
        to: &str,
        subject: &str,
        body: &str,
    ) -> anyhow::Result<()> {
        let email = Message::builder()
            .from(self.from_address.parse()?)
            .to(to.parse()?)
            .subject(subject)
            .header(ContentType::TEXT_PLAIN)
            .body(body.to_string())?;

        let mailer: AsyncSmtpTransport<Tokio1Executor> =
            AsyncSmtpTransport::<Tokio1Executor>::relay(&self.smtp_host)?
                .port(self.smtp_port)
                .credentials(self.credentials.clone())
                .build();

        mailer.send(email).await?;
        Ok(())
    }

    /// Send a scan report email with CSV attachment
    pub async fn send_report(
        &self,
        to: &str,
        scan_count: i32,
        _csv_data: &str,
    ) -> anyhow::Result<()> {
        let date = chrono::Local::now().format("%m/%d/%Y").to_string();
        let subject = format!("Cabinet Scan Report - {}", date);

        let body = format!(
            "Cabinet Scan Report\n\
            Date: {}\n\
            Total Scans: {}\n\n\
            Please find the scan data attached as CSV.\n\n\
            ---\n\
            This is an automated report from Cabinet.",
            date, scan_count
        );

        // For now, just send a simple text email
        // TODO: Add CSV attachment using lettre's multipart support
        self.send_email(to, &subject, &body).await
    }

    /// Test the SMTP connection
    pub async fn test_connection(&self) -> anyhow::Result<()> {
        let mailer: AsyncSmtpTransport<Tokio1Executor> =
            AsyncSmtpTransport::<Tokio1Executor>::relay(&self.smtp_host)?
                .port(self.smtp_port)
                .credentials(self.credentials.clone())
                .build();

        mailer.test_connection().await?;
        Ok(())
    }
}

/// Generate CSV data from scans
pub fn generate_csv_report(scans: &[crate::db::models::Scan]) -> anyhow::Result<String> {
    let mut wtr = csv::Writer::from_writer(vec![]);

    // Write header
    wtr.write_record([
        "Barcode",
        "Type",
        "Ticket Number",
        "Job Reference",
        "Device ID",
        "User ID",
        "Location",
        "Latitude",
        "Longitude",
        "Scanned At",
        "Synced At",
    ])?;

    // Write data
    for scan in scans {
        wtr.write_record([
            &scan.barcode,
            scan.barcode_type.as_deref().unwrap_or(""),
            scan.ticket_number.as_deref().unwrap_or(""),
            scan.barcode_job_ref.as_deref().unwrap_or(""),
            &scan.device_id,
            scan.user_id.as_deref().unwrap_or(""),
            scan.location.as_deref().unwrap_or(""),
            &scan.latitude.map(|l| l.to_string()).unwrap_or_default(),
            &scan.longitude.map(|l| l.to_string()).unwrap_or_default(),
            &scan.scanned_at,
            &scan.synced_at,
        ])?;
    }

    let data = String::from_utf8(wtr.into_inner()?)?;
    Ok(data)
}
