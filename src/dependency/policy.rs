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
            _ => Err((String::from("Unexpected update policy, expected one of [Always, SwitchOrUpdate, UpdateOnly, SwitchOrTask, SwitchConfigurationOnly, TaskOrUpdate, TaskInvokedOnly, Never]"), 30)),
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
