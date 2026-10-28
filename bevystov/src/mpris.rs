use zbus::{
    blocking::Connection,
    interface,
    names::{InterfaceName, WellKnownName},
    zvariant::{Array, ObjectPath, OwnedValue, Str},
};

use crate::database::{Track, TrackId};

type MprisDict = std::collections::HashMap<String, OwnedValue>;
type CommandsSender = std::sync::mpsc::Sender<MprisCommand>;
type CommandsReceiver = std::sync::mpsc::Receiver<MprisCommand>;
type EventsSender = std::sync::mpsc::Sender<MprisEvent>;
type EventsReceiver = std::sync::mpsc::Receiver<MprisEvent>;

pub struct MediaControls {
    commands: CommandsReceiver,
    events: EventsSender,
}

impl MediaControls {
    pub fn spawn() -> std::io::Result<Self> {
        let (command_tx, command_rx) = std::sync::mpsc::channel::<MprisCommand>();
        let (event_tx, event_rx) = std::sync::mpsc::channel::<MprisEvent>();

        std::thread::Builder::new()
            .name("mpris".into())
            .spawn(move || {
                if let Err(error) = Self::run(command_tx, event_rx) {
                    eprintln!("MPRIS error: {error}");
                }
            })?;

        Ok(Self {
            commands: command_rx,
            events: event_tx,
        })
    }

    pub fn recv(&self) -> Option<MprisCommand> {
        self.commands.try_recv().ok()
    }

    pub fn send(&self, ev: MprisEvent) {
        let _ = self.events.send(ev);
    }

