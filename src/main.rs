use structopt::StructOpt;
use std::path::PathBuf;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::str::FromStr;
use std::convert::TryInto;
use rusqlite::{params, Connection};
use uuid::Uuid;

#[derive(Debug)]
struct TagRow {
    id: Uuid,
    name: String,
    parent: Option<Uuid>,
}

// TODO: add notes to tagtree
#[derive(Debug)]
struct TagTree {
    id: Uuid,
    name: String,
    children: Vec<TagTree>,
}

impl From<&TagRow> for TagTree {
    fn from(tag_row: &TagRow) -> Self {
        TagTree {
            id: tag_row.id,
            name: tag_row.name.clone(),
            children: vec![],
        }
    }
}

fn unflatten_tags(tag_rows: &Vec<TagRow>, root_id: Option<Uuid>) -> Vec<TagTree> {
    let mut children = vec![];

    for tag_row in tag_rows {
        if tag_row.parent == root_id {
            children.push(TagTree {
                id: tag_row.id,
                name: tag_row.name.clone(),
                children: unflatten_tags(tag_rows, Some(tag_row.id))
            });        
        }
    }

    children
}

#[derive(Debug)]
enum MemexError {
    Sqlite(rusqlite::Error),
    Io(std::io::Error),
}

impl From<std::io::Error> for MemexError {
    fn from(err: std::io::Error) -> MemexError {
        MemexError::Io(err)
    }
}

impl From<rusqlite::Error> for MemexError {
    fn from(err: rusqlite::Error) -> MemexError {
        MemexError::Sqlite(err)
    }
}

fn init_memex(db: &Connection) -> Result<(), MemexError> {
    db.execute_batch(include_str!("setup.sql"))?;

    Ok(())
}

fn add_file_to_blobstore(filename: PathBuf) -> Result<blake3::Hash, MemexError> {
    let data = fs::read(filename).unwrap();
    let hash = blake3::hash(&data);
    let mut file = File::create(hash.to_hex().as_str())?;
    file.write_all(&data)?;
    Ok(hash)
}

fn add_note(db: &Connection, name: String, filename: PathBuf) -> Result<Uuid, MemexError> {
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

fn create_tag(db: &Connection, tag: Vec<String>) -> Result<(), MemexError> {
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

fn delete_tag(db: &Connection, tag: Vec<String>) -> Result<(), MemexError> {
    let id = get_tag_id_by_name(db, tag)?;
    db.execute("DELETE FROM Tags WHERE id = ?1", params![id])?;

    Ok(())
}

fn tag_note(db: &Connection, note: Uuid, tag: Vec<String>) -> Result<(), MemexError> {
    let tag_id = get_tag_id_by_name(db, tag)?;
    db.execute("INSERT INTO TagMap (note_id, tag_id) VALUES (?1, ?2)", params![note, tag_id])?;
    Ok(())
}

fn untag_note(db: &Connection, note: Uuid, tag: Vec<String>) -> Result<(), MemexError> {
    let tag_id = get_tag_id_by_name(db, tag)?;
    db.execute("DELETE FROM TagMap WHERE note_id = ?1 AND tag_id = ?2", params![note, tag_id])?;
    Ok(())
}

fn update_note(db: &Connection, note: Uuid, filename: PathBuf) -> Result<(), MemexError> {
    let hash = add_file_to_blobstore(filename)?.as_bytes().to_vec();
    db.execute("UPDATE Notes SET hash = ?1 WHERE id = ?2", params![hash, note])?;
    Ok(())
}

fn rename_note(db: &Connection, note: Uuid, name: String) -> Result<(), MemexError> {
    db.execute("UPDATE Notes SET name = ?1 WHERE id = ?2", params![name, note])?;
    Ok(())
}

fn get_tag_id_by_name(db: &Connection, tag: Vec<String>) -> Result<Uuid, MemexError> {
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

fn hash_from_str(s: String) -> Result<blake3::Hash, ()> {
    let hash_bytes = hex::decode(s).map_err(|_| ())?;
    let hash_array: [u8; blake3::OUT_LEN] = hash_bytes[..].try_into().map_err(|_| ())?;
    Ok(hash_array.into())
}

#[derive(Debug)]
struct TagList(Vec<String>);

fn parse_taglist(s: &str) -> TagList {
    let mut out = vec![];
    for tag_part in s.split("::") {
        out.push(tag_part.to_string());
    }
    TagList(out)
}

#[derive(StructOpt)]
enum Cmdline {
    Init {},
    AddNote {
        name: String,
        #[structopt(parse(from_os_str))]
        filename: PathBuf,
    },
    UpdateNote {
        note: Uuid,
        #[structopt(parse(from_os_str))]
        filename: PathBuf,
    },
    RenameNote {
        note: Uuid,
        name: String,
    },
    CreateTag {
        #[structopt(parse(from_str = parse_taglist))]
        tag: TagList,
    },
    DeleteTag {
        #[structopt(parse(from_str = parse_taglist))]
        tag: TagList,
    },
    TagNote {
        note: Uuid,
        #[structopt(parse(from_str = parse_taglist))]
        tag: TagList,
    },
    UntagNote {
        note: Uuid,
        #[structopt(parse(from_str = parse_taglist))]
        tag: TagList,
    },
}

fn main() -> Result<(), MemexError> {
    let cmdline = Cmdline::from_args();

    let db = Connection::open("index.db")?;

    match cmdline {
        Cmdline::Init {} => init_memex(&db)?,
        Cmdline::AddNote { name, filename } => { println!("{}", add_note(&db, name, filename)?); },
        Cmdline::UpdateNote { note, filename } => update_note(&db, note, filename)?,
        Cmdline::RenameNote { note, name } => rename_note(&db, note, name)?,
        Cmdline::CreateTag { tag } => create_tag(&db, tag.0)?,
        Cmdline::DeleteTag { tag } => delete_tag(&db, tag.0)?,
        Cmdline::TagNote { note, tag } => tag_note(&db, note, tag.0)?,
        Cmdline::UntagNote { note, tag } => untag_note(&db, note, tag.0)?,
    }

    /*
    let mut stmt = conn.prepare("SELECT id, name, parent FROM Tags")?;
    let tag_iter = stmt.query_map(params![], |row| {
        Ok(TagRow {
            id: row.get(0)?,
            name: row.get(1)?,
            parent: row.get(2)?,
        })
    })?;

    println!("{:#?}", unflatten_tags(&tag_iter.map(|x| x.unwrap()).collect(), None));
    */

    Ok(())
}
