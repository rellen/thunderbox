//! The Prometheus data source of thunderbox.
//!
//! This data source answers a query with a range query of the Prometheus HTTP
//! API. It is the first data source of thunderbox that reads a server on another
//! machine. `docs/contributor/architecture.md` lists it.
//!
//! The data source turns the point limit of a panel into the step of the range
//! query. The step is the distance between two points. A panel that is 80 cells
//! wide and that draws with a braille marker asks for 160 points. The data
//! source then selects a step that gives 160 points or fewer, and the server
//! does the work of reduction. The `step` module states the rules.
//!
//! # Examples
//!
//! ```no_run
//! use thunderbox_source_prometheus::{Prometheus, Settings};
//!
//! # fn main() -> thunderbox_core::Result<()> {
//! let source = Prometheus::new("http://127.0.0.1:9090", Settings::default())?;
//! # Ok(())
//! # }
//! ```
//!
//! A dashboard declares the same data source in this form:
//!
//! ```toml
//! [source.prom]
//! kind = "prometheus"
//! url = "http://127.0.0.1:9090"
//! min_step = "15s"
//! timeout = "30s"
//! ```

mod convert;
mod step;

use std::collections::BTreeMap;
use std::time::Duration;

use async_trait::async_trait;
use prometheus_http_query::Client;
use thunderbox_core::{Error, Frame, QueryRequest, Result, SourceDecl, parse_duration_ms};
use thunderbox_source::{DataSource, SourceDescriptor};

use crate::step::Grid;

/// The type name that a dashboard writes in the `kind` field.
pub const KIND: &str = "prometheus";

/// The time that this data source adds to its own timeout, for the caller.
///
/// The client stops a request at the timeout of the settings. This data source
/// then builds an error that names the server and the fault. The descriptor
/// states the sum of the two values, so the caller waits for that error and
/// reports no wait of its own. ADR 0012 in `docs/contributor/adr` states the third duty of a
/// data source.
const ANSWER_MARGIN: Duration = Duration::from_secs(5);

/// The address that this data source reads when a dashboard names none.
///
/// A Prometheus server listens on this address when nobody changes its
/// settings.
pub const DEFAULT_URL: &str = "http://127.0.0.1:9090";

/// The settings of a [`Prometheus`] data source.
///
/// The default value suits a server that a person runs on the same machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// The name that the user interface shows for this data source.
    pub name: String,
    /// The smallest step that this data source asks the server for, in
    /// milliseconds.
    ///
    /// Set this value to the interval at which the server collects the metric.
    /// A step below that interval gives a chart that holds more holes than
    /// values.
    pub min_step_ms: i64,
    /// The longest time that this data source waits for an answer.
    pub timeout: Duration,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            name: "Prometheus".to_string(),
            min_step_ms: step::MIN_STEP_MS,
            timeout: Duration::from_secs(30),
        }
    }
}

impl Settings {
    /// Reads the settings from the options of a dashboard.
    ///
    /// The function reads these keys. It returns an error for any other key.
    ///
    /// - `name`, which holds a text;
    /// - `min_step`, which holds a duration such as `15s`;
    /// - `timeout`, which holds a duration such as `30s`.
    ///
    /// # Errors
    ///
    /// This function returns an error in each of these conditions:
    ///
    /// - the options hold a key that is not in the list above;
    /// - a value does not have the type that its key needs;
    /// - a duration has a form that thunderbox cannot read.
    pub fn from_options(options: &BTreeMap<String, toml::Value>) -> Result<Self> {
        let mut settings = Self::default();
        for (key, value) in options {
            match key.as_str() {
                "name" => settings.name = text(key, value)?.to_string(),
                "min_step" => settings.min_step_ms = duration_ms(key, value)?,
                "timeout" => {
                    let millis = duration_ms(key, value)?;
                    let millis = u64::try_from(millis).map_err(|_| {
                        Error::Dashboard(format!("the {key} of a prometheus source is negative"))
                    })?;
                    settings.timeout = Duration::from_millis(millis.max(1));
                }
                other => {
                    return Err(Error::Dashboard(format!(
                        "a prometheus source has no option {other:?}; it takes \
                         url, name, min_step, and timeout"
                    )));
                }
            }
        }
        Ok(settings)
    }
}

