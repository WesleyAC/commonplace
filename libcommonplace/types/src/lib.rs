use std::fmt;
use uuid::Uuid;
use serde::{Serialize, Deserialize};

#[derive(Debug)]
pub struct TagRow {
    pub id: Uuid,
    pub name: String,
    pub parent: Option<Uuid>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TagTree {
    pub id: Uuid,
    pub name: String,
    pub children: Vec<TagTree>,
    pub notes: Vec<Uuid>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Note {
    pub id: Uuid,
    pub hash: [u8; 32],
    pub name: String,
    pub mimetype: String,
}

impl From<&TagRow> for TagTree {
    fn from(tag_row: &TagRow) -> Self {
        TagTree {
            id: tag_row.id,
            name: tag_row.name.clone(),
            children: vec![],
            notes: vec![],
        }
    }
}

impl TagTree {
    fn pretty_print(&self, f: &mut fmt::Formatter, depth: usize) -> fmt::Result {
        write!(f, "{}{}: {:?}\n", " ".repeat(depth), self.name, self.notes)?;
        for child in &self.children {
            child.pretty_print(f, depth + 1)?
        }
        Ok(())
    }
}

impl fmt::Display for TagTree {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.pretty_print(f, 0)
    }
}
