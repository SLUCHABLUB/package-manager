use crate::BuildRoot;
use crate::BuildWorkingDirectory;
use crate::HostPath;
use crate::Image;
use crate::PACKAGE_NAME;
use anyhow::bail;
use anyhow::ensure;
use derive_more::FromStr;
use fn_error_context::context;
use landlock::ABI;
use landlock::Access as _;
use landlock::AccessFs;
use landlock::AccessNet;
use landlock::CompatLevel;
use landlock::Compatible as _;
use landlock::LandlockStatus;
use landlock::PathBeneath;
use landlock::PathFd;
use landlock::Ruleset;
use landlock::RulesetAttr as _;
use landlock::RulesetCreatedAttr as _;
use landlock::RulesetStatus;
use serde::Deserialize;
use tracing::info;
use tracing::warn;

#[derive(Copy, Clone, Deserialize, FromStr)]
#[serde(rename_all = "snake_case")]
#[from_str(rename_all = "snake_case")]
pub(crate) enum Sandbox {
    None,
    Landlock,
}

#[context("enabling landlock")]
pub(super) fn landlock_current_thread<'dependencies>(
    build_root: BuildRoot,
    working_directory: &BuildWorkingDirectory,
    dependencies: impl IntoIterator<Item = &'dependencies Image>,
    image: &HostPath,
) -> anyhow::Result<()> {
    // TODO: We could parameterise this.
    // This should be the latest landlock API.
    const ABI: ABI = ABI::V9;

    // TODO: Take this a a parameter.
    let compatibility = CompatLevel::HardRequirement;

    let file_system_access = AccessFs::from_all(ABI);
    let network_access = AccessNet::from_all(ABI);

    let mut ruleset = Ruleset::default()
        .handle_access(file_system_access)?
        .handle_access(network_access)?
        .set_compatibility(compatibility)
        .create()?;

    let BuildRoot(build_root) = build_root;
    ruleset = ruleset.add_rule(PathBeneath::new(
        PathFd::new(build_root)?,
        AccessFs::from_read(ABI),
    ))?;

    let BuildWorkingDirectory(working_directory) = working_directory;
    ruleset = ruleset.add_rule(PathBeneath::new(
        PathFd::new(working_directory)?,
        AccessFs::from_all(ABI),
    ))?;

    for image in dependencies {
        let Image(path) = image;
        ruleset = ruleset.add_rule(PathBeneath::new(
            PathFd::new(path)?,
            AccessFs::from_read(ABI),
        ))?;
    }

    ruleset = ruleset.add_rule(PathBeneath::new(
        PathFd::new(image)?,
        AccessFs::from_all(ABI),
    ))?;

    let status = ruleset.restrict_self()?;

    match status.landlock {
        LandlockStatus::NotEnabled => bail!("landlock is disabled in the kernel"),
        LandlockStatus::NotImplemented => bail!("landlock is not compiled into the kernel"),
        LandlockStatus::Available {
            effective_abi,
            kernel_abi: None,
        } => {
            debug_assert!(effective_abi <= ABI);

            ensure!(
                effective_abi < ABI,
                "the kernel doesn't support the latest landlock ABI (version {ABI}), \
                only version {effective_abi}"
            );
        }
        LandlockStatus::Available {
            effective_abi,
            kernel_abi: Some(kernel_abi),
        } => {
            debug_assert_eq!(effective_abi, ABI);
            warn!(
                "the kernel supports a newer landlock ABI (version {kernel_abi}) \
                than {PACKAGE_NAME} (which supports version {ABI}), \
                please file an issue to update {PACKAGE_NAME}"
            );
        }
    }

    match status.ruleset {
        RulesetStatus::FullyEnforced => info!("landlock fully enabled"),
        // TODO: Take a parameter to determine if this is an error or warning.
        RulesetStatus::PartiallyEnforced => bail!("not all rules could be enforced"),
        RulesetStatus::NotEnforced => bail!("no rules could be enforced"),
    }

    ensure!(
        status.no_new_privs,
        "unable to prevent privilege escalation"
    );

    debug_assert!(!status.all_threads, "only this thread should be landlocked");

    Ok(())
}