/// Returns the text of an option.
///
/// # Errors
///
/// This function returns an error if the value is not a text value.
fn text<'a>(key: &str, value: &'a toml::Value) -> Result<&'a str> {
    value.as_str().ok_or_else(|| {
        Error::Dashboard(format!(
            "the {key} of a prometheus source must be a text value"
        ))
    })
}

/// Returns a duration option as a count of milliseconds.
///
/// # Errors
///
/// This function returns an error if the value is not a text value, or if
/// thunderbox cannot read the duration.
fn duration_ms(key: &str, value: &toml::Value) -> Result<i64> {
    parse_duration_ms(text(key, value)?)
}

/// A data source that reads a Prometheus server.
#[derive(Clone)]
pub struct Prometheus {
    client: Client,
    settings: Settings,
}

/// Returns the address without the password that it can hold.
///
/// A person can write a user name and a password into the address of a server.
/// The client reads that pair, and it builds a header of the kind `Basic` from
/// it. The address is therefore one place that a password lives. Every text
/// that thunderbox writes about a data source passes through this function, so no
/// such text holds a password.
///
/// The function keeps the user name, because a user name is not a secret and it
/// tells a person which address the message names.
///
/// The function reads the text, and it parses no address. A caller gives it an
/// address that no parser accepted, so the function cannot depend on a parser.
/// Returns the address without the password that it may hold.
///
/// A person can write a user name and a password into the `url` field of a
/// Prometheus data source. The client reads that pair and builds a header of
/// the kind `Basic` from it, so the address is one place that a password lives.
///
/// Every text of this crate that names an address calls this function. That
/// rule binds each `Debug` implementation, each `Display` implementation, each
/// log message, each panic message and each error. A text that a person reads,
/// and a text that a file holds, must carry no password.
/// ADR 0033 in `docs/contributor/adr` states the rule in full.
///
/// The function keeps the user name. A user name is no secret, and it names
/// which server the message means.
///
/// The function reads the text and parses no address, because a caller gives it
/// an address that no parser accepts.
fn without_password(address: &str) -> String {
    let Some((scheme, rest)) = address.split_once("://") else {
        return address.to_string();
    };
    // The user name and the password stand before the first `@`, and that `@`
    // stands before the path, the query and the fragment.
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let Some(at) = rest[..end].rfind('@') else {
        return address.to_string();
    };
    let pair = &rest[..at];
    let name = pair.split_once(':').map_or(pair, |(name, _)| name);
    format!("{scheme}://{name}@{}", &rest[at + 1..])
}

/// Returns the text of an error, and the text of each error that caused it.
///
/// `reqwest` writes `builder error` for each fault of its builder, and it keeps
/// the real fault in [`std::error::Error::source`]. A person who reads the
/// first text alone cannot repair the fault. A machine that holds no
/// certificate of a certificate authority gives this text:
///
/// ```text
/// builder error: unexpected error: No CA certificates were loaded from the system
/// ```
///
/// An error of the builder names no address, so this text holds no password.
/// A caller that gives this function another error must check that rule.
fn with_causes(error: &dyn std::error::Error) -> String {
    let mut text = error.to_string();
    let mut cause = error.source();
    while let Some(next) = cause {
        text.push_str(": ");
        text.push_str(&next.to_string());
        cause = next.source();
    }
    text
}

impl std::fmt::Debug for Prometheus {
    /// Writes the address and the settings.
    ///
    /// The client of the Prometheus crate holds no `Debug` implementation.
    /// This implementation writes the address in its place. It writes no
    /// password, because this crate removes a password from every address that
    /// it writes.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Prometheus")
            .field("url", &without_password(self.client.base_url().as_str()))
            .field("settings", &self.settings)
            .finish()
    }
}

