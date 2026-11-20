//! The error type for this crate.

use thiserror::Error;

/// A result that uses the [`enum@Error`] type of this crate.
pub type Result<T> = std::result::Result<T, Error>;

/// An error that a core operation returns.
#[derive(Debug, Error)]
pub enum Error {
    /// The dashboard text is not valid TOML.
    #[error("The dashboard is not valid TOML. {0}")]
    DashboardSyntax(#[from] toml::de::Error),

    /// The dashboard is valid TOML, but the content is not correct.
    #[error("The dashboard is not correct. {0}")]
    Dashboard(String),

    /// A time value in the dashboard has a form that this crate cannot read.
    #[error("The time value {input:?} is not correct. {reason}")]
    Time {
        /// The text that this crate cannot read.
        input: String,
        /// The reason why this crate cannot read the text.
        reason: String,
    },

    /// The frame does not have the columns that the panel needs.
    #[error("The frame does not have the shape that this panel needs. {0}")]
    FrameShape(String),

    /// The dashboard names a data source that the registry does not hold.
    #[error("The registry does not hold the data source {0}.")]
    UnknownSource(String),

    /// The data source returned an error for a query.
    #[error("The data source returned an error. {0}")]
    Query(String),
}
