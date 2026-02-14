//! Database migrations module

pub use sea_orm_migration::prelude::*;

mod m20240101_000001_create_charge_points;
mod m20240101_000002_create_connectors;
mod m20240101_000003_create_transactions;
mod m20240101_000004_create_users;
mod m20240101_000005_create_api_keys;
mod m20240101_000006_create_id_tags;
mod m20240101_000007_create_tariffs;
mod m20240101_000008_add_billing_to_transactions;
mod m20240101_000009_add_meter_data_to_transactions;
mod m20240101_000010_add_ocpp_version_to_charge_points;
mod m20240101_000011_add_password_to_charge_points;
mod m20240101_000012_create_reservations;
mod m20240101_000013_create_charging_profiles;
mod m20240101_000014_add_external_order_id_to_transactions;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240101_000001_create_charge_points::Migration),
            Box::new(m20240101_000002_create_connectors::Migration),
            Box::new(m20240101_000003_create_transactions::Migration),
            Box::new(m20240101_000004_create_users::Migration),
            Box::new(m20240101_000005_create_api_keys::Migration),
            Box::new(m20240101_000006_create_id_tags::Migration),
            Box::new(m20240101_000007_create_tariffs::Migration),
            Box::new(m20240101_000008_add_billing_to_transactions::Migration),
            Box::new(m20240101_000009_add_meter_data_to_transactions::Migration),
            Box::new(m20240101_000010_add_ocpp_version_to_charge_points::Migration),
            Box::new(m20240101_000011_add_password_to_charge_points::Migration),
            Box::new(m20240101_000012_create_reservations::Migration),
            Box::new(m20240101_000013_create_charging_profiles::Migration),
            Box::new(m20240101_000014_add_external_order_id_to_transactions::Migration),
        ]
    }
}
