pub mod state;
pub mod turn_orchestrator;
pub mod stats;

#[cfg(test)]
mod test_resolve_turn;

#[cfg(test)]
mod test_critical_hits;

#[cfg(test)]
mod test_fainting;

#[cfg(test)]
mod test_multi_attacks;

#[cfg(test)]
mod test_end_of_turn;

#[cfg(test)]
mod test_action_prevention;