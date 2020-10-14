use std::collections::HashMap;
use std::path::PathBuf;
use std::fs;
use std::fs::File;
use std::io::Write;
use rusqlite::params;
use uuid::Uuid;
pub use libcommonplace_types::{TagRow, TagTree, Note};

pub use rusqlite::Connection;

#[derive(Debug)]
pub enum CommonplaceError {
    Sqlite(rusqlite::Error),
    Io(std::io::Error),
}

impl From<std::io::Error> for CommonplaceError {
    fn from(err: std::io::Error) -> CommonplaceError {
        CommonplaceError::Io(err)
    }
}

impl From<rusqlite::Error> for CommonplaceError {
    fn from(err: rusqlite::Error) -> CommonplaceError {
        CommonplaceError::Sqlite(err)
    }
}

fn get_tag_tree_internal(tag_rows: &Vec<TagRow>, tagmap_rows: &Vec<(Uuid, Uuid)>, root_id: Option<Uuid>) -> Vec<TagTree> {
    let mut children = vec![];

    for tag_row in tag_rows {
        if tag_row.parent == root_id {
            children.push(TagTree {
                id: tag_row.id,
                name: tag_row.name.clone(),
                children: get_tag_tree_internal(tag_rows, tagmap_rows, Some(tag_row.id)),
                notes: tagmap_rows.iter().filter_map(|x| {
                    if x.0 == tag_row.id {
                        Some(x.1.clone())
                    } else {
                        None
                    }
                }).collect(),
            });
        }
    }

    children
}

pub fn get_tag_tree(db: &Connection) -> Result<Vec<TagTree>, CommonplaceError> {
    let mut tag_query = db.prepare("SELECT id, name, parent FROM Tags")?;
    let tag_rows = tag_query.query_map(params![], |row| {
        Ok(TagRow {
            id: row.get("id")?,
            name: row.get("name")?,
            parent: row.get("parent")?,
        })
    })?.map(|x| x.unwrap()).collect();

    let mut tagmap_query = db.prepare("SELECT tag_id, note_id FROM TagMap")?;
    let tagmap_rows: Vec<(Uuid, Uuid)> = tagmap_query.query_map(params![], |row| {
        Ok((row.get("tag_id")?, row.get("note_id")?))
    })?.map(|x| x.unwrap()).collect();

    Ok(get_tag_tree_internal(&tag_rows, &tagmap_rows, None))
}


pub fn get_all_notes(db: &Connection) -> Result<HashMap<Uuid, Note>, CommonplaceError> {
    let mut notes_query = db.prepare("SELECT * FROM Notes")?;
    let notes_rows: Vec<Note> = notes_query.query_map(params![], |row| {
        let mut hash: [u8; 32] = [0; 32];
        hash.copy_from_slice(&row.get::<&str, Vec<u8>>("hash")?[..]);
        Ok(Note {
            id: row.get("id")?,
            hash,
            name: row.get("name")?,
            mimetype: row.get("mimetype")?,
        })
    })?.map(|x| x.unwrap()).collect();

    let mut res = HashMap::new();

    for note in notes_rows {
        res.insert(note.id, note);
    }

    Ok(res)
}

pub fn init_memex(db: &Connection) -> Result<(), CommonplaceError> {
    db.execute_batch(include_str!("setup.sql"))?;

    Ok(())
}

pub fn add_file_to_blobstore(filename: PathBuf) -> Result<blake3::Hash, CommonplaceError> {
    let data = fs::read(filename).unwrap();
    let hash = blake3::hash(&data);
    let mut file = File::create(hash.to_hex().as_str())?;
    file.write_all(&data)?;
    Ok(hash)
}

pub fn add_bytes_to_blobstore(contents: Vec<u8>) -> Result<blake3::Hash, CommonplaceError> {
    let hash = blake3::hash(&contents);
    let mut file = File::create(hash.to_hex().as_str())?;
    file.write_all(&contents)?;
    file.flush()?;
    file.sync_all()?;
    Ok(hash)
}

