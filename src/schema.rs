//! Schema knowledge for constructor flows.
//!
//! mentci-lib needs to know what fields each record-kind has,
//! what enum variants are valid for typed fields, and which
//! source/target kind pairs each `RelationKind` actually
//! applies to. The constructor flows in [`crate::constructor`]
//! consume this knowledge to surface the right choices.
//!
//! **Source: sema, queried at runtime.** Per
//! [mentci/reports/119](https://github.com/LiGoldragon/mentci/blob/main/reports/119-schema-in-sema-corrected-direction-2026-04-30.md):
//! schema lives in sema as `KindDecl` / `FieldDecl` /
//! `VariantDecl` records, written by the engine's seed step at
//! first boot and read-only thereafter. `CompiledSchema` here
//! is the consumer side: every method below queries sema for
//! the relevant records, deserialises them into the trait's
//! return shapes, and hands them to the constructor flows.
//!
//! Do **NOT** wire `CompiledSchema` to read `signal::ALL_KINDS`
//! directly. That const exists only as the build-time input to
//! the seed projector; mentci-lib has no business reading it.
//! See beads `mentci-next-lvg` for the in-flight correction
//! and reports/119 §2.1 for the reasoning.
//!
//! Implementation arc:
//! 1. `KindDecl` / `FieldDecl` / `VariantDecl` lands in signal
//!    as new record kinds (with their own `#[derive(Schema)]`
//!    so they describe themselves — the recursion).
//! 2. The seed step pipes those records into sema on engine
//!    first boot.
//! 3. `CompiledSchema`'s methods become `Query` calls into
//!    criome via the existing connection driver.

use signal::RelationKind;

/// What the schema layer exposes to constructor flows.
pub trait SchemaSource {
    /// All record-kind names known to the schema.
    fn kinds(&self) -> Vec<String>;

    /// Field descriptions for one record-kind.
    fn fields_of(&self, kind_name: &str) -> Vec<FieldDesc>;

    /// Which `RelationKind` variants are valid as edges
    /// between a given source-kind and target-kind. When
    /// empty, the pair is meaningless.
    fn valid_relation_kinds(
        &self,
        source_kind: &str,
        target_kind: &str,
    ) -> Vec<RelationKind>;
}

/// Description of one field on a record-kind. Used by the
/// constructor-flow renderer to lay out fields.
#[derive(Debug, Clone)]
pub struct FieldDesc {
    pub name: String,
    pub ty: FieldType,
    pub is_required: bool,
}

/// Field shape, abstractly. Maps to typed constructor
/// widgets in the shell.
#[derive(Debug, Clone)]
pub enum FieldType {
    /// Free-form text input.
    Text,
    /// 64-bit integer.
    Integer,
    /// 64-bit float.
    Float,
    /// Boolean.
    Bool,
    /// A reference to another slot of a particular kind.
    SlotRef { of_kind: String },
    /// A typed enum — variants enumerated in the schema.
    Enum { variants: Vec<String> },
    /// A list of one of the above.
    List { item: Box<FieldType> },
}

/// The compile-time schema source — read from signal's typed
/// kinds at build time.
pub struct CompiledSchema;

impl SchemaSource for CompiledSchema {
    fn kinds(&self) -> Vec<String> {
        todo!()
    }

    fn fields_of(&self, _kind_name: &str) -> Vec<FieldDesc> {
        todo!()
    }

    fn valid_relation_kinds(
        &self,
        _source_kind: &str,
        _target_kind: &str,
    ) -> Vec<RelationKind> {
        todo!()
    }
}
