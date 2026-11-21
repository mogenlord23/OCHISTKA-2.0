//! Pure one-click route planning for OCHISTKA (NO network or process changes).
//!
//! Every requested service is judged separately from its OWN observation
//! (`Reachable` / `Unreachable` / `Unchecked`): one shared "direct works" flag can never stand
//! for YouTube, Discord, Telegram, sites and games at once. Direct, VPN and one whole bypass
//! profile are mutually exclusive. A successful Windows `--dry-run` DOES NOT
//! grant verified coverage: only a separately reviewed, explicitly pinned evidence entry can.
//! No entries are approved yet, so the desync path is never selected.

use super::{Coverage, Engine, Platform, Profile, Readiness};
use crate::vpn::service::{ConnState, ConnectionStatus};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessPath {
    Direct,
    Vpn,
    Desync(&'static str),
}

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

/// What the real VPN service reports. `active` covers connecting AND connected (no packet
/// capture may overlap either); `verified` means the VPN itself passed its traffic check. It is
/// NOT evidence that any particular service works.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct VpnSignal {
    pub active: bool,
    pub verified: bool,
}

impl VpnSignal {
    /// Maps the status of the existing `VpnService` (no separate bookkeeping).
    pub fn from_status(status: &ConnectionStatus) -> Self {
        match status.state {
            ConnState::Connecting => Self {
                active: true,
                verified: false,
            },
            ConnState::Connected => Self {
                active: true,
                verified: status.verified,
            },
            ConnState::Disconnected | ConnState::Error => Self::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RouteBlock {
    MissingServiceRequest,
    NeedDirectObservation,
    VpnNotVerified,
    /// The VPN mode in use is a system proxy: it does not capture this service's traffic.
    VpnModeDoesNotCover,
    DesyncAdapterUnavailable,
    UnverifiedCoverage,
}

impl RouteBlock {
    pub fn message(self) -> &'static str {
        match self {
            RouteBlock::MissingServiceRequest => "Не выбрано, что нужно открыть",
            RouteBlock::NeedDirectObservation => "Доступность сервиса ещё не проверена",
            RouteBlock::VpnNotVerified => "VPN запущен, но его работа не подтверждена",
            RouteBlock::VpnModeDoesNotCover => {
                "VPN в режиме системного прокси не охватывает этот сервис"
            }
            RouteBlock::DesyncAdapterUnavailable => "Обход блокировок на этой системе недоступен",
            RouteBlock::UnverifiedCoverage => "Обход для этого сервиса пока не подтверждён",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoutePlan {
    Use(AccessPath),
    Block(RouteBlock),
}

/// Decision for one service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ServiceRoute {
    pub service: Coverage,
    pub plan: RoutePlan,
}

/// Per-service decisions plus the one overall decision the single button acts on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccessPlan {
    pub per_service: Vec<ServiceRoute>,
    pub overall: RoutePlan,
}

pub(super) struct ConfirmedCoverage {
    pub(super) profile_id: &'static str,
    pub(super) engine: Engine,
    pub(super) covers: &'static [Coverage],
}

/// Intentionally empty. Add an entry ONLY when a real, repeatable Windows
/// network experiment confirms the entire claimed service coverage.
/// This cannot be filled by a compilation test, --dry-run, or localhost test.
pub(super) const CONFIRMED_EXTERNAL_PROFILES: &[ConfirmedCoverage] = &[];

/// A system-proxy VPN (the only VPN mode implemented) is seen by proxy-aware programs, so only
/// sites are claimed. Telegram, Discord and games are not assumed to follow the system proxy.
fn vpn_proxy_covers(service: Coverage) -> bool {
    matches!(service, Coverage::Sites)
}

/// Policy for the future single "Обход блокировок" control.
pub fn plan_services(
    requested: &[Coverage],
    observed: &ServiceObservations,
    vpn: VpnSignal,
    adapter_ready: &Readiness,
    platform: Platform,
    candidate: Option<&Profile>,
) -> AccessPlan {
    plan_with_approved(
        requested,
        observed,
        vpn,
        adapter_ready,
        platform,
        candidate,
        CONFIRMED_EXTERNAL_PROFILES,
    )
}

pub(super) fn plan_with_approved(
    requested: &[Coverage],
    observed: &ServiceObservations,
    vpn: VpnSignal,
    adapter_ready: &Readiness,
    platform: Platform,
    candidate: Option<&Profile>,
    approved: &[ConfirmedCoverage],
) -> AccessPlan {
    if requested.is_empty() {
        return AccessPlan {
            per_service: Vec::new(),
            overall: RoutePlan::Block(RouteBlock::MissingServiceRequest),
        };
    }
    let mut per_service: Vec<ServiceRoute> = Vec::new();
    for &service in requested {
        if per_service.iter().any(|r| r.service == service) {
            continue;
        }
        let plan = plan_one(
            service,
            observed,
            vpn,
            adapter_ready,
            platform,
            candidate,
            approved,
        );
        per_service.push(ServiceRoute { service, plan });
    }
    let overall = overall_plan(&per_service);
    AccessPlan {
        per_service,
        overall,
    }
}

fn plan_one(
    service: Coverage,
    observed: &ServiceObservations,
    vpn: VpnSignal,
    adapter_ready: &Readiness,
    platform: Platform,
    candidate: Option<&Profile>,
    approved: &[ConfirmedCoverage],
) -> RoutePlan {
    // A running (or starting) VPN is never overlaid with desync. Earlier direct observations
    // are stale while it runs, and the VPN link itself proves nothing about a given service.
    if vpn.active {
        return if !vpn.verified {
            RoutePlan::Block(RouteBlock::VpnNotVerified)
        } else if vpn_proxy_covers(service) {
            RoutePlan::Use(AccessPath::Vpn)
        } else {
            RoutePlan::Block(RouteBlock::VpnModeDoesNotCover)
        };
    }
    match observed.get(service) {
        Reachability::Reachable => return RoutePlan::Use(AccessPath::Direct),
        Reachability::Unchecked => return RoutePlan::Block(RouteBlock::NeedDirectObservation),
        Reachability::Unreachable => {}
    }
    if !matches!(adapter_ready, Readiness::Ready) {
        return RoutePlan::Block(RouteBlock::DesyncAdapterUnavailable);
    }
    let Some(candidate) = candidate else {
        return RoutePlan::Block(RouteBlock::UnverifiedCoverage);
    };
    if Engine::for_platform(platform) != Some(candidate.engine) {
        return RoutePlan::Block(RouteBlock::UnverifiedCoverage);
    }
    let proven = approved.iter().any(|entry| {
        entry.profile_id == candidate.id
            && entry.engine == candidate.engine
            && entry.covers.contains(&service)
    });
    if proven {
        RoutePlan::Use(AccessPath::Desync(candidate.id))
    } else {
        RoutePlan::Block(RouteBlock::UnverifiedCoverage)
    }
}

/// One overall decision: any blocked service blocks the whole request (the first reason is
/// reported); otherwise all-direct, all-VPN, or the single desync profile for the services that
/// need it (reachable ones simply ride along).
fn overall_plan(per_service: &[ServiceRoute]) -> RoutePlan {
    if let Some(blocked) = per_service.iter().find_map(|r| match r.plan {
        RoutePlan::Block(reason) => Some(reason),
        RoutePlan::Use(_) => None,
    }) {
        return RoutePlan::Block(blocked);
    }
    let mut desync: Option<&'static str> = None;
    let mut vpn = false;
    for route in per_service {
        match route.plan {
            RoutePlan::Use(AccessPath::Desync(id)) => match desync {
                None => desync = Some(id),
                Some(existing) if existing == id => {}
                // Two different whole profiles must never run together.
                Some(_) => return RoutePlan::Block(RouteBlock::UnverifiedCoverage),
            },
            RoutePlan::Use(AccessPath::Vpn) => vpn = true,
            _ => {}
        }
    }
    match (desync, vpn) {
        (Some(_), true) => RoutePlan::Block(RouteBlock::UnverifiedCoverage),
        (Some(id), false) => RoutePlan::Use(AccessPath::Desync(id)),
        (None, true) => RoutePlan::Use(AccessPath::Vpn),
        (None, false) => RoutePlan::Use(AccessPath::Direct),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate() -> Profile {
        super::super::sonicdpi_profile()
    }

    fn approval(profile: &Profile, covers: &'static [Coverage]) -> Vec<ConfirmedCoverage> {
        vec![ConfirmedCoverage {
            profile_id: profile.id,
            engine: Engine::SonicDpi,
            covers,
        }]
    }

    fn plan(
        wanted: &[Coverage],
        observed: &ServiceObservations,
        vpn: VpnSignal,
        readiness: &Readiness,
        platform: Platform,
        profile: &Profile,
        approved: &[ConfirmedCoverage],
    ) -> AccessPlan {
        plan_with_approved(
            wanted,
            observed,
            vpn,
            readiness,
            platform,
            Some(profile),
            approved,
        )
    }

    fn route(plan: &AccessPlan, service: Coverage) -> RoutePlan {
        plan.per_service
            .iter()
            .find(|r| r.service == service)
            .unwrap()
            .plan
    }

    const NO_VPN: VpnSignal = VpnSignal {
        active: false,
        verified: false,
    };

    #[test]
    fn authentic_unverified_netzapret_candidate_cannot_be_selected() {
        let profile = candidate();
        let observed = ServiceObservations::new().with(Coverage::Sites, Reachability::Unreachable);
        let plan = plan_services(
            &[Coverage::Sites, Coverage::Discord],
            &observed,
            NO_VPN,
            &Readiness::Ready,
            Platform::Windows,
            Some(&profile),
        );
        assert_eq!(
            plan.overall,
            RoutePlan::Block(RouteBlock::UnverifiedCoverage)
        );
    }

    #[test]
    fn unchecked_is_never_success_and_one_service_cannot_vouch_for_another() {
        let profile = candidate();
        // Only «Сайты» were observed reachable: Discord and Telegram stay unchecked.
        let observed = ServiceObservations::new().with(Coverage::Sites, Reachability::Reachable);
        let wanted = [Coverage::Sites, Coverage::Discord, Coverage::Telegram];
        let p = plan(
            &wanted,
            &observed,
            NO_VPN,
            &Readiness::Ready,
            Platform::Windows,
            &profile,
            &[],
        );
        assert_eq!(
            route(&p, Coverage::Sites),
            RoutePlan::Use(AccessPath::Direct)
        );
        assert_eq!(
            route(&p, Coverage::Discord),
            RoutePlan::Block(RouteBlock::NeedDirectObservation)
        );
        assert_eq!(
            route(&p, Coverage::Telegram),
            RoutePlan::Block(RouteBlock::NeedDirectObservation)
        );
        assert_eq!(
            p.overall,
            RoutePlan::Block(RouteBlock::NeedDirectObservation)
        );
        assert_eq!(observed.get(Coverage::Games), Reachability::Unchecked);
    }

    #[test]
    fn mixed_availability_is_decided_per_service() {
        let profile = candidate();
        let approved = approval(&profile, &[Coverage::Sites, Coverage::Discord]);
        let observed = ServiceObservations::new()
            .with(Coverage::Sites, Reachability::Reachable)
            .with(Coverage::Discord, Reachability::Unreachable)
            .with(Coverage::Telegram, Reachability::Unchecked);
        let wanted = [Coverage::Sites, Coverage::Discord, Coverage::Telegram];
        let p = plan(
            &wanted,
            &observed,
            NO_VPN,
            &Readiness::Ready,
            Platform::Windows,
            &profile,
            &approved,
        );
        assert_eq!(
            route(&p, Coverage::Sites),
            RoutePlan::Use(AccessPath::Direct)
        );
        assert_eq!(
            route(&p, Coverage::Discord),
            RoutePlan::Use(AccessPath::Desync(profile.id))
        );
        assert_eq!(
            p.overall,
            RoutePlan::Block(RouteBlock::NeedDirectObservation)
        );
        // Once every service has its own observation the single decision is the whole profile.
        let observed = observed.with(Coverage::Telegram, Reachability::Reachable);
        let p = plan(
            &wanted,
            &observed,
            NO_VPN,
            &Readiness::Ready,
            Platform::Windows,
            &profile,
            &approved,
        );
        assert_eq!(p.overall, RoutePlan::Use(AccessPath::Desync(profile.id)));
        // Telegram blocked but not covered by the approval: the whole request is refused.
        let observed = observed.with(Coverage::Telegram, Reachability::Unreachable);
        let p = plan(
            &wanted,
            &observed,
            NO_VPN,
            &Readiness::Ready,
            Platform::Windows,
            &profile,
            &approved,
        );
        assert_eq!(
            route(&p, Coverage::Telegram),
            RoutePlan::Block(RouteBlock::UnverifiedCoverage)
        );
        assert_eq!(p.overall, RoutePlan::Block(RouteBlock::UnverifiedCoverage));
    }

    #[test]
    fn all_reachable_means_direct_and_nothing_to_start() {
        let profile = candidate();
        let observed = ServiceObservations::new()
            .with(Coverage::Sites, Reachability::Reachable)
            .with(Coverage::Games, Reachability::Reachable);
        let p = plan(
            &[Coverage::Sites, Coverage::Games],
            &observed,
            NO_VPN,
            &Readiness::NotImplemented("not ready".into()),
            Platform::Windows,
            &profile,
            &[],
        );
        assert_eq!(p.overall, RoutePlan::Use(AccessPath::Direct));
    }

    #[test]
    fn active_vpn_is_exclusive_never_proof_for_all_services_and_blocks_desync() {
        let profile = candidate();
        let approved = approval(&profile, &[Coverage::Sites, Coverage::Discord]);
        let blocked_everywhere = ServiceObservations::new()
            .with(Coverage::Sites, Reachability::Unreachable)
            .with(Coverage::Discord, Reachability::Unreachable);
        let verified = VpnSignal {
            active: true,
            verified: true,
        };
        let p = plan(
            &[Coverage::Sites],
            &blocked_everywhere,
            verified,
            &Readiness::Ready,
            Platform::Windows,
            &profile,
            &approved,
        );
        assert_eq!(p.overall, RoutePlan::Use(AccessPath::Vpn));
        // The system-proxy VPN does not claim Discord, even with an approved desync profile.
        let both = [Coverage::Sites, Coverage::Discord];
        let p = plan(
            &both,
            &blocked_everywhere,
            verified,
            &Readiness::Ready,
            Platform::Windows,
            &profile,
            &approved,
        );
        assert_eq!(route(&p, Coverage::Sites), RoutePlan::Use(AccessPath::Vpn));
        assert_eq!(
            route(&p, Coverage::Discord),
            RoutePlan::Block(RouteBlock::VpnModeDoesNotCover)
        );
        assert_eq!(p.overall, RoutePlan::Block(RouteBlock::VpnModeDoesNotCover));
        // Connecting / unverified VPN: blocked, and stale "reachable" observations do not help.
        let reachable = ServiceObservations::new().with(Coverage::Sites, Reachability::Reachable);
        let unverified = VpnSignal {
            active: true,
            verified: false,
        };
        let p = plan(
            &[Coverage::Sites],
            &reachable,
            unverified,
            &Readiness::Ready,
            Platform::Windows,
            &profile,
            &approved,
        );
        assert_eq!(p.overall, RoutePlan::Block(RouteBlock::VpnNotVerified));
    }

    #[test]
    fn approval_is_per_service_engine_and_platform() {
        let profile = candidate();
        let approved = approval(&profile, &[Coverage::Sites]);
        let observed = ServiceObservations::new()
            .with(Coverage::Sites, Reachability::Unreachable)
            .with(Coverage::Discord, Reachability::Unreachable);
        let chosen = |wanted: &[Coverage], platform, readiness: &Readiness| {
            plan(
                wanted, &observed, NO_VPN, readiness, platform, &profile, &approved,
            )
            .overall
        };
        assert_eq!(
            chosen(&[Coverage::Sites], Platform::Windows, &Readiness::Ready),
            RoutePlan::Use(AccessPath::Desync(profile.id))
        );
        assert_eq!(
            chosen(&[Coverage::Discord], Platform::Windows, &Readiness::Ready),
            RoutePlan::Block(RouteBlock::UnverifiedCoverage)
        );
        assert_eq!(
            chosen(&[Coverage::Sites], Platform::Linux, &Readiness::Ready),
            RoutePlan::Block(RouteBlock::UnverifiedCoverage)
        );
        assert_eq!(
            chosen(
                &[Coverage::Sites],
                Platform::Windows,
                &Readiness::NotImplemented("x".into())
            ),
            RoutePlan::Block(RouteBlock::DesyncAdapterUnavailable)
        );
        assert_eq!(
            plan_with_approved(
                &[Coverage::Sites],
                &observed,
                NO_VPN,
                &Readiness::Ready,
                Platform::Windows,
                None,
                &approved
            )
            .overall,
            RoutePlan::Block(RouteBlock::UnverifiedCoverage)
        );
    }

    #[test]
    fn two_different_whole_profiles_are_never_combined() {
        let routes = [
            ServiceRoute {
                service: Coverage::Sites,
                plan: RoutePlan::Use(AccessPath::Desync("a")),
            },
            ServiceRoute {
                service: Coverage::Discord,
                plan: RoutePlan::Use(AccessPath::Desync("b")),
            },
        ];
        assert_eq!(
            overall_plan(&routes),
            RoutePlan::Block(RouteBlock::UnverifiedCoverage)
        );
        let mixed = [
            ServiceRoute {
                service: Coverage::Sites,
                plan: RoutePlan::Use(AccessPath::Desync("a")),
            },
            ServiceRoute {
                service: Coverage::Discord,
                plan: RoutePlan::Use(AccessPath::Vpn),
            },
        ];
        assert_eq!(
            overall_plan(&mixed),
            RoutePlan::Block(RouteBlock::UnverifiedCoverage)
        );
    }

    #[test]
    fn empty_request_and_duplicates() {
        let profile = candidate();
        let p = plan(
            &[],
            &ServiceObservations::new(),
            NO_VPN,
            &Readiness::Ready,
            Platform::Windows,
            &profile,
            &[],
        );
        assert_eq!(
            p.overall,
            RoutePlan::Block(RouteBlock::MissingServiceRequest)
        );
        let observed = ServiceObservations::new().with(Coverage::Sites, Reachability::Reachable);
        let p = plan(
            &[Coverage::Sites, Coverage::Sites],
            &observed,
            NO_VPN,
            &Readiness::Ready,
            Platform::Windows,
            &profile,
            &[],
        );
        assert_eq!(p.per_service.len(), 1);
    }

    #[test]
    fn vpn_signal_follows_the_real_connection_state() {
        let status = |state, verified| ConnectionStatus {
            state,
            server_id: None,
            server_name: None,
            since: None,
            message: String::new(),
            verified,
        };
        assert_eq!(
            VpnSignal::from_status(&status(ConnState::Disconnected, true)),
            VpnSignal::default()
        );
        assert_eq!(
            VpnSignal::from_status(&status(ConnState::Error, true)),
            VpnSignal::default()
        );
        assert_eq!(
            VpnSignal::from_status(&status(ConnState::Connecting, true)),
            VpnSignal {
                active: true,
                verified: false
            }
        );
        assert_eq!(
            VpnSignal::from_status(&status(ConnState::Connected, false)),
            VpnSignal {
                active: true,
                verified: false
            }
        );
        assert_eq!(
            VpnSignal::from_status(&status(ConnState::Connected, true)),
            VpnSignal {
                active: true,
                verified: true
            }
        );
    }

    #[test]
    fn observations_default_to_unchecked_and_last_value_wins() {
        let mut o = ServiceObservations::new();
        assert_eq!(o.get(Coverage::Games), Reachability::Unchecked);
        o.set(Coverage::Games, Reachability::Unreachable);
        o.set(Coverage::Games, Reachability::Reachable);
        assert_eq!(o.get(Coverage::Games), Reachability::Reachable);
    }
}
