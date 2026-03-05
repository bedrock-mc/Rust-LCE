#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootReport {
    pub completed_steps: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BootstrapError {
    DuplicateStep(&'static str),
    StepFailed {
        step: &'static str,
        reason: String,
        completed_steps: Vec<&'static str>,
    },
}

type BootAction = Box<dyn FnMut() -> Result<(), String>>;

struct BootStep {
    id: &'static str,
    action: BootAction,
}

#[derive(Default)]
pub struct BootSequence {
    steps: Vec<BootStep>,
}

impl BootSequence {
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }

    pub fn register<F>(&mut self, id: &'static str, action: F) -> Result<(), BootstrapError>
    where
        F: FnMut() -> Result<(), String> + 'static,
    {
        if self.steps.iter().any(|step| step.id == id) {
            return Err(BootstrapError::DuplicateStep(id));
        }

        self.steps.push(BootStep {
            id,
            action: Box::new(action),
        });
        Ok(())
    }

    pub fn run(&mut self) -> Result<BootReport, BootstrapError> {
        let mut completed_steps = Vec::new();

        for step in &mut self.steps {
            match (step.action)() {
                Ok(()) => completed_steps.push(step.id),
                Err(reason) => {
                    return Err(BootstrapError::StepFailed {
                        step: step.id,
                        reason,
                        completed_steps,
                    });
                }
            }
        }

        Ok(BootReport { completed_steps })
    }
}
