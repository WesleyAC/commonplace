CREATE TABLE Tags(
	id BLOB PRIMARY KEY NOT NULL, -- UUID4
	name TEXT NOT NULL,
	parent BLOB DEFAULT NULL,
	FOREIGN KEY(parent) REFERENCES Tags(id) ON DELETE CASCADE,
	UNIQUE (name, parent)
);

-- Note that you can have multiple notes with the same name and the same hash.
-- This is by design - since the notes can have a different set of tags, it
-- seems useful to allow this. This, of course, necessitates a separate ID to
-- keep track of each note object.

-- The current assumption is that notes will not usually be deleted, they will
-- instead be tagged with an "archive" tag. If a note is deleted, its history
-- will be deleted along with it, and it will be as if it never existed.
CREATE TABLE Notes(
	id BLOB PRIMARY KEY NOT NULL, -- UUID4
	hash BLOB NOT NULL, -- blake3
	name TEXT NOT NULL,
	mimetype TEXT NOT NULL
);

CREATE TABLE Blobs(
  hash BLOB PRIMARY KEY NOT NULL, -- blake3
  contents BLOB NOT NULL
);

CREATE TABLE TagMap(
	note_id BLOB NOT NULL,
	tag_id BLOB NOT NULL,
	PRIMARY KEY(note_id, tag_id),
	FOREIGN KEY(note_id) REFERENCES Notes(id),
	FOREIGN KEY(tag_id) REFERENCES Tags(id)
);

-- Since we don't usually delete notes (see above comment), we only need to
-- track updates. Current state is in Notes, past state is in NoteHistory, and
-- on deletion everything goes away.
CREATE TRIGGER note_update_history 
AFTER UPDATE ON Notes
BEGIN
	INSERT INTO NoteHistory (note_id, hash, name, mimetype, time)
	VALUES (
		old.id,
		old.hash,
		old.name,
		old.mimetype,
		strftime('%s', 'now')
	);
END;

CREATE TABLE NoteHistory(
	note_id BLOB NOT NULL, -- UUID4
	hash BLOB NOT NULL, -- blake3
	name TEXT NOT NULL,
	mimetype TEXT NOT NULL,
	time INTEGER NOT NULL, -- UTC epoch time
	FOREIGN KEY(note_id) REFERENCES Notes(id) ON DELETE CASCADE
);