impl Prometheus {
    /// Returns a data source that reads the server at `url`.
    ///
    /// # Errors
    ///
    /// This function returns an error in each of these conditions:
    ///
    /// - `url` is not an address that thunderbox can read;
    /// - the program cannot build a client that speaks HTTP.
    ///
    /// On Linux, the client reads the certificates of each certificate
    /// authority from the machine. It returns an error when the machine holds
    /// none, and it does so for an address of plain HTTP as well. The variable
    /// `SSL_CERT_FILE` names a file of such certificates.
    pub fn new(url: &str, settings: Settings) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(settings.timeout)
            .build()
            .map_err(|e| {
                Error::Dashboard(format!(
                    "thunderbox cannot build a client. {}",
                    with_causes(&e)
                ))
            })?;
        let client = Client::from(http, url).map_err(|e| {
            Error::Dashboard(format!(
                "the address {:?} of a prometheus source is not correct. {e}",
                without_password(url)
            ))
        })?;
        Ok(Self { client, settings })
    }

    /// Returns a data source from the declaration in a dashboard.
    ///
    /// The declaration supplies the address in its `url` field. The data source
    /// reads [`DEFAULT_URL`] when the field is not there.
    ///
    /// # Errors
    ///
    /// This function returns an error if the declaration holds an option that
    /// [`Settings::from_options`] rejects, or an address that
    /// [`Prometheus::new`] rejects.
    pub fn from_declaration(declaration: &SourceDecl) -> Result<Self> {
        let settings = Settings::from_options(&declaration.options)?;
        let url = declaration.url.as_deref().unwrap_or(DEFAULT_URL);
        Self::new(url, settings)
    }

    /// Returns the settings of this data source.
    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    /// Returns the timeout that the server applies to a query, in seconds.
    ///
    /// The client stops a request that takes too long. This value asks the
    /// server to stop the same request, so that the server does no work that
    /// nobody reads. The value is 1 second or more.
    fn server_timeout_seconds(&self) -> i64 {
        i64::try_from(self.settings.timeout.as_secs().max(1)).unwrap_or(i64::MAX)
    }
}

#[async_trait]
impl DataSource for Prometheus {
    fn descriptor(&self) -> SourceDescriptor {
        // The server reduces the number of points to the step of the query, so
        // the caller does not reduce them again.
        SourceDescriptor::new(KIND, self.settings.name.clone())
            .downsampling()
            .with_label_values()
            .with_answer_limit(self.settings.timeout + ANSWER_MARGIN)
    }

    async fn query(&self, request: QueryRequest) -> Result<Vec<Frame>> {
        let expr = request.query.expr.trim();
        if expr.is_empty() {
            return Err(Error::Query("the query holds no text".into()));
        }
        let grid = Grid::for_request(&request, self.settings.min_step_ms);
        let answer = self
            .client
            .query_range(
                expr,
                grid.start_seconds(),
                grid.end_seconds(),
                grid.step_seconds(),
            )
            .timeout(self.server_timeout_seconds())
            .get()
            .await
            .map_err(|e| Error::Query(e.to_string()))?;
        convert::frames(expr, &grid, answer.into_inner().0)
    }

