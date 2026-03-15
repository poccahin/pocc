//! `pocc-market` – POCC Silicon Economy market-making primitives.
//!
//! This crate provides the pricing and auction infrastructure that lets every
//! edge daemon participate in POCC's decentralised cognitive-task marketplace.

pub mod adaptive_pricer;

pub use adaptive_pricer::AdaptivePricingEngine;
