use structopt::StructOpt;
use std::path::PathBuf;
use uuid::Uuid;
use walkdir::WalkDir;

use libcommonplace::{open_db, init_memex, add_note, update_note, rename_note, create_tag, delete_tag, tag_note, untag_note, get_tag_tree, CommonplaceError, Connection};

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
    Init {
        #[structopt(parse(from_os_str))]
        directory: Option<PathBuf>,
    },
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

fn import_directory(db: &Connection, directory: PathBuf) {
    std::env::set_current_dir(directory).unwrap();
    for entry in WalkDir::new(".") {
        let entry = entry.unwrap();
        let path = entry.path();
        let mut components = path.components().filter_map(|x| {
            match x {
                std::path::Component::Normal(c) => Some(String::from(c.to_str().unwrap())),
                _ => None,
            }
        }).collect::<Vec<String>>();

        if components.len() > 0 && !components.first().unwrap().starts_with(".") {
            if path.is_dir() {
                create_tag(db, components).unwrap();
            } else {
                let name = components.pop().unwrap();
                let note = add_note(db, name, path.to_path_buf()).unwrap();
                if components.len() > 0 {
                    tag_note(db, note, components).unwrap();
                }
            }
        }
    }
}

fn main() -> Result<(), CommonplaceError> {
    let cmdline = Cmdline::from_args();

    let db = open_db()?;

    match cmdline {
        Cmdline::Init { directory } => {
            init_memex(&db)?;
            if let Some(directory) = directory {
                import_directory(&db, directory);
            }
        },
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
