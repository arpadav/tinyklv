//! Top-level AST representation produced from a `#[derive(Klv)]` input
//!
//! Defines [`MainContainer`] (the parsed and validated view of an annotated
//! struct) and [`MainField`] (a single field within that struct). These types
//! are the interface between the attribute-parsing passes (`attr`, `symbol`)
//! and the code-generation passes (`expand`)
//!
//! Author: aav
pub(crate) mod attr;
pub(crate) mod symbol;
pub(crate) mod types;
