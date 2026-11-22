//! Результат проверки доступности сервиса без обхода. Нужен проверке сервисов (`probe`), которой
//! пользуется VPN. Планирование обхода удалено (ADR 0019).

use super::Coverage;

/// Independent result of checking one service WITHOUT any bypass. `Unchecked` is never success.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Reachability {
    Reachable,
    Unreachable,
    #[default]
    Unchecked,
}

/// Per-service observations. Missing entries are `Unchecked`; the last value set wins.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ServiceObservations {
    entries: Vec<(Coverage, Reachability)>,
}

impl ServiceObservations {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with(mut self, service: Coverage, result: Reachability) -> Self {
        self.set(service, result);
        self
    }

    pub fn set(&mut self, service: Coverage, result: Reachability) {
        match self.entries.iter_mut().find(|(s, _)| *s == service) {
            Some(entry) => entry.1 = result,
            None => self.entries.push((service, result)),
        }
    }

    pub fn get(&self, service: Coverage) -> Reachability {
        self.entries
            .iter()
            .find(|(s, _)| *s == service)
            .map_or(Reachability::Unchecked, |(_, r)| *r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_observations_are_unchecked_and_the_last_value_wins() {
        let mut obs = ServiceObservations::new();
        assert_eq!(obs.get(Coverage::Youtube), Reachability::Unchecked);
        obs.set(Coverage::Youtube, Reachability::Reachable);
        obs.set(Coverage::Youtube, Reachability::Unreachable);
        assert_eq!(obs.get(Coverage::Youtube), Reachability::Unreachable);
        let obs = ServiceObservations::new().with(Coverage::Sites, Reachability::Reachable);
        assert_eq!(obs.get(Coverage::Sites), Reachability::Reachable);
        assert_eq!(obs.get(Coverage::Discord), Reachability::Unchecked);
    }
}