pub fn open_db() -> Result<Connection, CommonplaceError> {
    Ok(Connection::open("index.db")?)
}

pub fn add_note(db: &Connection, name: String, filename: PathBuf) -> Result<Uuid, CommonplaceError> {
    // TODO: check that file doesn't exist

    let id = Uuid::new_v4();
    let hash = add_file_to_blobstore(filename)?.as_bytes().to_vec();
    let mimetype = "application/octet-stream";

    db.execute(
        "INSERT INTO Notes (id, hash, name, mimetype) VALUES (?1, ?2, ?3, ?4)",
        params![id, hash, name, mimetype]
    )?;

    Ok(id)
}

pub fn create_tag(db: &Connection, tag: Vec<String>) -> Result<(), CommonplaceError> {
    let mut parent: Option<Uuid> = None;

    for tag_part in tag {
        match db.query_row(
            "SELECT * FROM Tags WHERE name = ?1 AND parent IS ?2",
            params![tag_part, parent],
            |row| row.get("id")
        ) {
            Ok(id) => parent = id,
            Err(_) => {
                db.execute("INSERT INTO Tags (id, name, parent) VALUES (?1, ?2, ?3)", params![Uuid::new_v4(), tag_part, parent])?;
                parent = db.query_row(
                    "SELECT * FROM Tags WHERE name = ?1 AND parent IS ?2",
                    params![tag_part, parent],
                    |row| row.get("id")
                )?;
            },
        }
    }

    Ok(())
}

pub fn delete_tag(db: &Connection, tag: Vec<String>) -> Result<(), CommonplaceError> {
    let id = get_tag_id_by_name(db, tag)?;
    db.execute("DELETE FROM Tags WHERE id = ?1", params![id])?;

    Ok(())
}

pub fn tag_note(db: &Connection, note: Uuid, tag: Vec<String>) -> Result<(), CommonplaceError> {
    let tag_id = get_tag_id_by_name(db, tag)?;
    db.execute("INSERT INTO TagMap (note_id, tag_id) VALUES (?1, ?2)", params![note, tag_id])?;
    Ok(())
}

pub fn untag_note(db: &Connection, note: Uuid, tag: Vec<String>) -> Result<(), CommonplaceError> {
    let tag_id = get_tag_id_by_name(db, tag)?;
    db.execute("DELETE FROM TagMap WHERE note_id = ?1 AND tag_id = ?2", params![note, tag_id])?;
    Ok(())
}

pub fn update_note(db: &Connection, note: Uuid, filename: PathBuf) -> Result<(), CommonplaceError> {
    let hash = add_file_to_blobstore(filename)?.as_bytes().to_vec();
    db.execute("UPDATE Notes SET hash = ?1 WHERE id = ?2", params![hash, note])?;
    Ok(())
}

pub fn update_note_bytes(db: &Connection, note: Uuid, contents: Vec<u8>) -> Result<(), CommonplaceError> {
    let hash = add_bytes_to_blobstore(contents)?.as_bytes().to_vec();
    db.execute("UPDATE Notes SET hash = ?1 WHERE id = ?2", params![hash, note])?;
    Ok(())
}

pub fn rename_note(db: &Connection, note: Uuid, name: String) -> Result<(), CommonplaceError> {
    db.execute("UPDATE Notes SET name = ?1 WHERE id = ?2", params![name, note])?;
    Ok(())
}

pub fn get_tag_id_by_name(db: &Connection, tag: Vec<String>) -> Result<Uuid, CommonplaceError> {
    let mut id: Option<Uuid> = None;

    for tag_part in tag {
        id = db.query_row(
            "SELECT * FROM Tags WHERE name = ?1 AND parent IS ?2",
            params![tag_part, id],
            |row| row.get("id")
        )?
    }

    Ok(id.unwrap())
}
