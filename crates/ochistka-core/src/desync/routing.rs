//! Pure one-click route planning for OCHISTKA (NO network or process changes).
//!
//! Direct, VPN, and one whole Zapret profile are mutually exclusive.
//! A parsed preset and successful Windows --dry-run DO NOT grant verified
//! coverage: only a separately reviewed, explicitly pinned evidence entry can.
//! No entries are approved yet.

use super::{Coverage, Engine, Platform, Profile, Readiness};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessPath {
    Direct,
    Vpn,
    Desync(&'static str),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RouteBlock {
    MissingServiceRequest,
    NeedDirectObservation,
    VpnNotVerified,
    DesyncAdapterUnavailable,
    UnverifiedCoverage,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoutePlan {
    Use(AccessPath),
    Block(RouteBlock),
}

struct ConfirmedCoverage {
    profile_id: &'static str,
    engine: Engine,
    covers: &'static [Coverage],
}

/// Intentionally empty. Add an entry ONLY when a real, repeatable Windows
/// network experiment confirms the entire claimed service coverage.
/// This cannot be filled by a compilation test, --dry-run, or localhost test.
const CONFIRMED_EXTERNAL_PROFILES: &[ConfirmedCoverage] = &[];

/// Policy for the future single "Обход блокировок" control.
/// The caller supplies an actual direct observation and VPN verification; the
/// adapter's own VPN active state is authoritative to prevent simultaneous
/// capture. No packet interception happens here.
pub fn choose_access_path(
    requested: &[Coverage],
    direct_reachable: Option<bool>,
    vpn_active: bool,
    vpn_verified: bool,
    adapter_ready: &Readiness,
    platform: Platform,
    candidate: Option<&Profile>,
) -> RoutePlan {
    choose_with_approved(
        requested,
        direct_reachable,
        vpn_active,
        vpn_verified,
        adapter_ready,
        platform,
        candidate,
        CONFIRMED_EXTERNAL_PROFILES,
    )
}

#[allow(clippy::too_many_arguments)]
fn choose_with_approved(
    requested: &[Coverage],
    direct_reachable: Option<bool>,
    vpn_active: bool,
    vpn_verified: bool,
    adapter_ready: &Readiness,
    platform: Platform,
    candidate: Option<&Profile>,
    approved: &[ConfirmedCoverage],
) -> RoutePlan {
    if requested.is_empty() {
        return RoutePlan::Block(RouteBlock::MissingServiceRequest);
    }
    // A currently running VPN is never overlaid with desync, even when a
    // browser test through the VPN happens to report "reachable".
    if vpn_active {
        return if vpn_verified {
            RoutePlan::Use(AccessPath::Vpn)
        } else {
            RoutePlan::Block(RouteBlock::VpnNotVerified)
        };
    }
    // No assumption that an unmeasured connection is blocked or reachable.
    match direct_reachable {
        Some(true) => return RoutePlan::Use(AccessPath::Direct),
        None => return RoutePlan::Block(RouteBlock::NeedDirectObservation),
        Some(false) => {}
    }
    if !matches!(adapter_ready, Readiness::Ready) {
        return RoutePlan::Block(RouteBlock::DesyncAdapterUnavailable);
    }
    let Some(candidate) = candidate else {
        return RoutePlan::Block(RouteBlock::UnverifiedCoverage);
    };
    let expected_engine = Engine::for_platform(platform);
    if expected_engine != Some(candidate.engine) {
        return RoutePlan::Block(RouteBlock::UnverifiedCoverage);
    }

    if approved.iter().any(|entry| {
        entry.profile_id == candidate.id
            && entry.engine == candidate.engine
            && requested
                .iter()
                .all(|service| entry.covers.contains(service))
    }) {
        RoutePlan::Use(AccessPath::Desync(candidate.id))
    } else {
        RoutePlan::Block(RouteBlock::UnverifiedCoverage)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate() -> Profile {
        super::super::windows_netzapret_v13_lite_candidate_profile().unwrap()
    }

    #[test]
    fn authentic_unverified_netzapret_candidate_cannot_be_selected() {
        let profile = candidate();
        let plan = choose_access_path(
            &[Coverage::Sites, Coverage::Discord],
            Some(false),
            false,
            false,
            &Readiness::Ready,
            Platform::Windows,
            Some(&profile),
        );
        assert_eq!(plan, RoutePlan::Block(RouteBlock::UnverifiedCoverage));
    }

    #[test]
    fn direct_first_only_when_actually_observed_reachable() {
        let profile = candidate();
        let inputs = (
            &[Coverage::Sites][..],
            false,
            false,
            &Readiness::NotImplemented("not ready".into()),
            Platform::Windows,
            Some(&profile),
        );
        assert_eq!(
            choose_access_path(
                inputs.0,
                Some(true),
                inputs.1,
                inputs.2,
                inputs.3,
                inputs.4,
                inputs.5
            ),
            RoutePlan::Use(AccessPath::Direct),
        );
        assert_eq!(
            choose_access_path(
                inputs.0,
                None,
                inputs.1,
                inputs.2,
                inputs.3,
                inputs.4,
                inputs.5
            ),
            RoutePlan::Block(RouteBlock::NeedDirectObservation),
        );
    }

    #[test]
    fn vpn_is_exclusive_and_requires_verified_connection() {
        let profile = candidate();
        let requested = &[Coverage::Sites, Coverage::Telegram];
        assert_eq!(
            choose_access_path(
                requested,
                Some(false),
                true,
                true,
                &Readiness::Ready,
                Platform::Windows,
                Some(&profile)
            ),
            RoutePlan::Use(AccessPath::Vpn),
        );
        assert_eq!(
            choose_access_path(
                requested,
                Some(false),
                true,
                false,
                &Readiness::Ready,
                Platform::Windows,
                Some(&profile)
            ),
            RoutePlan::Block(RouteBlock::VpnNotVerified),
        );
        // A reported direct success must not override an active VPN.
        assert_eq!(
            choose_access_path(
                requested,
                Some(true),
                true,
                false,
                &Readiness::Ready,
                Platform::Windows,
                Some(&profile)
            ),
            RoutePlan::Block(RouteBlock::VpnNotVerified),
        );
    }

    #[test]
    fn only_explicit_proven_coverage_may_choose_whole_preset() {
        let profile = candidate();
        let approval = [ConfirmedCoverage {
            profile_id: profile.id,
            engine: Engine::Winws2,
            covers: &[Coverage::Sites, Coverage::Discord],
        }];
        let chosen = |wanted: &[Coverage], platform, readiness: &Readiness| {
            choose_with_approved(
                    wanted,
                    Some(false),
                    false,
                    false,
                    readiness,
                    platform,
                    Some(&profile),
                    &approval
                )
        };
        assert_eq!(
            chosen(
                &[Coverage::Sites, Coverage::Discord],
                Platform::Windows,
                &Readiness::Ready
            ),
            RoutePlan::Use(AccessPath::Desync(profile.id))
        );
        assert_eq!(
            chosen(&[Coverage::Telegram], Platform::Windows, &Readiness::Ready),
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
                &Readiness::NotImplemented("not verified".into())
            ),
            RoutePlan::Block(RouteBlock::DesyncAdapterUnavailable)
        );
    }

    #[test]
    fn empty_service_request_is_never_claimed_as_working() {
        assert_eq!(
            choose_access_path(
                &[],
                Some(true),
                false,
                false,
                &Readiness::Ready,
                Platform::Windows,
                None
            ),
            RoutePlan::Block(RouteBlock::MissingServiceRequest),
        );
    }
}
