use bimap::BiMap;
use uuid::Uuid;

pub const ROOT_INODE: u64 = 1;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Type {
    TAG,
    NOTE,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Entry {
    pub type_: Type,
    pub uuid: Uuid,
}

pub struct InodeMap {
    map: BiMap<u64, Entry>,
    next: u64,
}

impl InodeMap {
    pub fn new() -> Self {
        InodeMap {
            map: BiMap::new(),
            next: ROOT_INODE + 1,
        }
    }

    pub fn get_inode(&mut self, entry: Entry) -> u64 {
        match self.map.get_by_right(&entry) {
            Some(inode) => *inode,
            None => {
                self.next += 1;
                self.map.insert(self.next, entry);
                self.next
            }
        }
    }

    pub fn get_entry(&self, inode: u64) -> Option<Entry> {
        self.map.get_by_left(&inode).cloned()
    }
}