    /// Returns the values of one label.
    ///
    /// The text of the query is the name of the label. The text `instance` is
    /// an example, and it gives the address of each machine that the server
    /// collects from.
    async fn label_values(&self, query: &str) -> Result<Vec<String>> {
        let label = query.trim();
        if label.is_empty() {
            return Err(Error::Query(
                "a variable of a prometheus source needs the name of a label".into(),
            ));
        }
        self.client
            .label_values(label)
            .get()
            .await
            .map_err(|e| Error::Query(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(text: &str) -> BTreeMap<String, toml::Value> {
        let table: toml::Table = text.parse().expect("The test must hold correct TOML.");
        table.into_iter().collect()
    }

    #[test]
    fn the_default_settings_suit_a_server_on_this_machine() {
        let settings = Settings::default();
        assert_eq!(settings.min_step_ms, 1_000);
        assert_eq!(settings.timeout, Duration::from_secs(30));
    }

    #[test]
    fn from_options_reads_every_key() {
        let settings = Settings::from_options(&options(
            "name = \"prod\"\nmin_step = \"15s\"\ntimeout = \"5s\"",
        ))
        .unwrap();
        assert_eq!(settings.name, "prod");
        assert_eq!(settings.min_step_ms, 15_000);
        assert_eq!(settings.timeout, Duration::from_secs(5));
    }

    #[test]
    fn from_options_returns_an_error_for_a_key_that_it_does_not_know() {
        let error = Settings::from_options(&options("stpe = \"15s\""))
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("stpe"),
            "The message must name the key. {error}"
        );
        assert!(
            error.contains("min_step"),
            "The message must list the correct keys. {error}"
        );
    }

    #[test]
    fn from_options_returns_an_error_for_a_duration_that_it_cannot_read() {
        assert!(Settings::from_options(&options("min_step = \"soon\"")).is_err());
        assert!(Settings::from_options(&options("min_step = 15")).is_err());
    }

    #[test]
    fn from_declaration_reads_the_address_and_the_options() {
        let declaration: SourceDecl = toml::from_str(
            "kind = \"prometheus\"\nurl = \"http://example.com:9090\"\nmin_step = \"30s\"",
        )
        .unwrap();
        let source = Prometheus::from_declaration(&declaration).unwrap();
        assert_eq!(source.settings().min_step_ms, 30_000);
    }

    #[test]
    fn from_declaration_reads_the_default_address_when_the_dashboard_names_none() {
        let declaration: SourceDecl = toml::from_str("kind = \"prometheus\"").unwrap();
        assert!(Prometheus::from_declaration(&declaration).is_ok());
    }

    #[test]
    fn new_returns_an_error_for_an_address_that_is_not_correct() {
        let error = Prometheus::new("not an address", Settings::default())
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("not an address"),
            "The message is wrong. {error}"
        );
    }

    #[test]
    fn the_descriptor_states_that_the_server_reduces_the_points() {
        let source = Prometheus::new(DEFAULT_URL, Settings::default()).unwrap();
        let descriptor = source.descriptor();
        assert_eq!(descriptor.kind, "prometheus");
        assert!(
            descriptor.downsamples,
            "The step of the query does the reduction, so thunderbox must not do it again."
        );
        assert!(descriptor.supports_label_values);
    }

    /// The third duty of a data source is a limited time. This data source
    /// keeps its own timeout, and the descriptor states a value that is a
    /// little longer. A person then reads the error of this data source, and
    /// not the wait of the caller.
    #[test]
    fn the_descriptor_states_a_limit_that_is_longer_than_the_timeout() {
        let settings = Settings {
            timeout: Duration::from_secs(30),
            ..Settings::default()
        };
        let source = Prometheus::new(DEFAULT_URL, settings).unwrap();
        let limit = source
            .descriptor()
            .answer_limit
            .expect("This data source must state its own limit.");
        assert!(
            limit > Duration::from_secs(30),
            "The limit must leave this data source the time to build its error. {limit:?}"
        );
    }

    #[test]
    fn the_server_timeout_is_one_second_or_more() {
        let settings = Settings {
            timeout: Duration::from_millis(1),
            ..Settings::default()
        };
        let source = Prometheus::new(DEFAULT_URL, settings).unwrap();
        assert_eq!(source.server_timeout_seconds(), 1);
    }

    /// A person can write a password into the address of a server. The `Debug`
    /// text reaches a log and a panic message, so it must hold no password.
    #[test]
    fn the_debug_text_holds_no_password() {
        let source = Prometheus::new("http://alice:s3cret@127.0.0.1:9090", Settings::default())
            .expect("The address must be correct.");
        let text = format!("{source:?}");
        assert!(
            !text.contains("s3cret"),
            "The Debug text must hold no password. {text}"
        );
        assert!(
            text.contains("alice"),
            "The Debug text must keep the user name. {text}"
        );
    }

    /// A dashboard that names an address which no parser accepts gives this
    /// error to a person. That address can hold a password.
    #[test]
    fn the_error_of_an_address_that_is_not_correct_holds_no_password() {
        let error = Prometheus::new("http://alice:s3cret@ho st:9090", Settings::default())
            .expect_err("The address must be wrong.")
            .to_string();
        assert!(
            !error.contains("s3cret"),
            "The error must hold no password. {error}"
        );
        assert!(
            error.contains("alice"),
            "The error must keep the user name. {error}"
        );
    }

    #[test]
    fn without_password_keeps_an_address_that_holds_no_pair() {
        assert_eq!(
            without_password("http://127.0.0.1:9090"),
            "http://127.0.0.1:9090"
        );
        assert_eq!(without_password("not an address"), "not an address");
    }

    #[test]
    fn without_password_keeps_a_user_name_that_stands_alone() {
        assert_eq!(
            without_password("http://alice@127.0.0.1:9090"),
            "http://alice@127.0.0.1:9090"
        );
    }

    /// A path can hold the character `@`, and that `@` names no user name.
    #[test]
    fn without_password_reads_no_pair_from_a_path() {
        assert_eq!(
            without_password("http://127.0.0.1:9090/a@b"),
            "http://127.0.0.1:9090/a@b"
        );
    }

    /// A password can hold the character `@`. The pair then ends at the last
    /// `@` that stands before the path.
    #[test]
    fn without_password_reads_the_last_mark_of_the_pair() {
        assert_eq!(
            without_password("http://alice:a@b@127.0.0.1:9090/x"),
            "http://alice@127.0.0.1:9090/x"
        );
    }

    /// An error that holds a cause, for the tests of `with_causes`.
    #[derive(Debug)]
    struct Layer {
        text: &'static str,
        cause: Option<Box<Layer>>,
    }

    impl std::fmt::Display for Layer {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(self.text)
        }
    }

