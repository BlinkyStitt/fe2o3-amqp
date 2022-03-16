//! Implements AMQP1.0 Connection

use std::{cmp::min, collections::BTreeMap, convert::TryInto, io};

use async_trait::async_trait;

use fe2o3_amqp_types::{
    definitions::{self, AmqpError},
    performatives::{Begin, Close, End, Open},
    states::ConnectionState,
};
use futures_util::{Sink, SinkExt};
use slab::Slab;
use tokio::{
    sync::{mpsc::Sender, oneshot},
    task::JoinHandle,
};
use tracing::{instrument, trace};
use url::Url;

use crate::{
    control::ConnectionControl,
    endpoint,
    frames::amqp::{Frame, FrameBody},
    session::frame::{SessionFrame, SessionFrameBody, SessionIncomingItem},
    session::Session,
};

use self::{builder::WithoutContainerId, engine::SessionId};

mod builder;
pub use builder::*;

pub(crate) mod engine;

mod error;
pub mod heartbeat;
pub use error::*;

/// Default max-frame-size
pub const DEFAULT_MAX_FRAME_SIZE: u32 = 256 * 1024;

/// Default channel-max
pub const DEFAULT_CHANNEL_MAX: u16 = 255;

/// A handle to the [`Connection`] event loop.
///
/// Dropping the handle will also stop the [`Connection`] event loop
#[derive(Debug)]
pub struct ConnectionHandle {
    pub(crate) control: Sender<ConnectionControl>,
    handle: JoinHandle<Result<(), Error>>,

    // outgoing channel for session
    pub(crate) outgoing: Sender<SessionFrame>,
}

impl Drop for ConnectionHandle {
    fn drop(&mut self) {
        let _ = self.control.try_send(ConnectionControl::Close(None));
    }
}

impl ConnectionHandle {
    /// Checks if the underlying event loop has stopped
    pub fn is_closed(&self) -> bool {
        self.control.is_closed()
    }

    /// Close the connection
    ///
    /// # Panics
    ///
    /// Panics if this is called after executing any of [`close`](#method.close),
    /// [`close_with_error`](#method.close_with_error) or [`on_close`](#method.on_close).
    /// This will cause the JoinHandle to be polled after completion, which causes a panic.
    pub async fn close(&mut self) -> Result<(), Error> {
        // If sending is unsuccessful, the `ConnectionEngine` event loop is
        // already dropped, this should be reflected by `JoinError` then.
        let _ = self.control.send(ConnectionControl::Close(None)).await;
        self.on_close().await
    }

    /// Close the connection with an error
    ///
    /// # Panics
    ///
    /// Panics if this is called after executing any of [`close`](#method.close),
    /// [`close_with_error`](#method.close_with_error) or [`on_close`](#method.on_close).
    /// This will cause the JoinHandle to be polled after completion, which causes a panic.
    pub async fn close_with_error(
        &mut self,
        error: impl Into<definitions::Error>,
    ) -> Result<(), Error> {
        // If sending is unsuccessful, the `ConnectionEngine` event loop is
        // already dropped, this should be reflected by `JoinError` then.
        let _ = self
            .control
            .send(ConnectionControl::Close(Some(error.into())))
            .await;
        self.on_close().await
    }

    /// Returns when the underlying event loop has stopped
    ///
    /// # Panics
    ///
    /// Panics if this is called after executing any of [`close`](#method.close),
    /// [`close_with_error`](#method.close_with_error) or [`on_close`](#method.on_close).
    /// This will cause the JoinHandle to be polled after completion, which causes a panic.
    pub async fn on_close(&mut self) -> Result<(), Error> {
        match (&mut self.handle).await {
            Ok(res) => res,
            Err(e) => Err(Error::JoinError(e)),
        }
    }

    /// Allocte (channel, session_id) for a new session
    pub(crate) async fn allocate_session(
        &mut self,
        tx: Sender<SessionIncomingItem>,
    ) -> Result<(u16, SessionId), AllocSessionError> {
        let (responder, resp_rx) = oneshot::channel();
        self.control
            .send(ConnectionControl::AllocateSession { tx, responder })
            .await?; // std::io::Error
        let result = resp_rx.await.map_err(|_| {
            AllocSessionError::Io(
                // The sending half is already dropped
                io::Error::new(
                    io::ErrorKind::Other,
                    "ConnectionEngine event_loop is dropped",
                ),
            )
        })?;
        result
    }
}

