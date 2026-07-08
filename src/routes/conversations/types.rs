pub use crate::engine::orchestrator::{
    StructuredResponse, SourcePart, UnknownPart
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VizKind { Svg, Html, Asset, Search, Data }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VizBlock { pub kind: VizKind, pub payload: String }
