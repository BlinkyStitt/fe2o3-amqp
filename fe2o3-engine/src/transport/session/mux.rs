
use fe2o3_amqp::primitives::{Symbol, UInt};
use fe2o3_types::definitions::{Fields, Handle, SequenceNo, TransferNumber};
use tokio::{sync::mpsc::{self, Receiver, Sender}, task::JoinHandle};

use crate::{error::EngineError, transport::{connection::{self, DEFAULT_CONTROL_CHAN_BUF, OutgoingChannelId}, session::SessionHandle}};

use super::{Session, SessionFrame, SessionFrameBody, SessionState};

pub const DEFAULT_SESSION_MUX_BUFFER_SIZE: usize = u16::MAX as usize;

pub(crate) struct SessionLocalOption {
    // control
    pub control: Receiver<SessionMuxControl>,
    
    // local states
    pub outgoing: Sender<SessionFrame>,
    // pub local_channel: u16,
    // local_state: SessionState,

    // pub next_incoming_id: TransferNumber,
    pub incoming_window: TransferNumber,
    pub next_outgoing_id: TransferNumber,
    pub outgoing_window: TransferNumber,

    pub handle_max: UInt,

    /// <field name="offered-capabilities" type="symbol" multiple="true"/>
    pub offered_capabilities: Option<Vec<Symbol>>,

    /// <field name="desired-capabilities" type="symbol" multiple="true"/>
    pub desired_capabilities: Option<Vec<Symbol>>,

    /// <field name="properties" type="fields"/>
    pub properties: Option<Fields>,
}

pub(crate) enum SessionMuxControl {
    End,
}

/// Mux has to be started from the Connection's Mux
pub(crate) struct SessionMux {
    // control
    control: Receiver<SessionMuxControl>,

    // local states
    outgoing: Sender<SessionFrame>,
    local_channel: OutgoingChannelId,
    local_state: SessionState,

    next_incoming_id: TransferNumber, // initialize with 0 first and change after receiving the remote Begin
    incoming_window: TransferNumber,
    next_outgoing_id: TransferNumber,
    outgoing_window: TransferNumber,

    handle_max: Handle,

    // remote states
    incoming: Receiver<Result<SessionFrame, EngineError>>,
    // remote_channel: u16,

    remote_incoming_window: SequenceNo,
    remote_outgoing_window: SequenceNo,
}

impl SessionMux {
    // pub async fn spawn_with_option(
    //     incoming: Receiver<Result<SessionFrame, EngineError>>,
    //     next_incoming_id: TransferNumber, // should be set after remote begin is received
    //     // remote_channel: u16,
    //     remote_incoming_window: Option<SequenceNo>,
    //     remote_outgoing_window: Option<SequenceNo>,
    //     local_channel: u16, // local channel number should be assigned by Connection Mux
    //     local_option: SessionLocalOption,
    // ) -> Result<JoinHandle<Result<(), EngineError>>, EngineError> {
    //     let SessionLocalOption {
    //         control,
    //         outgoing,
    //         // local_channel,
    //         // local_state,
    //         // next_incoming_id,
    //         incoming_window,
    //         next_outgoing_id,
    //         outgoing_window,
    //         handle_max,
            
    //         offered_capabilities,
    //         desired_capabilities,
    //         properties,
    //     } = local_option;

    //     let local_state = SessionState::Unmapped;

    //     let mux = SessionMux {
    //         outgoing,
    //         local_channel,
    //         local_state,
    //         next_incoming_id,
    //         incoming_window,
    //         next_outgoing_id,
    //         outgoing_window,
    //         handle_max,

    //         incoming,
    //         remote_incoming_window,
    //         remote_outgoing_window,
    //         control,
    //     };

    //     let handle = tokio::spawn(mux.mux_loop());
    //     Ok(handle)
    // }

    pub fn spawn(
        local_state: SessionState,
        local_channel: OutgoingChannelId,
        incoming: Receiver<Result<SessionFrame, EngineError>>,
        outgoing: Sender<SessionFrame>,
        next_outgoing_id: TransferNumber,
        incoming_window: TransferNumber,
        outgoing_window: TransferNumber,
        handle_max: Handle,
        offered_capabilities: Option<Vec<Symbol>>,
        desired_capabilities: Option<Vec<Symbol>>,
        properties: Option<Fields>,
        buffer_size: usize,
    ) -> Result<Session, EngineError> {

        // channels
        let (control_tx, control) = mpsc::channel(DEFAULT_CONTROL_CHAN_BUF);

        let mux = SessionMux {
            control,
            outgoing,
            local_channel,
            local_state,
            next_incoming_id: 0, // initialize with 0 and update when remote Begin is received
            incoming_window,
            next_outgoing_id,
            outgoing_window,
            handle_max,
            incoming,
            remote_incoming_window: 0, // initialize with 0 and update when remote Begin is received
            remote_outgoing_window: 0, // initialize with 0 and update when remote Begin is received
        };

        let handle = tokio::spawn(mux.mux_loop());
        let session = Session{
            mux: control_tx,
            handle
        };
        Ok(session)
    }

    // pub fn spawn(self) -> JoinHandle<Result<(), EngineError>> {
    //     tokio::spawn(self.mux_loop())
    // }

    async fn mux_loop(mut self) -> Result<(), EngineError> {
        loop {
            println!(">>> Debug SessionMux mux_loop");
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    }
}
