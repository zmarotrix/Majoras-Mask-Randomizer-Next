//! The build stages, one module each (named after the old `Builder` methods).

mod player_model;
pub(crate) mod hacks;
mod speedups;
mod asm;
mod asm_config;
pub(crate) mod audio;
mod cosmetics;
mod enemies;
mod entrances;
mod gibdo;
mod gimmicks;
mod hint_text;
mod item_text;
mod messages;
mod more_text;
mod npc_text;
pub(crate) mod regions;
mod shop_text;
pub(crate) mod items;

pub(crate) use item_text::article;
pub(crate) use items::{alternate_name, display_name, pronoun, pronoun_or_amount, runtime_item_description, verb};
pub(crate) use speedups::flags_value;