/// An AMQP 1.0 Connection.
///
/// # Open a new [`Connection`] with default configuration
///
/// Below is an example with a local broker (
/// [`TestAmqpBroker`](https://github.com/Azure/amqpnetlite/releases/download/test_broker.1609/TestAmqpBroker.zip))
/// listening on the localhost. The broker is executed with the following command
///
/// ```powershell
/// ./TestAmqpBroker.exe amqp://localhost:5672 /creds:guest:guest /queues:q1
/// ```
///
/// ```rust, ignore
/// let connection = Connection::open(
///     "connection-1", // container id
///     "amqp://guest:guest@localhost:5672" // url with username and password
/// ).await.unwrap();
/// ```
///
/// ## Default configuration
///
/// | Field | Default Value |
/// |-------|---------------|
/// |`max_frame_size`| [`DEFAULT_MAX_FRAME_SIZE`] |
/// |`channel_max`| [`DEFAULT_CHANNEL_MAX`] |
/// |`idle_time_out`| `None` |
/// |`outgoing_locales`| `None` |
/// |`incoming_locales`| `None` |
/// |`offered_capabilities`| `None` |
/// |`desired_capabilities`| `None` |
/// |`Properties`| `None` |
/// 
/// ## Customize configuration with [`Builder`]
///
/// The example above creates a connection with the default configuration. If the user needs to customize the
/// configuration, the connection [`Builder`] should be used.
///
/// ```rust, ignore
/// let connection = Connection::builder()
///     .container_id("connection-1")
///     .max_frame_size(4096)
///     .channel_max(64)
///     .idle_time_out(50_000 as u32)
///     .open("amqp://guest:guest@localhost:5672")
///     .await.unwrap();
/// ```
///
/// ## TLS
///
/// TLS is supported with `rustls` by setting the url scheme to `"amqps"`. 
/// The user must use the builder to supply a custom `ClientConfig` for the TLS connection. 
/// If there is no custom `ClientConfig`, the TLS handshake will use the the following
/// config.
/// 
/// ```rust, ignore
/// ClientConfig::builder()
///     .with_safe_defaults()
///     .with_root_certificates(RootCertStore::empty())
///     .with_no_client_auth()
/// ```
/// 
/// The code example below shows TLS connection without supplying a custom `ClientConfig`
/// 
/// ```rust, ignore
/// let connection = Connection::open("connection-1", "amqps://localhost:5672").await.unwrap();
/// ```
/// 
/// The code example below shows how a user can supply a custom `ClientConfig` using builder.
///
/// ```rust, ignore
/// // Set custom `ClientConfig`
/// let connection = Connection::builder()
///     .container_id("connection-1")
///     .client_config(config)
///     .open("amqps://localhost:5672")
///     .await.unwrap();
/// ```
///
/// ## SASL
///
/// If `username` and `password` are supplied with the url, the connection negotiation will start with
/// SASL PLAIN negotiation. Other than filling `username` and `password` in the url, one could also
/// supply the information with `sasl_profile` field of the [`Builder`]. Please note that the SASL profile
/// found in the url will override whatever `SaslProfile` supplied to the [`Builder`].
///
/// The examples below shows two ways of starting the connection with SASL negotiation.
/// 
/// 1. Start SASL negotiation with SASL PLAIN profile extracted from the url
///
///     ```rust,ignore
///     let connection = Connection::open("connection-1", "amqp://guest:guest@localhost:5672").await.unwrap();
///     ```
/// 
/// 2. Start SASL negotiation with the builder. Please note that tf the url contains `username` and `password`, 
/// the profile supplied to the builder will be overriden.
/// 
///     ```
///     // This is equivalent to the line above
///     let profile = SaslProfile::Plain {
///         username: "guest".to_string(),
///         password: "guest".to_string()
///     };
///     let connection = Connection::builder()
///         .container_id("connection-1")
///         .sasl_profile(profile)
///         .open("amqp://localhost:5672")
///         .await.unwrap();
///     ```
///
#[derive(Debug)]
pub struct Connection {
    control: Sender<ConnectionControl>,

