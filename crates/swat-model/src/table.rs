//! Database tables SWAT+ reads one derived type per row (`read (unit, *) table(i)`): topography,
//! hydrology, land use, curve numbers, snow and the like.
//!
//! Each table keeps its rows as values in the order of the Fortran type's components, described
//! by a schema ([`Field`]s taken from the type definition, with its defaults), and is looked up by
//! name as SWAT+ does: the first row whose name is equal, compared as Fortran compares strings.

/// The kind of a component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Int,
    Real,
    Text,
}

/// One component of a derived type: its name, kind, default (as written in the Fortran) and
/// array length (1 for a scalar).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Field {
    pub name: &'static str,
    pub kind: Kind,
    pub default: &'static str,
    pub count: usize,
}

/// A scalar component.
pub const fn f(name: &'static str, kind: Kind, default: &'static str) -> Field {
    Field { name, kind, default, count: 1 }
}

/// An array component of `count` values, all with the same default.
pub const fn a(name: &'static str, kind: Kind, default: &'static str, count: usize) -> Field {
    Field { name, kind, default, count }
}

/// A value.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Int(i32),
    Real(f32),
    Text(String),
}

impl Value {
    pub fn as_real(&self) -> Option<f32> {
        match self {
            Value::Real(v) => Some(*v),
            Value::Int(v) => Some(*v as f32),
            Value::Text(_) => None,
        }
    }
    pub fn as_int(&self) -> Option<i32> {
        match self {
            Value::Int(v) => Some(*v),
            _ => None,
        }
    }
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Value::Text(v) => Some(v),
            _ => None,
        }
    }
}

/// A table: its schema and rows, each row the type's values flattened (arrays expanded).
#[derive(Clone, Debug, PartialEq)]
pub struct Table {
    pub file: String,
    pub fields: &'static [Field],
    pub rows: Vec<Vec<Value>>,
}

/// The values of a field's defaults, flattened, for a new row.
pub fn default_row(fields: &[Field]) -> Vec<Value> {
    let mut row = Vec::new();
    for fd in fields {
        for _ in 0..fd.count {
            row.push(match fd.kind {
                Kind::Int => Value::Int(fd.default.trim().parse().unwrap_or(0)),
                Kind::Real => Value::Real(fd.default.trim().parse().unwrap_or(0.0)),
                Kind::Text => Value::Text(fd.default.to_string()),
            });
        }
    }
    row
}

impl Table {
    pub fn empty(file: &str, fields: &'static [Field]) -> Self {
        Self { file: file.to_string(), fields, rows: Vec::new() }
    }

    /// Position of the first value of `field` in a row.
    pub fn offset(&self, field: &str) -> Option<usize> {
        let mut at = 0;
        for fd in self.fields {
            if fd.name == field {
                return Some(at);
            }
            at += fd.count;
        }
        None
    }

    /// Value `index` of `field` in row `row` (0-based).
    pub fn get(&self, row: usize, field: &str, index: usize) -> Option<&Value> {
        let at = self.offset(field)? + index;
        self.rows.get(row)?.get(at)
    }

    pub fn real(&self, row: usize, field: &str) -> Option<f32> {
        self.get(row, field, 0)?.as_real()
    }

    pub fn int(&self, row: usize, field: &str) -> Option<i32> {
        self.get(row, field, 0)?.as_int()
    }

    pub fn text(&self, row: usize, field: &str) -> Option<&str> {
        self.get(row, field, 0)?.as_text()
    }

    /// The first row named `name` (its first component), as SWAT+'s linear lookups find it.
    pub fn find(&self, name: &str) -> Option<usize> {
        (0..self.rows.len())
            .find(|&r| self.rows.get(r).and_then(|row| row.first()).and_then(Value::as_text).is_some_and(|n| fortran_eq(n, name)))
    }
}

/// Fortran string equality: trailing blanks don't count.
pub fn fortran_eq(a: &str, b: &str) -> bool {
    a.trim_end_matches(' ') == b.trim_end_matches(' ')
}
