use fuser::{
    FileAttr, FileType, Filesystem, MountOption, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry,
    Request,
};
use libc::ENOENT;
use crate::inode_map::{InodeMap, ROOT_INODE};
use libcommonplace_types::NoteOrTag;
use std::env;
use std::ffi::OsStr;
use std::time::{Duration, UNIX_EPOCH};
use std::convert::TryInto;

mod inode_map;

const TTL: Duration = Duration::from_secs(1);
const BLOCK_SIZE: u64 = 512;

fn file_attr(ino: u64, size: u64) -> FileAttr {
    FileAttr {
        ino,
        size,
        blocks: (size / BLOCK_SIZE) + 1,
        atime: UNIX_EPOCH,
        mtime: UNIX_EPOCH,
        ctime: UNIX_EPOCH,
        crtime: UNIX_EPOCH,
        kind: FileType::RegularFile,
        perm: 0o644,
        nlink: 1,
        uid: 0,
        gid: 0,
        rdev: 0,
        flags: 0,
        blksize: BLOCK_SIZE.try_into().unwrap(),
        padding: 0,
    }
}

fn dir_attr(ino: u64) -> FileAttr {
    FileAttr {
        ino,
        size: 0,
        blocks: 0,
        atime: UNIX_EPOCH,
        mtime: UNIX_EPOCH,
        ctime: UNIX_EPOCH,
        crtime: UNIX_EPOCH,
        kind: FileType::Directory,
        perm: 0o755,
        nlink: 2,
        uid: 0,
        gid: 0,
        rdev: 0,
        flags: 0,
        blksize: 512,
        padding: 0,
    }
}

struct FS {
    inode_map: InodeMap,
    db: libcommonplace::Connection,
}

impl FS {
    fn new() -> Self {
        Self {
            inode_map: InodeMap::new(),
            db: libcommonplace::open_db().unwrap(),
        }
    }
}

impl Filesystem for FS {
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        //println!("lookup(parent = {:?}, name = {:?})", parent, name);

        let tagtree = libcommonplace::get_tag_tree(&self.db).unwrap();
        let all_notes = libcommonplace::get_all_notes(&self.db).unwrap();

        let (children, notes) = if parent == ROOT_INODE {
            let untagged_notes = libcommonplace::get_untagged_notes(&self.db).unwrap();
            (tagtree, untagged_notes)
        } else {
            let uuid = match self.inode_map.get_entry(parent) {
                Some(entry) => entry.uuid,
                None => return reply.error(ENOENT),
            };


            if let Some(NoteOrTag::Tag(parent)) = libcommonplace_types::get_by_uuid(&tagtree, &all_notes, uuid) {
                (parent.children.clone(), parent.notes.iter().map(|x| x.uuid).collect())
            } else {
                return reply.error(ENOENT);
            }
        };

        for note_id in &notes {
            if let Some(note) = all_notes.get(&note_id) {
                if name.to_str() == Some(&note.name) {
                    return reply.entry(&TTL, &file_attr(self.inode_map.get_inode(inode_map::Entry { type_: inode_map::Type::NOTE, uuid: note.id }), libcommonplace::get_note_size(&self.db, note.id).unwrap_or(0)), 0);
                }
            }
        }

        for tag in &children {
            if name.to_str() == Some(&tag.name) {
                return reply.entry(&TTL, &dir_attr(self.inode_map.get_inode(inode_map::Entry { type_: inode_map::Type::TAG, uuid: tag.id.uuid })), 0);
                
            }
        }

        reply.error(ENOENT)
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        //println!("getattr({:?})", ino);

        let entry = self.inode_map.get_entry(ino);
        let valid_dir = ino == ROOT_INODE || entry.clone().map(|x| x.type_) == Some(inode_map::Type::TAG);

        if valid_dir {
            reply.attr(&TTL, &dir_attr(ino))
        } else {
            if let Some(entry) = entry {
                reply.attr(&TTL, &file_attr(ino, libcommonplace::get_note_size(&self.db, entry.uuid).unwrap_or(0)))
            } else {
                reply.error(ENOENT)
            }
        }
    }

    fn read(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock: Option<u64>,
        reply: ReplyData,
    ) {
        //println!("read(ino = {}, offset = {})", ino, offset);
        if let Some(entry) = self.inode_map.get_entry(ino) {
            if let Ok(contents) = libcommonplace::get_note_contents(&self.db, entry.uuid) {
                let start = offset as usize;
                let end = std::cmp::min(offset as usize + size as usize, contents.len());
                reply.data(&contents[start..end]);
            } else {
                reply.error(ENOENT);
            }
        } else {
            reply.error(ENOENT);
        }
    }

    fn readdir(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        //println!("readdir(ino = {}, offset = {})", ino, offset);

        let mut entries = vec![];

        let tagtree = libcommonplace::get_tag_tree(&self.db).unwrap();
        let notes = libcommonplace::get_all_notes(&self.db).unwrap();

        if ino == ROOT_INODE {
            for tag in tagtree {
                let inode = self.inode_map.get_inode(inode_map::Entry {
                    type_: inode_map::Type::TAG,
                    uuid: tag.id.uuid,
                });
                entries.push((inode, FileType::Directory, tag.name));
            }
            let untagged_notes = libcommonplace::get_untagged_notes(&self.db).unwrap();
            for note_id in untagged_notes {
                let note = notes.get(&note_id).unwrap();
                let inode = self.inode_map.get_inode(inode_map::Entry {
                    type_: inode_map::Type::NOTE,
                    uuid: note_id,
                });
                entries.push((inode, FileType::RegularFile, note.name.clone()));
            }
        } else {
            let entry = match self.inode_map.get_entry(ino) {
                Some(entry) => entry,
                None => return reply.error(ENOENT),
            };

            let details = libcommonplace_types::get_by_uuid(&tagtree, &notes, entry.uuid);

            match details {
                Some(NoteOrTag::Tag(tag)) => {
                    for tag in &tag.children {
                        let inode = self.inode_map.get_inode(inode_map::Entry {
                            type_: inode_map::Type::TAG,
                            uuid: tag.id.uuid,
                        });
                        entries.push((inode, FileType::Directory, tag.name.clone()));
                    }
                    for note_id in &tag.notes {
                        let note = notes.get(&note_id.uuid).unwrap();
                        let inode = self.inode_map.get_inode(inode_map::Entry {
                            type_: inode_map::Type::NOTE,
                            uuid: note_id.uuid,
                        });
                        entries.push((inode, FileType::RegularFile, note.name.clone()));
                    }
                }
                _ => return reply.error(ENOENT), // TODO better error for readdir on a file?
            };
        }

        for (i, entry) in entries.into_iter().enumerate().skip(offset as usize) {
            // i + 1 means the index of the next entry
            if reply.add(entry.0, (i + 1) as i64, entry.1, entry.2) {
                break;
            }
        }
        reply.ok();
    }
}

fn main() {
    let mountpoint = env::args_os().nth(1).unwrap();
    let options = vec![MountOption::RO, MountOption::AutoUnmount, MountOption::FSName("commonplace".to_string())];
    fuser::mount2(FS::new(), mountpoint, &options).unwrap();
}