    impl std::error::Error for Layer {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            self.cause
                .as_deref()
                .map(|c| c as &(dyn std::error::Error + 'static))
        }
    }

    #[test]
    fn the_text_of_an_error_holds_each_cause() {
        let error = Layer {
            text: "builder error",
            cause: Some(Box::new(Layer {
                text: "unexpected error",
                cause: Some(Box::new(Layer {
                    text: "no certificate",
                    cause: None,
                })),
            })),
        };
        assert_eq!(
            with_causes(&error),
            "builder error: unexpected error: no certificate"
        );
        let alone = Layer {
            text: "builder error",
            cause: None,
        };
        assert_eq!(with_causes(&alone), "builder error");
    }

    /// The name of the test below, which a child process runs.
    #[cfg(target_os = "linux")]
    const CHILD: &str = "tests::a_client_on_a_machine_that_holds_no_certificate";

    /// A child process runs this test, and it writes the result. The test
    /// `the_error_of_the_client_names_its_cause` reads that result.
    ///
    /// The test asserts nothing, so it passes on a machine that holds a
    /// certificate as well. The attribute `ignore` keeps it out of an ordinary
    /// run.
    #[cfg(target_os = "linux")]
    #[test]
    #[ignore = "the test the_error_of_the_client_names_its_cause runs this test"]
    fn a_client_on_a_machine_that_holds_no_certificate() {
        match Prometheus::new("http://127.0.0.1:9090", Settings::default()) {
            Ok(_) => println!("RESULT: the client builds"),
            Err(error) => println!("RESULT: {error}"),
        }
    }

    /// A machine that holds no certificate of a certificate authority cannot
    /// build a client on Linux. The error must name that cause. The text
    /// `builder error` alone told a person nothing, and the sandbox of Nix
    /// showed only that text.
    ///
    /// The test sets the variable in a child process. A variable that this
    /// process set would reach every other test of this process. Cargo also
    /// replaces a variable that names a file which does not exist. It does so
    /// before it starts a test, so this process cannot rely on that variable.
    #[cfg(target_os = "linux")]
    #[test]
    fn the_error_of_the_client_names_its_cause() {
        let out = std::process::Command::new(
            std::env::current_exe().expect("The test must know its own program."),
        )
        .args(["--exact", CHILD, "--ignored", "--nocapture"])
        .env("SSL_CERT_FILE", "/no-cert-file.crt")
        .env_remove("SSL_CERT_DIR")
        .output()
        .expect("The test must be able to start its own program.");
        let text = String::from_utf8_lossy(&out.stdout);
        let result = text
            .lines()
            .find_map(|line| line.strip_prefix("RESULT: "))
            .unwrap_or_else(|| panic!("The child process wrote no result. {text}"));
        assert!(
            result.contains("thunderbox cannot build a client. builder error: "),
            "The error must name the cause after the text of the builder. {result}"
        );
    }
}
