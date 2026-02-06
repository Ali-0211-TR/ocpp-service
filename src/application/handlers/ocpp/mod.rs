//! OCPP 1.6 Action handlers
//! 
//! Each action has its own handler module for clean separation of concerns.

use log::warn;
use ocpp_rs::v16::call::Action;
use ocpp_rs::v16::call_result::ResultPayload;

use crate::application::OcppHandler;

mod handle_authorize;
mod handle_boot_notification;
mod handle_data_transfer;
mod handle_diagnostics_status_notification;
mod handle_firmware_status_notification;
mod handle_heartbeat;
mod handle_meter_values;
mod handle_security_event_notification;
mod handle_start_transaction;
mod handle_status_notification;
mod handle_stop_transaction;

pub use handle_authorize::handle_authorize;
pub use handle_boot_notification::handle_boot_notification;
pub use handle_data_transfer::handle_data_transfer;
pub use handle_diagnostics_status_notification::handle_diagnostics_status_notification;
pub use handle_firmware_status_notification::handle_firmware_status_notification;
pub use handle_heartbeat::handle_heartbeat;
pub use handle_meter_values::handle_meter_values;
pub use handle_security_event_notification::handle_security_event_notification;
pub use handle_start_transaction::handle_start_transaction;
pub use handle_status_notification::handle_status_notification;
pub use handle_stop_transaction::handle_stop_transaction;

/// Routes OCPP actions to their respective handlers
pub async fn action_matcher(handler: &OcppHandler, action: Action) -> ResultPayload {
    match action {
        // ===== Charge Point initiated (CP -> CS) =====
        // Эти сообщения станция отправляет серверу
        
        Action::Authorize(payload) => handle_authorize(handler, payload).await,
        Action::BootNotification(payload) => handle_boot_notification(handler, payload).await,
        Action::DataTransfer(payload) => handle_data_transfer(handler, payload).await,
        Action::DiagnosticsStatusNotification(payload) => {
            handle_diagnostics_status_notification(handler, payload).await
        }
        Action::FirmwareStatusNotification(payload) => {
            handle_firmware_status_notification(handler, payload).await
        }
        Action::Heartbeat(payload) => handle_heartbeat(handler, payload).await,
        Action::MeterValues(payload) => handle_meter_values(handler, payload).await,
        Action::SecurityEventNotification(payload) => {
            handle_security_event_notification(handler, payload).await
        }
        Action::StartTransaction(payload) => handle_start_transaction(handler, payload).await,
        Action::StatusNotification(payload) => handle_status_notification(handler, payload).await,
        Action::StopTransaction(payload) => handle_stop_transaction(handler, payload).await,

        // ===== Central System initiated (CS -> CP) =====
        // Эти команды сервер отправляет станции, они не должны приходить от станции
        // Если они пришли - это ошибка протокола, возвращаем NotImplemented
        
        Action::CancelReservation(_) |
        Action::CertificateSigned(_) |
        Action::ChangeAvailability(_) |
        Action::ChangeConfiguration(_) |
        Action::ClearCache(_) |
        Action::ClearChargingProfile(_) |
        Action::DeleteCertificate(_) |
        Action::ExtendedTriggerMessage(_) |
        Action::GetCompositeSchedule(_) |
        Action::GetConfiguration(_) |
        Action::GetDiagnostics(_) |
        Action::GetInstalledCertificateIds(_) |
        Action::GetLocalListVersion(_) |
        Action::GetLog(_) |
        Action::InstallCertificate(_) |
        Action::LogStatusNotification(_) |
        Action::RemoteStartTransaction(_) |
        Action::RemoteStopTransaction(_) |
        Action::ReserveNow(_) |
        Action::Reset(_) |
        Action::SendLocalList(_) |
        Action::SetChargingProfile(_) |
        Action::SignCertificate(_) |
        Action::SignedFirmwareStatusNotification(_) |
        Action::SignedUpdateFirmware(_) |
        Action::TriggerMessage(_) |
        Action::UnlockConnector(_) |
        Action::UpdateFirmware(_) => {
            warn!(
                "[{}] Received CS->CP action from charge point (protocol error): {:?}",
                handler.charge_point_id,
                action.as_ref()
            );
            // Return empty response for protocol errors
            ResultPayload::PossibleEmptyResponse(
                ocpp_rs::v16::call_result::EmptyResponses::EmptyResponse(
                    ocpp_rs::v16::call_result::EmptyResponse {},
                ),
            )
        }
    }
}