    // local
    local_state: ConnectionState,
    local_open: Open,
    local_sessions: Slab<Sender<SessionIncomingItem>>,
    session_by_incoming_channel: BTreeMap<u16, usize>,
    session_by_outgoing_channel: BTreeMap<u16, usize>,

    // remote
    remote_open: Option<Open>,

    // mutually agreed channel max
    agreed_channel_max: u16,
}

/* ------------------------------- Public API ------------------------------- */
impl Connection {
    /// Creates a Builder for [`Connection`]
    pub fn builder<'a>() -> builder::Builder<'a, WithoutContainerId, ()> {
        builder::Builder::new()
    }

    /// Negotiate and open a [`Connection`] with the default configuration
    ///
    /// # Default configuration
    ///
    /// | Field | Default Value |
    /// |-------|---------------|
    /// |`max_frame_size`| [`DEFAULT_MAX_FRAME_SIZE`] |
    /// |`channel_max`| [`DEFAULT_CHANNEL_MAX`] |
    /// |`idle_time_out`| `None` |
    /// |`outgoing_locales`| `None` |
    /// |`incoming_locales`| `None` |
    /// |`offered_capabilities`| `None` |
    /// |`desired_capabilities`| `None` |
    /// |`Properties`| `None` |
    ///
    /// The negotiation depends on the url supplied.
    ///
    /// # Raw AMQP
    ///
    /// ```rust, ignore
    /// let connection = Connection::open("connection-1", "amqp://localhost:5672").await.unwrap();
    /// ```
    /// 
    /// # TLS
    /// 
    /// This will start TLS negotiation with the following `ClientConfig`
    /// 
    /// ```rust, ignore
    /// ClientConfig::builder()
    ///     .with_safe_defaults()
    ///     .with_root_certificates(RootCertStore::empty())
    ///     .with_no_client_auth()
    /// ```
    /// 
    /// ```rust, ignore
    /// let connection = Connection::open("connection-1", "amqps://localhost:5672").await.unwrap();
    /// ```
    ///
    /// # SASL
    ///
    /// Start SASL negotiation with SASL PLAIN profile extracted from the url
    /// 
    /// ```rust, ignore
    /// let connection = Connection::open("connection-1", "amqp://guest:guest@localhost:5672").await.unwrap();
    /// ```
    ///
    pub async fn open(
        container_id: impl Into<String>, // TODO: default container id? random uuid-ish
        url: impl TryInto<Url, Error = url::ParseError>,
    ) -> Result<ConnectionHandle, OpenError> {
        Connection::builder()
            .container_id(container_id)
            .open(url)
            .await
    }
}

/* ------------------------------- Private API ------------------------------ */
impl Connection {
    fn new(
        control: Sender<ConnectionControl>,
        local_state: ConnectionState,
        local_open: Open,
    ) -> Self {
        let agreed_channel_max = local_open.channel_max.0;
        Self {
            control,
            local_state,
            local_open,
            local_sessions: Slab::new(),
            session_by_incoming_channel: BTreeMap::new(),
            session_by_outgoing_channel: BTreeMap::new(),

            remote_open: None,
            agreed_channel_max,
        }
    }
}

#[async_trait]
impl endpoint::Connection for Connection {
    type AllocError = AllocSessionError;
    type Error = Error;
    type State = ConnectionState;
    type Session = Session;

    fn local_state(&self) -> &Self::State {
        &self.local_state
    }

    fn local_state_mut(&mut self) -> &mut Self::State {
        &mut self.local_state
    }

    fn local_open(&self) -> &Open {
        &self.local_open
    }

