//! OCPP 1.6 inbound adapter and factory
//!
//! `V16InboundAdapter` implements `OcppInboundPort` by delegating to the
//! existing `OcppHandler`, which handles all OCPP 1.6 message parsing,
//! action routing, and response serialization via the `rust_ocpp` crate.

use std::sync::Arc;

use async_trait::async_trait;

use crate::application::events::SharedEventBus;
use crate::application::ports::{OcppAdapterFactory, OcppInboundPort};
use crate::application::OcppHandlerV16;
use crate::application::{BillingService, ChargePointService};
use crate::application::{CommandSender, SharedCommandSender};
use crate::domain::OcppVersion;

// ── V16InboundAdapter ──────────────────────────────────────────

/// OCPP 1.6 inbound adapter.
///
/// Wraps the existing `OcppHandler` to satisfy the `OcppInboundPort` trait.
/// One instance is created per charge-point connection.
pub struct V16InboundAdapter {
    handler: Arc<OcppHandlerV16>,
    cp_id: String,
}

impl V16InboundAdapter {
    pub fn new(
        charge_point_id: String,
        service: Arc<ChargePointService>,
        billing_service: Arc<BillingService>,
        command_sender: Arc<CommandSender>,
        event_bus: SharedEventBus,
    ) -> Self {
        let handler = Arc::new(OcppHandlerV16::new(
            charge_point_id.clone(),
            service,
            billing_service,
            command_sender,
            event_bus,
        ));
        Self {
            handler,
            cp_id: charge_point_id,
        }
    }
}

#[async_trait]
impl OcppInboundPort for V16InboundAdapter {
    async fn handle_message(&self, text: &str) -> Option<String> {
        self.handler.handle(text).await
    }

    fn version(&self) -> OcppVersion {
        OcppVersion::V16
    }

    fn charge_point_id(&self) -> &str {
        &self.cp_id
    }
}

// ── V16AdapterFactory ──────────────────────────────────────────

/// Factory for creating OCPP 1.6 inbound adapters.
///
/// Holds shared references to application services that each per-connection
/// adapter needs.
pub struct V16AdapterFactory {
    service: Arc<ChargePointService>,
    billing_service: Arc<BillingService>,
    command_sender: SharedCommandSender,
    event_bus: SharedEventBus,
}

impl V16AdapterFactory {
    pub fn new(
        service: Arc<ChargePointService>,
        billing_service: Arc<BillingService>,
        command_sender: SharedCommandSender,
        event_bus: SharedEventBus,
    ) -> Self {
        Self {
            service,
            billing_service,
            command_sender,
            event_bus,
        }
    }
}

impl OcppAdapterFactory for V16AdapterFactory {
    fn create_inbound_adapter(&self, charge_point_id: String) -> Box<dyn OcppInboundPort> {
        Box::new(V16InboundAdapter::new(
            charge_point_id,
            self.service.clone(),
            self.billing_service.clone(),
            self.command_sender.clone(),
            self.event_bus.clone(),
        ))
    }

    fn version(&self) -> OcppVersion {
        OcppVersion::V16
    }
}
