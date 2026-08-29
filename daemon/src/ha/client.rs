use std::collections::VecDeque;
use std::time::{Duration, Instant};

use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use crate::ha::registry::{AreaEntry, DeviceEntry, EntityEntry, Registry};
use crate::ha::url::Endpoint;

const COMMAND_TIMEOUT: Duration = Duration::from_secs(30);
// A large instance genuinely takes longer than a command to snapshot; sharing
// one budget produces a reconnect loop that never converges.
const SNAPSHOT_TIMEOUT: Duration = Duration::from_secs(90);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(20);

// A server that dribbles one frame every few seconds must not be able to hold
// the handshake open forever, and the frames it sends while we wait must not
// accumulate without limit.
const MAX_PENDING: usize = 4096;
const SEND_TIMEOUT: Duration = Duration::from_secs(15);

const MAX_FRAME: usize = 64 * 1024 * 1024;

type Stream = WebSocketStream<MaybeTlsStream<TcpStream>>;

// Without this, rustls refuses to pick a provider and panics inside the first
// handshake rather than returning an error we could report.
pub fn install_crypto_provider() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

#[derive(Debug)]
pub enum Error {
    Connect(String),
    Auth(String),
    Timeout(&'static str),
    Closed,
    Protocol(String),
    Server { code: String, message: String },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connect(m) => write!(f, "could not reach Home Assistant: {m}"),
            Self::Auth(m) => write!(f, "Home Assistant rejected the access token: {m}"),
            Self::Timeout(what) => write!(f, "Home Assistant did not answer in time ({what})"),
            Self::Closed => write!(f, "the connection closed"),
            Self::Protocol(m) => write!(f, "unexpected reply from Home Assistant: {m}"),
            Self::Server { code, message } => write!(f, "Home Assistant refused the request ({code}): {message}"),
        }
    }
}

impl Error {
    pub fn is_retryable(&self) -> bool {
        !matches!(self, Self::Auth(_))
    }
}

pub struct Snapshot {
    pub registry: Registry,
    pub states: Vec<Value>,
}

pub struct Session {
    stream: Stream,
    next_id: u64,
    pending: VecDeque<Value>,
    pub ha_version: String,
}

impl Session {
    pub async fn connect(endpoint: &Endpoint, token: &str) -> Result<Self, Error> {
        install_crypto_provider();
        let config = WebSocketConfig {
            max_message_size: Some(MAX_FRAME),
            max_frame_size: Some(MAX_FRAME),
            ..Default::default()
        };

        let connect = tokio_tungstenite::connect_async_tls_with_config(
            endpoint.websocket.as_str(),
            Some(config),
            false,
            // Omitting the connector makes tokio-tungstenite build the default
            // rustls one, which verifies against the system trust store. Passing
            // a Connector here is the only way to weaken that.
            None,
        );

        let (stream, _) = tokio::time::timeout(CONNECT_TIMEOUT, connect)
            .await
            .map_err(|_| Error::Timeout("connecting"))?
            .map_err(|e| Error::Connect(e.to_string()))?;

        let mut session = Self {
            stream,
            next_id: 1,
            pending: VecDeque::new(),
            ha_version: String::new(),
        };
        session.authenticate(token).await?;
        Ok(session)
    }

    async fn authenticate(&mut self, token: &str) -> Result<(), Error> {
        let hello = self.read_message(COMMAND_TIMEOUT, "waiting for the auth prompt").await?;
        match hello.get("type").and_then(Value::as_str) {
            Some("auth_required") => {}
            Some("auth_ok") => {
                self.ha_version = version_of(&hello);
                return Ok(());
            }
            other => {
                return Err(Error::Protocol(format!(
                    "expected auth_required, got {}",
                    other.unwrap_or("no type")
                )))
            }
        }

        self.send(json!({ "type": "auth", "access_token": token })).await?;
        let reply = self.read_message(COMMAND_TIMEOUT, "authenticating").await?;
        match reply.get("type").and_then(Value::as_str) {
            Some("auth_ok") => {
                self.ha_version = version_of(&reply);
                Ok(())
            }
            Some("auth_invalid") => Err(Error::Auth(
                reply
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("invalid token")
                    .to_string(),
            )),
            other => Err(Error::Protocol(format!(
                "expected auth_ok, got {}",
                other.unwrap_or("no type")
            ))),
        }
    }