    fn allocate_session(
        &mut self,
        tx: Sender<SessionIncomingItem>,
    ) -> Result<(u16, usize), Self::AllocError> {
        match &self.local_state {
            ConnectionState::Start
            | ConnectionState::HeaderSent
            | ConnectionState::HeaderReceived
            | ConnectionState::HeaderExchange
            | ConnectionState::CloseSent
            | ConnectionState::Discarding
            | ConnectionState::End => return Err(AllocSessionError::IllegalState),
            // TODO: what about pipelined open?
            _ => {}
        };

        // get new entry index
        let entry = self.local_sessions.vacant_entry();
        let session_id = entry.key();

        // check if there is enough
        if session_id > self.agreed_channel_max as usize {
            return Err(AllocSessionError::ChannelMaxReached);
        } else {
            entry.insert(tx);
            let channel = session_id as u16; // TODO: a different way of allocating session id?
            self.session_by_outgoing_channel.insert(channel, session_id);
            Ok((channel, session_id))
        }
    }

    fn deallocate_session(&mut self, session_id: usize) {
        self.local_sessions.remove(session_id);
    }

    /// Reacting to remote Open frame
    #[instrument(name = "RECV", skip_all)]
    async fn on_incoming_open(&mut self, channel: u16, open: Open) -> Result<(), Self::Error> {
        trace!(channel, frame = ?open);
        match &self.local_state {
            ConnectionState::HeaderExchange => self.local_state = ConnectionState::OpenReceived,
            ConnectionState::OpenSent => self.local_state = ConnectionState::Opened,
            ConnectionState::ClosePipe => self.local_state = ConnectionState::CloseSent,
            _ => return Err(Error::amqp_error(AmqpError::IllegalState, None)),
        }

        // set channel_max to mutually acceptable
        self.agreed_channel_max = min(self.local_open.channel_max.0, open.channel_max.0);
        self.remote_open = Some(open);

        Ok(())
    }

    /// Reacting to remote Begin frame
    #[instrument(name = "RECV", skip_all)]
    async fn on_incoming_begin(&mut self, channel: u16, begin: Begin) -> Result<(), Self::Error> {
        trace!(channel, frame = ?begin);
        match &self.local_state {
            ConnectionState::Opened => {}
            // TODO: what about pipelined
            _ => return Err(Error::amqp_error(AmqpError::IllegalState, None)), // TODO: what to do?
        }

        match begin.remote_channel {
            Some(outgoing_channel) => {
                let session_id = self
                    .session_by_outgoing_channel
                    .get(&outgoing_channel)
                    .ok_or_else(|| Error::amqp_error(AmqpError::NotFound, None))?; // Close with error NotFound

                if self.session_by_incoming_channel.contains_key(&channel) {
                    return Err(Error::amqp_error(AmqpError::NotAllowed, None)); // TODO: this is probably not how not allowed should be used?
                }
                self.session_by_incoming_channel
                    .insert(channel, *session_id);

                // forward begin to session
                let tx = self
                    .local_sessions
                    .get_mut(*session_id)
                    .ok_or_else(|| Error::amqp_error(AmqpError::NotFound, None))?;
                let sframe = SessionFrame::new(channel, SessionFrameBody::Begin(begin));
                tx.send(sframe).await?;
            }
            None => {
                // If a session is locally initiated, the remote-channel MUST NOT be set. When an endpoint responds
                // to a remotely initiated session, the remote-channel MUST be set to the channel on which the
                // remote session sent the begin.
                // TODO: allow remotely initiated session
                return Err(Error::amqp_error(
                    AmqpError::NotImplemented,
                    Some("Remotely initiazted session is not supported yet".to_string()),
                )); // Close with error NotImplemented
            }
        }

        Ok(())
    }

    /// Reacting to remote End frame
    #[instrument(name = "RECV", skip_all)]
    async fn on_incoming_end(&mut self, channel: u16, end: End) -> Result<(), Self::Error> {
        trace!(channel, frame = ?end);
        match &self.local_state {
            ConnectionState::Opened => {}
            _ => return Err(Error::amqp_error(AmqpError::IllegalState, None)),
        }

        // Forward to session
        let sframe = SessionFrame::new(channel, SessionFrameBody::End(end));
        // Drop incoming channel
        let session_id = self
            .session_by_incoming_channel
            .remove(&channel)
            .ok_or_else(|| Error::amqp_error(AmqpError::NotFound, None))?;
        self.local_sessions
            .get_mut(session_id)
            .ok_or_else(|| Error::amqp_error(AmqpError::NotFound, None))?
            .send(sframe)
            .await?;

        Ok(())
    }

