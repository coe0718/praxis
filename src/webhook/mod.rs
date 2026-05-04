//! Webhook sub-module for direct-delivery extensions.

pub mod direct_delivery;

pub use direct_delivery::deliver_webhook_to_channel;