    fn run(commands: CommandsSender, events: EventsReceiver) -> zbus::Result<()> {
        const BUS_NAME: WellKnownName =
            WellKnownName::from_static_str_unchecked("org.mpris.MediaPlayer2.bevystov");
        const BUS_PROPERTIES_IFACE: InterfaceName =
            InterfaceName::from_static_str_unchecked("org.freedesktop.DBus.Properties");
        const MPRIS_OBJECT_PATH: ObjectPath =
            ObjectPath::from_static_str_unchecked("/org/mpris/MediaPlayer2");
        const MPRIS_PLAYER_IFACE: InterfaceName =
            InterfaceName::from_static_str_unchecked("org.mpris.MediaPlayer2.Player");

        // TODO: What if name already exists? Currently it replaces it.

        let connection = Connection::session()?;
        connection.request_name(BUS_NAME)?;

        connection
            .object_server()
            .at(MPRIS_OBJECT_PATH, MprisRoot)?;

        let mpris_player = MprisPlayer {
            commands,
            state: MprisState {
                status: MprisStatus::Stopped,
                metadata: None,
            },
        };

        connection
            .object_server()
            .at(MPRIS_OBJECT_PATH, mpris_player)?;

        let player_iface = connection
            .object_server()
            .interface::<_, MprisPlayer>(MPRIS_OBJECT_PATH)?;

        for event in events {
            let changed = {
                let mut player = player_iface.get_mut();
                match event {
                    MprisEvent::Status(status) => {
                        player.state.status = status;
                    }
                    MprisEvent::Metadata(metadata) => {
                        player.state.metadata = Some(metadata);
                    }
                    MprisEvent::Both(status, metadata) => {
                        player.state.status = status;
                        player.state.metadata = metadata;
                    }
                }
                player.state.as_dict()
            };

            // org.freedesktop.DBus.Properties.PropertiesChanged
            //
            //   s       interface name
            //   a{sv}   changed properties
            //   as      invalidated properties
            //
            let body = &(MPRIS_PLAYER_IFACE, changed, Vec::<&str>::new());

            connection.emit_signal(
                None::<&str>,
                MPRIS_OBJECT_PATH,
                BUS_PROPERTIES_IFACE,
                "PropertiesChanged",
                body,
            )?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MprisCommand {
    Play,
    Pause,
    PlayPause,
    Stop,
    Next,
    Previous,
}

#[derive(Debug)]
pub enum MprisEvent {
    Status(MprisStatus),
    Metadata(MprisMetadata),
    Both(MprisStatus, Option<MprisMetadata>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MprisStatus {
    Playing,
    Paused,
    Stopped,
}

impl MprisStatus {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Playing => "Playing",
            Self::Paused => "Paused",
            Self::Stopped => "Stopped",
        }
    }

    fn as_dict(self) -> MprisDict {
        MprisDict::from([(
            String::from("PlaybackStatus"),
            OwnedValue::from(Str::from(self.as_str())),
        )])
    }
}

#[derive(Debug)]
pub struct MprisMetadata {
    id: String,
    title: String,
    artist: String,
    album: String,
}

impl MprisMetadata {
    pub fn new(id: TrackId, track: &Track) -> Self {
        Self {
            id: format!("/org/mpris/MediaPlayer2/Track/{}", id.0),
            title: String::from(track.title()),
            artist: String::from(track.artist()),
            album: String::from(track.album()),
        }
    }

    fn as_dict(&self) -> MprisDict {
        MprisDict::from([
            (
                String::from("mpris:trackid"),
                OwnedValue::from(ObjectPath::from_str_unchecked(self.id.as_str())),
            ),
            (
                String::from("xesam:title"),
                OwnedValue::from(Str::from(self.title.as_str())),
            ),
            (
                String::from("xesam:artist"),
                OwnedValue::try_from(Array::from(vec![self.artist.as_str()])).unwrap(),
            ),
            (
                String::from("xesam:album"),
                OwnedValue::from(Str::from(self.album.as_str())),
            ),
        ])
    }

    fn empty_dict() -> MprisDict {
        MprisDict::from([
            (String::from("xesam:title"), OwnedValue::from(Str::from(""))),
            (
                String::from("xesam:artist"),
                OwnedValue::try_from(Array::from(Vec::<&str>::new())).unwrap(),
            ),
            (String::from("xesam:album"), OwnedValue::from(Str::from(""))),
        ])
    }
}

struct MprisRoot;

#[interface(name = "org.mpris.MediaPlayer2")]
impl MprisRoot {
    #[zbus(property)]
    fn can_quit(&self) -> bool {
        false
    }

    #[zbus(property)]
    fn can_raise(&self) -> bool {
        false
    }

    #[zbus(property)]
    fn has_track_list(&self) -> bool {
        false
    }

    #[zbus(property)]
    fn identity(&self) -> &str {
        "bevystov"
    }

    #[zbus(property)]
    fn desktop_entry(&self) -> &str {
        "bevystov"
    }

    #[zbus(property)]
    fn supported_uri_schemes(&self) -> &[&str] {
        &[]
    }

    #[zbus(property)]
    fn supported_mime_types(&self) -> &[&str] {
        &["audio/mpeg", "audio/flac", "audio/ogg"]
    }
}

struct MprisPlayer {
    commands: CommandsSender,
    state: MprisState,
}

impl MprisPlayer {
    fn send(&self, command: MprisCommand) -> zbus::fdo::Result<()> {
        self.commands
            .send(command)
            .map_err(|_| zbus::fdo::Error::Failed(String::from("MPRIS Player thread has stopped")))
    }
}

#[derive(Debug)]
struct MprisState {
    status: MprisStatus,
    metadata: Option<MprisMetadata>,
}

impl MprisState {
    fn as_dict(&self) -> MprisDict {
        let mut map = self.status.as_dict();

        let metadata = match self.metadata.as_ref() {
            Some(metadata) => metadata.as_dict(),
            None => MprisMetadata::empty_dict(),
        };
        map.insert(String::from("Metadata"), OwnedValue::from(metadata));

        map
    }
}

#[interface(name = "org.mpris.MediaPlayer2.Player")]
impl MprisPlayer {
    fn play(&self) -> zbus::fdo::Result<()> {
        self.send(MprisCommand::Play)
    }

    fn pause(&self) -> zbus::fdo::Result<()> {
        self.send(MprisCommand::Pause)
    }

    fn play_pause(&self) -> zbus::fdo::Result<()> {
        self.send(MprisCommand::PlayPause)
    }

    fn stop(&self) -> zbus::fdo::Result<()> {
        self.send(MprisCommand::Stop)
    }

    fn next(&self) -> zbus::fdo::Result<()> {
        self.send(MprisCommand::Next)
    }

    fn previous(&self) -> zbus::fdo::Result<()> {
        self.send(MprisCommand::Previous)
    }

    #[zbus(property)]
    fn playback_status(&self) -> &str {
        self.state.status.as_str()
    }

    #[zbus(property)]
    fn metadata(&self) -> MprisDict {
        self.state.as_dict()
    }

    #[zbus(property)]
    fn can_play(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_pause(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_go_next(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_go_previous(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_seek(&self) -> bool {
        false
    }

    #[zbus(property)]
    fn can_control(&self) -> bool {
        true
    }
}
