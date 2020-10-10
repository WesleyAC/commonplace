use structopt::StructOpt;
use std::path::PathBuf;
use uuid::Uuid;
use libcommonplace::{open_db, init_memex, add_note, update_note, rename_note, create_tag, delete_tag, tag_note, untag_note, get_tag_tree, MemexError};

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
    ShowTree {},
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

    let db = open_db()?;

    match cmdline {
        Cmdline::Init {} => init_memex(&db)?,
        Cmdline::ShowTree {} => { for tree in get_tag_tree(&db)? { println!("{}", tree); } },
        Cmdline::AddNote { name, filename } => { println!("{}", add_note(&db, name, filename)?); },
        Cmdline::UpdateNote { note, filename } => update_note(&db, note, filename)?,
        Cmdline::RenameNote { note, name } => rename_note(&db, note, name)?,
        Cmdline::CreateTag { tag } => create_tag(&db, tag.0)?,
        Cmdline::DeleteTag { tag } => delete_tag(&db, tag.0)?,
        Cmdline::TagNote { note, tag } => tag_note(&db, note, tag.0)?,
        Cmdline::UntagNote { note, tag } => untag_note(&db, note, tag.0)?,
    }

    Ok(())
}
