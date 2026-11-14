//! Shared GPUI controls. Workspace geometry lives in `design`; colors come from `crate::theme`.
//! Screens own their layout, actions, and data.

mod components;
mod date_format;
pub(crate) mod design;
mod painting;
mod window;

pub use components::*;
pub use date_format::*;
pub use painting::*;
pub use window::*;
