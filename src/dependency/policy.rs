#[derive(Clone, Default)]
pub enum UpdatePolicy {
    Always,
    #[default]
    SwitchOrUpdate,
    UpdateOnly,
    SwitchOrTask,
    SwitchConfigurationOnly,
    TaskOrUpdate,
    TaskInvokedOnly,
    Never,
}

#[derive(Clone, Copy)]
pub enum UpdateContext {
    Update,
    SwitchConfiguration,
    TaskInvoked,
    ResolveOnly,
}

impl UpdatePolicy {
    pub fn load(value: &str) -> Result<Self, (String, u8)> {
        match value {
            "Always" => Ok(UpdatePolicy::Always),
            "SwitchOrUpdate" => Ok(UpdatePolicy::SwitchOrUpdate),
            "UpdateOnly" => Ok(UpdatePolicy::UpdateOnly),
            "SwitchOrTask" => Ok(UpdatePolicy::SwitchOrTask),
            "SwitchConfigurationOnly" => Ok(UpdatePolicy::SwitchConfigurationOnly),
            "TaskOrUpdate" => Ok(UpdatePolicy::TaskOrUpdate),
            "TaskInvokedOnly" => Ok(UpdatePolicy::TaskInvokedOnly),
            "Never" => Ok(UpdatePolicy::Never),
            _ => Err((
                String::from(
                    "Unexpected update policy, expected one of [Always, SwitchOrUpdate, UpdateOnly, SwitchOrTask, SwitchConfigurationOnly, TaskOrUpdate, TaskInvokedOnly, Never]",
                ),
                30,
            )),
        }
    }

    pub fn should_update(&self, context: &UpdateContext) -> bool {
        match self {
            UpdatePolicy::Always => true,
            UpdatePolicy::Never => false,
            UpdatePolicy::SwitchOrUpdate => match context {
                UpdateContext::Update => true,
                UpdateContext::SwitchConfiguration => true,
                UpdateContext::TaskInvoked => false,
                UpdateContext::ResolveOnly => false,
            },
            UpdatePolicy::UpdateOnly => match context {
                UpdateContext::Update => true,
                UpdateContext::SwitchConfiguration => false,
                UpdateContext::TaskInvoked => false,
                UpdateContext::ResolveOnly => false,
            },
            UpdatePolicy::SwitchOrTask => match context {
                UpdateContext::Update => false,
                UpdateContext::SwitchConfiguration => true,
                UpdateContext::TaskInvoked => true,
                UpdateContext::ResolveOnly => false,
            },
            UpdatePolicy::SwitchConfigurationOnly => match context {
                UpdateContext::Update => false,
                UpdateContext::SwitchConfiguration => true,
                UpdateContext::TaskInvoked => false,
                UpdateContext::ResolveOnly => false,
            },
            UpdatePolicy::TaskOrUpdate => match context {
                UpdateContext::Update => true,
                UpdateContext::SwitchConfiguration => false,
                UpdateContext::TaskInvoked => true,
                UpdateContext::ResolveOnly => false,
            },
            UpdatePolicy::TaskInvokedOnly => match context {
                UpdateContext::Update => false,
                UpdateContext::SwitchConfiguration => false,
                UpdateContext::TaskInvoked => true,
                UpdateContext::ResolveOnly => false,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_all_known_update_policies() {
        assert!(matches!(
            UpdatePolicy::load("Always").unwrap(),
            UpdatePolicy::Always
        ));
        assert!(matches!(
            UpdatePolicy::load("SwitchOrUpdate").unwrap(),
            UpdatePolicy::SwitchOrUpdate
        ));
        assert!(matches!(
            UpdatePolicy::load("UpdateOnly").unwrap(),
            UpdatePolicy::UpdateOnly
        ));
        assert!(matches!(
            UpdatePolicy::load("SwitchOrTask").unwrap(),
            UpdatePolicy::SwitchOrTask
        ));
        assert!(matches!(
            UpdatePolicy::load("SwitchConfigurationOnly").unwrap(),
            UpdatePolicy::SwitchConfigurationOnly
        ));
        assert!(matches!(
            UpdatePolicy::load("TaskOrUpdate").unwrap(),
            UpdatePolicy::TaskOrUpdate
        ));
        assert!(matches!(
            UpdatePolicy::load("TaskInvokedOnly").unwrap(),
            UpdatePolicy::TaskInvokedOnly
        ));
        assert!(matches!(
            UpdatePolicy::load("Never").unwrap(),
            UpdatePolicy::Never
        ));
    }

    #[test]
    fn rejects_unknown_update_policy() {
        let error = match UpdatePolicy::load("Sometimes") {
            Ok(_) => panic!("expected unknown update policy to fail"),
            Err(error) => error,
        };

        assert!(error.0.contains("Unexpected update policy"));
        assert_eq!(error.1, 30);
    }

    #[test]
    fn update_policy_matches_expected_context_matrix() {
        let contexts = [
            UpdateContext::Update,
            UpdateContext::SwitchConfiguration,
            UpdateContext::TaskInvoked,
            UpdateContext::ResolveOnly,
        ];

        let cases = [
            (UpdatePolicy::Always, [true, true, true, true]),
            (UpdatePolicy::SwitchOrUpdate, [true, true, false, false]),
            (UpdatePolicy::UpdateOnly, [true, false, false, false]),
            (UpdatePolicy::SwitchOrTask, [false, true, true, false]),
            (
                UpdatePolicy::SwitchConfigurationOnly,
                [false, true, false, false],
            ),
            (UpdatePolicy::TaskOrUpdate, [true, false, true, false]),
            (UpdatePolicy::TaskInvokedOnly, [false, false, true, false]),
            (UpdatePolicy::Never, [false, false, false, false]),
        ];

        for (policy, expected) in cases {
            for (context, expected) in contexts.iter().zip(expected) {
                assert_eq!(policy.should_update(context), expected);
            }
        }
    }
}
