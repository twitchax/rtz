//! All of the types used in the library.

// Result types.

/// A shortened version of [`anyhow::Result<T>`].
pub type Res<T> = anyhow::Result<T>;
/// A shortened version of [`anyhow::Result<()>`](anyhow::Result).
pub type Void = anyhow::Result<()>;
/// A shortened version of [`anyhow::Error`].
#[allow(dead_code)]
pub type Err = anyhow::Error;

// Precision types.

/// The (double-precision) floating point type used in the library.
#[cfg(feature = "double-precision")]
pub type Float = f64;
/// The (single-precision) floating point type used in the library.
#[cfg(not(feature = "double-precision"))]
pub type Float = f32;