    async fn send(&mut self, value: Value) -> Result<(), Error> {
        self.stream
            .send(Message::Text(value.to_string()))
            .await
            .map_err(|e| Error::Connect(e.to_string()))
    }

    async fn read_message(&mut self, budget: Duration, what: &'static str) -> Result<Value, Error> {
        // The budget covers the whole operation. Re-arming it per frame would
        // let a trickle of pings keep this alive indefinitely.
        let deadline = Instant::now() + budget;
        loop {
            let remaining = deadline
                .checked_duration_since(Instant::now())
                .ok_or(Error::Timeout(what))?;
            let next = tokio::time::timeout(remaining, self.stream.next())
                .await
                .map_err(|_| Error::Timeout(what))?;
            match next {
                Some(Ok(Message::Text(text))) => {
                    return serde_json::from_str(&text)
                        .map_err(|e| Error::Protocol(format!("malformed JSON: {e}")))
                }
                Some(Ok(Message::Ping(_) | Message::Pong(_) | Message::Binary(_) | Message::Frame(_))) => continue,
                Some(Ok(Message::Close(_))) | None => return Err(Error::Closed),
                Some(Err(e)) => return Err(Error::Connect(e.to_string())),
            }
        }
    }

    async fn request(&mut self, mut command: Value, budget: Duration, what: &'static str) -> Result<Value, Error> {
        let id = self.next_id;
        self.next_id += 1;
        command["id"] = json!(id);
        self.send(command).await?;

        loop {
            let message = self.read_message(budget, what).await?;
            let same_id = message.get("id").and_then(Value::as_u64) == Some(id);
            let is_result = message.get("type").and_then(Value::as_str) == Some("result");
            if !(same_id && is_result) {
                if self.pending.len() >= MAX_PENDING {
                    return Err(Error::Protocol(
                        "Home Assistant sent more unsolicited data than we can hold".into(),
                    ));
                }
                self.pending.push_back(message);
                continue;
            }
            if message.get("success").and_then(Value::as_bool) == Some(true) {
                return Ok(message.get("result").cloned().unwrap_or(Value::Null));
            }
            let error = message.get("error").cloned().unwrap_or(Value::Null);
            return Err(Error::Server {
                code: error
                    .get("code")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
                message: error
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("no message")
                    .to_string(),
            });
        }
    }

    pub async fn snapshot(&mut self) -> Result<Snapshot, Error> {
        let areas = self.list("config/area_registry/list").await?;
        let devices = self.list("config/device_registry/list").await?;
        let entities = self.list("config/entity_registry/list").await?;

        let states = self
            .request(json!({ "type": "get_states" }), SNAPSHOT_TIMEOUT, "fetching states")
            .await?;
        let states = states.as_array().cloned().unwrap_or_default();

        Ok(Snapshot {
            registry: Registry {
                areas: decode::<AreaEntry>(areas, "areas"),
                devices: decode::<DeviceEntry>(devices, "devices"),
                entities: decode::<EntityEntry>(entities, "entities"),
            },
            states,
        })
    }

    async fn list(&mut self, kind: &'static str) -> Result<Value, Error> {
        self.request(json!({ "type": kind }), SNAPSHOT_TIMEOUT, kind).await
    }


    pub async fn lovelace_config(&mut self) -> Option<Value> {
        self.request(
            json!({ "type": "lovelace/config", "url_path": Value::Null }),
            COMMAND_TIMEOUT,
            "reading the dashboard",
        )
        .await
        .ok()
    }

    pub async fn subscribe(&mut self, event_type: &str) -> Result<(), Error> {
        self.request(
            json!({ "type": "subscribe_events", "event_type": event_type }),
            COMMAND_TIMEOUT,
            "subscribing",
        )
        .await
        .map(|_| ())
    }

