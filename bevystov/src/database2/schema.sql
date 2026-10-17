CREATE TABLE tracks (
    id INTEGER PRIMARY KEY,
    title TEXT NOT NULL,
    rating INTEGER NOT NULL,
    duration INTEGER NOT NULL,
    path TEXT NOT NULL
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
    FOREIGN KEY (artist_id) REFERENCES artists (id)
        ON UPDATE CASCADE ON DELETE CASCADE
    FOREIGN KEY (track_id) REFERENCES tracks (id)
        ON UPDATE CASCADE ON DELETE CASCADE
);

CREATE TABLE album_tracks (
    album_id INTEGER NOT NULL,
    track_id INTEGER NOT NULL,
    PRIMARY KEY (album_id, track_id),
    FOREIGN KEY (album_id) REFERENCES albums (id)
        ON UPDATE CASCADE ON DELETE CASCADE
    FOREIGN KEY (track_id) REFERENCES tracks (id)
        ON UPDATE CASCADE ON DELETE CASCADE
);