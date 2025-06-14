use std::{error::Error as _, fmt};

use fj_core::{
    Core,
    algorithms::{bounding_volume::BoundingVolume, triangulate::Triangulate},
    validation::{ValidationConfig, ValidationErrors},
};
use fj_interop::{InvalidTolerance, Tolerance};
use fj_math::{Aabb, Point, Scalar};
use fj_viewer::make_viewer_and_spawn_thread;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{Args, export, viewer};

/// An instance of Fornjot
///
/// This is the main entry point into the Fornjot API.
#[derive(Default)]
pub struct Instance {
    /// The instance of the Fornjot core
    pub core: Core,
}

impl Instance {
    /// Construct an instance of `Instance`
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct an instance of `Instance`, using the provided configuration
    pub fn with_validation_config(config: ValidationConfig) -> Self {
        let core = fj_core::Core::with_validation_config(config);
        Self { core }
    }

    /// Export or display a model, according to CLI arguments
    ///
    /// This function is intended to be called by applications that define a
    /// model and want to provide a standardized CLI interface for dealing with
    /// that model.
    ///
    /// This function is used by Fornjot's own testing infrastructure, but is
    /// useful beyond that, when using Fornjot directly to define a model.
    ///
    /// # Errors
    ///
    /// Returns an error if logger setup fails, validation fails, tolerance is invalid,
    /// model is empty, or export/display fails.
    pub fn process_model<M>(&mut self, model: &M) -> Result
    where
        for<'r> (&'r M, Tolerance): Triangulate,
        for<'r> &'r M: BoundingVolume<3>,
    {
        let args = Args::parse();
        self.process_model_with_args(model, args)
    }

    /// Like [`process_model`], but allows passing CLI arguments directly.
    ///
    /// This is useful for testing and programmatic use.
    pub fn process_model_with_args<M>(&mut self, model: &M, args: Args) -> Result
    where
        for<'r> (&'r M, Tolerance): Triangulate,
        for<'r> &'r M: BoundingVolume<3>,
    {
        self.init_logger()?;

        if !args.ignore_validation {
            self.core.layers.validation.take_errors()?;
        }

        let aabb = model.aabb(&self.core.layers.geometry).unwrap_or(Aabb {
            min: Point::origin(),
            max: Point::origin(),
        });

        let tolerance = self.compute_tolerance(&aabb, args.tolerance)?;

        let tri_mesh = (model, tolerance).triangulate(&mut self.core);

        if let Some(path) = args.export {
            export::export(tri_mesh.all_triangles(), &path)?;
            return Ok(());
        }

        make_viewer_and_spawn_thread(|viewer| {
            viewer.display_model(tri_mesh);
        })?;

        Ok(())
    }

    /// Initialize the logger, if not already set.
    fn init_logger(&self) -> std::result::Result<(), tracing::subscriber::SetGlobalDefaultError> {
        // try_init returns an error if the logger is already set, which is fine in tests.
        let _ = tracing_subscriber::registry()
            .with(tracing_subscriber::fmt::layer())
            .with(tracing_subscriber::EnvFilter::from_default_env())
            .try_init();
        Ok(())
    }

    /// Compute the tolerance to use for triangulation.
    ///
    /// Returns an error if the bounding box is degenerate (all extents zero).
    fn compute_tolerance(
        &self,
        aabb: &Aabb<3>,
        user_defined: Option<Tolerance>,
    ) -> std::result::Result<Tolerance, Error> {
        match user_defined {
            Some(tol) => Ok(tol),
            None => {
                let mut min_extent = Scalar::MAX;
                let mut found_nonzero = false;
                for extent in aabb.size().components {
                    if extent > Scalar::ZERO && extent < min_extent {
                        min_extent = extent;
                        found_nonzero = true;
                    }
                }
                if !found_nonzero {
                    return Err(Error::EmptyModel);
                }
                let tolerance = min_extent / Scalar::from_f64(1000.);
                Ok(Tolerance::from_scalar(tolerance)?)
            }
        }
    }
}

/// Return value of [`Instance::process_model`]
pub type Result = std::result::Result<(), Error>;

/// Error returned by [`Instance::process_model`]
#[derive(thiserror::Error)]
pub enum Error {
    /// Failed to set up logger
    #[error("Failed to set up logger")]
    Tracing(#[from] tracing::subscriber::SetGlobalDefaultError),

    /// Error displaying model
    #[error("Error displaying model")]
    Display(#[from] viewer::Error),

    /// Error exporting model
    #[error("Error exporting model")]
    Export(#[from] export::Error),

    /// Invalid tolerance
    #[error(transparent)]
    Tolerance(#[from] InvalidTolerance),

    /// Unhandled validation errors
    #[error(transparent)]
    Validation(#[from] ValidationErrors),

    /// The model is empty or has zero size
    #[error("The model is empty or has zero size; cannot compute tolerance")]
    EmptyModel,
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // When returning an error from Rust's `main` function, the runtime uses
        // the error's `Debug` implementation to display it, not the `Display`
        // one. This is unfortunate, and forces us to override `Debug` here.

        // We should be able to replace this with `Report`, once it is stable:
        // https://doc.rust-lang.org/std/error/struct.Report.html

        write!(f, "{self}")?;

        let mut source = self.source();

        if source.is_some() {
            write!(f, "\n\nCaused by:")?;
        }

        let mut i = 0;
        while let Some(s) = source {
            write!(f, "\n    {i}: {s}")?;
            source = s.source();
            i += 1;
        }

        Ok(())
    }
}