    pub async fn call_service(
        &mut self,
        domain: &str,
        service: &str,
        entity_id: &str,
        data: Value,
    ) -> Result<(), Error> {
        let mut service_data = match data {
            Value::Object(map) => map,
            _ => serde_json::Map::new(),
        };
        service_data.remove("entity_id");
        self.request(
            json!({
                "type": "call_service",
                "domain": domain,
                "service": service,
                "service_data": Value::Object(service_data),
                "target": { "entity_id": entity_id },
            }),
            COMMAND_TIMEOUT,
            "calling a service",
        )
        .await
        .map(|_| ())
    }

}

fn version_of(message: &Value) -> String {
    message
        .get("ha_version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn decode<T: serde::de::DeserializeOwned>(value: Value, what: &str) -> Vec<T> {
    let Some(items) = value.as_array() else {
        return Vec::new();
    };
    let mut out = Vec::with_capacity(items.len());
    let mut skipped = 0usize;
    for item in items {
        match serde_json::from_value(item.clone()) {
            Ok(entry) => out.push(entry),
            Err(_) => skipped += 1,
        }
    }
    if skipped > 0 {
        eprintln!("atriumd: skipped {skipped} unreadable {what} entries");
    }
    out
}

pub struct EventReader {
    stream: SplitStream<Stream>,
    pending: VecDeque<Value>,
}

pub struct CommandWriter {
    sink: SplitSink<Stream, Message>,
    next_id: u64,
}

impl Session {
    // Reading and writing must be separate once events stream: one `&mut Session`
    // cannot be borrowed by both a next_event() and an outgoing command in the
    // same select!.
    pub fn into_parts(self) -> (CommandWriter, EventReader) {
        let Self { stream, next_id, pending, .. } = self;
        let (sink, stream) = stream.split();
        (
            CommandWriter { sink, next_id },
            EventReader { stream, pending },
        )
    }
}

impl EventReader {
    pub async fn next_event(&mut self) -> Result<Value, Error> {
        if let Some(queued) = self.pending.pop_front() {
            return Ok(queued);
        }
        loop {
            match self.stream.next().await {
                Some(Ok(Message::Text(text))) => {
                    return serde_json::from_str(&text)
                        .map_err(|e| Error::Protocol(format!("malformed JSON: {e}")))
                }
                Some(Ok(Message::Ping(_) | Message::Pong(_) | Message::Binary(_) | Message::Frame(_))) => continue,
                Some(Ok(Message::Close(_))) | None => return Err(Error::Closed),
                Some(Err(e)) => return Err(Error::Connect(e.to_string())),
            }
        }
    }
}

impl CommandWriter {
    pub async fn ping(&mut self) -> Result<(), Error> {
        let id = self.next_id;
        self.next_id += 1;
        self.send(json!({ "id": id, "type": "ping" })).await
    }

    async fn send(&mut self, value: Value) -> Result<(), Error> {
        tokio::time::timeout(SEND_TIMEOUT, self.sink.send(Message::Text(value.to_string())))
            .await
            .map_err(|_| Error::Timeout("sending"))?
            .map_err(|e| Error::Connect(e.to_string()))
    }

    pub async fn call_service(
        &mut self,
        domain: &str,
        service: &str,
        entity_id: &str,
        data: Value,
    ) -> Result<u64, Error> {
        let mut service_data = match data {
            Value::Object(map) => map,
            _ => serde_json::Map::new(),
        };
        service_data.remove("entity_id");

        let id = self.next_id;
        self.next_id += 1;
        let message = json!({
            "id": id,
            "type": "call_service",
            "domain": domain,
            "service": service,
            "service_data": Value::Object(service_data),
            "target": { "entity_id": entity_id },
        });
        self.send(message).await?;
        Ok(id)
    }

    pub async fn close(&mut self) {
        let _ = self.sink.close().await;
    }
}
