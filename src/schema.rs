//! Schema knowledge for constructor flows.
//!
//! mentci-lib needs to know what fields each record-kind has,
//! what enum variants are valid for typed fields, and which
//! source/target kind pairs each `RelationKind` actually
//! applies to. The constructor flows in [`crate::constructor`]
//! consume this knowledge to surface the right choices.
//!
//! **Today:** schema is compiled in — derived at build-time
//! from signal's hand-written record-kind types.
//!
//! **Tomorrow:** schema-in-sema. Per criome ARCH and report
//! 111 §13, signal's record-kind type definitions become
//! records in sema themselves ("datatypes-datatypes"). At
//! that point this module reads schema from sema records and
//! the rest of mentci-lib is unchanged. The contract — schema
//! is data the constructors consume — is the same in both
//! eras.

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
