use std::fmt;
use uuid::Uuid;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct NoteId {
    pub uuid: Uuid,
}

impl fmt::Display for NoteId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.uuid)
    }
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct TagId {
    pub uuid: Uuid,
}

impl fmt::Display for TagId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.uuid)
    }
}

#[derive(Debug)]
pub struct TagRow {
    pub id: TagId,
    pub name: String,
    pub parent: Option<TagId>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TagTree {
    pub id: TagId,
    pub name: String,
    pub children: Vec<TagTree>,
    pub notes: Vec<NoteId>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Note {
    pub id: Uuid,
    pub hash: [u8; 32],
    pub name: String,
    pub mimetype: String,
}

#[derive(Debug, Clone)]
pub enum NoteOrTag<'a> {
    Note(&'a Note),
    Tag(&'a TagTree),
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

pub fn get_by_uuid<'a>(tag_tree: &'a Vec<TagTree>, notes: &'a HashMap<Uuid, Note>, uuid: Uuid) -> Option<NoteOrTag<'a>> {
    if let Some(note) = notes.get(&uuid) {
        return Some(NoteOrTag::Note(&note));
    }
    for tree in tag_tree {
        if tree.id.uuid == uuid {
            return Some(NoteOrTag::Tag(&tree));
        }
        if let Some(ret) = get_by_uuid(&tree.children, notes, uuid) {
            return Some(ret);
        }
    }
    None
}

pub fn get_by_full_name<'a>(tag_tree: &'a Vec<TagTree>, notes: &'a HashMap<Uuid, Note>, name: Vec<String>) -> Option<NoteOrTag<'a>> {
    if let Some((head, tail)) = name.split_first() {
        for tag_tree in tag_tree {
            if tail.len() == 0 {
                for note_id in &tag_tree.notes {
                    if let Some(note) = notes.get(&note_id.uuid) {
                        if &note.name == head {
                            return Some(NoteOrTag::Note(&note));
                        }
                    }
                }
            }
            if &tag_tree.name == head {
                if tail.len() == 0 {
                    return Some(NoteOrTag::Tag(&tag_tree));
                } else {
                    return get_by_full_name(&tag_tree.children, notes, tail.to_vec())
                }
            }
        }
    }
    None
}

pub fn get_tags_for_note(tag_tree: &Vec<TagTree>, note: &NoteId) -> Vec<TagId> {
    let mut out = vec![];
    for tag_tree in tag_tree {
        for try_note in &tag_tree.notes {
            if try_note == note {
                out.push(tag_tree.id);
            }
        }
        out.append(&mut get_tags_for_note(&tag_tree.children, note));
    }

    out
}

pub fn get_tag_name(tag_tree: &Vec<TagTree>, tag: &TagId) -> Option<Vec<String>> {
    for tag_tree in tag_tree {
        let mut out = vec![];
        out.push(tag_tree.name.clone());
        if &tag_tree.id == tag {
            return Some(out)
        }
        if let Some(mut name) = get_tag_name(&tag_tree.children, tag) {
            out.append(&mut name);
            return Some(out);
        }
    }
    return None
}

pub fn get_tag_by_full_name(tag_tree: &Vec<TagTree>, name: Vec<&str>) -> Option<TagId> {
    if let Some((head, tail)) = name.split_first() {
        for tag_tree in tag_tree {
            if &tag_tree.name == head {
                if tail.len() == 0 {
                    return Some(tag_tree.id);
                } else {
                    return get_tag_by_full_name(&tag_tree.children, tail.to_vec())
                }
            }
        }
    }
    None
}
