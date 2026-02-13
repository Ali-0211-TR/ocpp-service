//! Database entities module

pub mod api_key;
pub mod charge_point;
pub mod connector;
pub mod id_tag;
pub mod reservation;
pub mod tariff;
pub mod transaction;
pub mod user;

pub use api_key::Entity as ApiKey;
pub use charge_point::Entity as ChargePoint;
pub use connector::Entity as Connector;
pub use id_tag::Entity as IdTag;
pub use reservation::Entity as Reservation;
pub use tariff::Entity as Tariff;
pub use transaction::Entity as Transaction;
pub use user::Entity as User;
