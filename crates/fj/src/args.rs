use std::{num::ParseFloatError, path::PathBuf, str::FromStr};

use fj_interop::{InvalidTolerance, Tolerance};
use fj_math::Scalar;

/// Standardized CLI for Fornjot models.
///
/// Provides a unified interface for exporting and validating models.
/// Used by example models and testing infrastructure.
#[derive(clap::Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Export model to this path
    #[arg(short, long, value_name = "PATH", help = "Path to export the model")]
    pub export: Option<PathBuf>,

    /// How much the export can deviate from the original model
    #[arg(
        short,
        long,
        value_parser = parse_tolerance,
        help = "Tolerance for export deviation (e.g. 0.001)"
    )]
    pub tolerance: Option<Tolerance>,

    /// Ignore validation errors
    #[arg(short, long, help = "Ignore validation errors during export")]
    pub ignore_validation: bool,
}

impl Args {
    /// Parse the command-line arguments.
    ///
    /// Convenience method that saves the caller from having to import the
    /// `clap::Parser` trait.
    pub fn parse() -> Self {
        <Self as clap::Parser>::parse()
    }
}

/// Parse a string into a Tolerance value.
fn parse_tolerance(input: &str) -> Result<Tolerance, ArgsError> {
    let tolerance = f64::from_str(input)
        .map_err(ArgsError::ParseTolerance)?;
    let tolerance = Scalar::from_f64(tolerance);
    Tolerance::from_scalar(tolerance)
        .map_err(ArgsError::InvalidTolerance)
}

#[derive(Debug, thiserror::Error)]
pub enum ArgsError {
    #[error("Error parsing tolerance: {0}")]
    ParseTolerance(#[from] ParseFloatError),

    #[error("Invalid tolerance: {0}")]
    InvalidTolerance(#[from] InvalidTolerance),
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_args_parse_export() {
        let args = Args::parse_from(&["test", "--export", "foo.step"]);
        assert_eq!(args.export, Some(PathBuf::from("foo.step")));
    }

    #[test]
    fn test_args_parse_tolerance() {
        let args = Args::parse_from(&["test", "--tolerance", "0.01"]);
        assert!(args.tolerance.is_some());
    }

    #[test]
    fn test_args_parse_ignore_validation() {
        let args = Args::parse_from(&["test", "--ignore-validation"]);
        assert!(args.ignore_validation);
    }
}