    /// Reacting to remote Close frame
    #[instrument(name = "RECV", skip_all)]
    async fn on_incoming_close(&mut self, channel: u16, close: Close) -> Result<(), Self::Error> {
        trace!(channel, frame=?close);

        match &self.local_state {
            ConnectionState::Opened => {
                self.local_state = ConnectionState::CloseReceived;
                self.control.send(ConnectionControl::Close(None)).await?;
            }
            ConnectionState::CloseSent => self.local_state = ConnectionState::End,
            _ => return Err(Error::amqp_error(AmqpError::IllegalState, None)),
        };

        match close.error {
            Some(error) => Err(Error::Remote(error)),
            None => Ok(()),
        }
    }

    #[instrument(name = "SEND", skip_all)]
    async fn send_open<W>(&mut self, writer: &mut W) -> Result<(), Self::Error>
    where
        W: Sink<Frame> + Send + Unpin,
        W::Error: Into<Error>,
    {
        let body = FrameBody::Open(self.local_open.clone());
        let frame = Frame::new(0u16, body);
        trace!(channel = 0, frame = ?frame.body);
        writer.send(frame).await.map_err(Into::into)?;

        // change local state after successfully sending the frame
        match &self.local_state {
            ConnectionState::HeaderExchange => self.local_state = ConnectionState::OpenSent,
            ConnectionState::OpenReceived => self.local_state = ConnectionState::Opened,
            ConnectionState::HeaderSent => self.local_state = ConnectionState::OpenPipe,
            _ => return Err(Error::amqp_error(AmqpError::IllegalState, None)),
        }

        Ok(())
    }

    fn on_outgoing_begin(&mut self, channel: u16, begin: Begin) -> Result<Frame, Self::Error> {
        // TODO: the engine already checks that
        // match &self.local_state {
        //     ConnectionState::Opened => {}
        //     _ => return Err(Error::Message("Illegal local connection state")),
        // }

        let frame = Frame::new(channel, FrameBody::Begin(begin));
        Ok(frame)
    }

    #[instrument(skip_all)]
    fn on_outgoing_end(&mut self, channel: u16, end: End) -> Result<Frame, Self::Error> {
        self.session_by_outgoing_channel
            .remove(&channel)
            .ok_or_else(|| Error::amqp_error(AmqpError::NotFound, None))?;
        let frame = Frame::new(channel, FrameBody::End(end));
        Ok(frame)
    }

    // TODO: set a timeout for recving incoming Close
    #[instrument(name = "SEND", skip_all)]
    async fn send_close<W>(
        &mut self,
        writer: &mut W,
        error: Option<definitions::Error>,
    ) -> Result<(), Self::Error>
    where
        W: Sink<Frame> + Send + Unpin,
        W::Error: Into<Error>,
    {
        let frame = Frame::new(0u16, FrameBody::Close(Close { error }));
        trace!(channel=0, frame = ?frame.body);
        writer.send(frame).await.map_err(Into::into)?;

        match &self.local_state {
            ConnectionState::Opened => self.local_state = ConnectionState::CloseSent,
            ConnectionState::CloseReceived => self.local_state = ConnectionState::End,
            ConnectionState::OpenSent => self.local_state = ConnectionState::ClosePipe,
            ConnectionState::OpenPipe => self.local_state = ConnectionState::OpenClosePipe,
            _ => return Err(Error::amqp_error(AmqpError::IllegalState, None)),
        }
        Ok(())
    }

    fn session_tx_by_incoming_channel(
        &mut self,
        channel: u16,
    ) -> Option<&mut Sender<SessionIncomingItem>> {
        let session_id = self.session_by_incoming_channel.get(&channel)?;
        self.local_sessions.get_mut(*session_id)
    }

    fn session_tx_by_outgoing_channel(
        &mut self,
        channel: u16,
    ) -> Option<&mut Sender<SessionIncomingItem>> {
        let session_id = self.session_by_outgoing_channel.get(&channel)?;
        self.local_sessions.get_mut(*session_id)
    }
}
