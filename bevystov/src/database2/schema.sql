CREATE TABLE tracks (
    id INTEGER PRIMARY KEY,
    title TEXT NOT NULL,
    rating INTEGER NOT NULL,
    duration INTEGER NOT NULL,
    bit_rate_kbps INTEGER NOT NULL,
    bit_depth INTEGER,
    sample_rate INTEGER,
    path TEXT NOT NULL,
    album_id INTEGER NOT NULL -- TODO: index
);

CREATE TABLE artists (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE albums (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE artist_tracks (
    artist_id INTEGER NOT NULL,
    track_id INTEGER NOT NULL,
    PRIMARY KEY (artist_id, track_id),
    FOREIGN KEY (artist_id) REFERENCES artists (id) ON UPDATE CASCADE ON DELETE CASCADE,
    FOREIGN KEY (track_id) REFERENCES tracks (id) ON UPDATE CASCADE ON DELETE CASCADE
);

-- Full-text search for tracks
CREATE VIRTUAL TABLE track_fts USING fts5(
    title,
    artist,
    album,
    tokenize = 'trigram'
);

-- Trigger for track deletions
CREATE TRIGGER tracks_ad AFTER DELETE ON tracks
BEGIN
    DELETE FROM track_fts WHERE rowid = old.id;
END;
