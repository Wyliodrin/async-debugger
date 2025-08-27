use serde::{Deserialize, Serialize};

use crate::domain::Task;
use std::{
    fmt::Debug,
    time::{Duration, SystemTime},
};
#[allow(dead_code)]
pub trait Warn<T>: Debug {
    fn is_enabled(&self) -> bool;
    /// Returns if the warning applies to `val`.
    fn check(&self, val: &T) -> Warning;

    /// Formats a description of the warning detected for a *specific* `val`.
    ///
    /// This may include dynamically formatted content specific to `val`, such
    /// as the specific numeric value that was over the line for detecting the
    /// warning.
    ///
    /// This should be a complete sentence describing the warning. For example,
    /// for the [`SelfWakePercent`] warning, this returns a string like:
    ///
    /// > "This task has woken itself for more than 50% of its total wakeups (86%)"
    fn format(&self, val: &T) -> String;

    // /// Returns a string summarizing the warning *in general*, suitable for
    // /// displaying in a list of all detected warnings.
    // ///
    // /// The list entry will begin with a count of the number of monitored
    // /// entities for which the warning was detected. Therefore, this should be a
    // /// sentence fragment suitable to follow a count. For example, for the
    // /// [`SelfWakePercent`] warning, this method will return a string like
    // ///
    // /// > "tasks have woken themselves more than 50% of the time"
    // ///
    // /// so that the warnings list can read
    // ///
    // /// > "45 tasks have woken themselves more than 50% of the time"
    // /// TODO
    fn summary(&self) -> &str;
}

/// A result for a warning check
pub enum Warning {
    /// No warning for this entity.
    Ok,

    /// A warning has been detected for this entity.
    Warn,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskWarnings {
    self_wake_percent: SelfWakePercent,
    lost_waker: LostWaker,
    never_yielded: NeverYielded,
    auto_boxed_feature: AutoBoxedFuture,
    large_feature: LargeFuture,
}

impl TaskWarnings {
    pub(crate) fn new() -> Self {
        let self_wake_percent = SelfWakePercent::default();
        let lost_waker = LostWaker::new();
        let never_yielded = NeverYielded::default();
        let auto_boxed_feature = AutoBoxedFuture::new();
        let large_feature = LargeFuture::default();

        Self {
            self_wake_percent,
            lost_waker,
            never_yielded,
            auto_boxed_feature,
            large_feature,
        }
    }

    pub(crate) fn check(&self, task: &Task) -> Vec<String> {
        let mut warnings = Vec::new();

        if self.self_wake_percent.is_enabled() {
            match self.self_wake_percent.check(task) {
                Warning::Warn => warnings.push(self.self_wake_percent.format(task)),
                Warning::Ok => {}
            };
        }

        if self.lost_waker.is_enabled() {
            match self.lost_waker.check(task) {
                Warning::Warn => warnings.push(self.lost_waker.format(task)),
                Warning::Ok => {}
            };
        }

        if self.never_yielded.is_enabled() {
            match self.never_yielded.check(task) {
                Warning::Warn => warnings.push(self.never_yielded.format(task)),
                Warning::Ok => {}
            }
        }

        warnings
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct SelfWakePercent {
    enabled: bool,
    min_percent: u64,
    description: String,
}

impl SelfWakePercent {
    pub(crate) const DEFAULT_PERCENT: u64 = 50;
    pub(crate) fn new(min_percent: u64) -> Self {
        Self {
            enabled: true,
            min_percent,
            description: format!(
                "tasks have woken themselves over {}% of the time",
                min_percent
            ),
        }
    }
}

impl Default for SelfWakePercent {
    fn default() -> Self {
        Self::new(Self::DEFAULT_PERCENT)
    }
}

impl Warn<Task> for SelfWakePercent {
    fn is_enabled(&self) -> bool {
        self.enabled
    }
    fn summary(&self) -> &str {
        self.description.as_str()
    }

    fn check(&self, task: &Task) -> Warning {
        // Don't fire warning for tasks that are not async
        if task.is_blocking() {
            return Warning::Ok;
        }
        let self_wakes = task.self_wake_percent();
        if self_wakes > self.min_percent {
            Warning::Warn
        } else {
            Warning::Ok
        }
    }

    fn format(&self, task: &Task) -> String {
        let self_wakes = task.self_wake_percent();
        let option_task_name = task.name.clone();
        let task_name;

        if let Some(name) = option_task_name {
            if name != "" {
                task_name = name;
            } else {
                task_name = task.id();
            }
        } else {
            task_name = task.id();
        }
        format!(
            "Task {task_name} has woken itself for more than {}% of its total wakeups ({}%)",
            self.min_percent, self_wakes
        )
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct LostWaker {
    enabled: bool,
}

impl LostWaker {
    fn new() -> Self {
        Self { enabled: true }
    }
}

impl Warn<Task> for LostWaker {
    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn summary(&self) -> &str {
        "tasks have lost their wakers"
    }

    fn check(&self, task: &Task) -> Warning {
        // Don't fire warning for tasks that are not async
        if task.is_blocking() {
            return Warning::Ok;
        }
        if !task.is_completed()
            && task.waker_count() == 0
            && !task.is_running()
            && !task.is_awakened()
        {
            Warning::Warn
        } else {
            Warning::Ok
        }
    }

    fn format(&self, task: &Task) -> String {
        let option_task_name = task.name.clone();
        let task_name;
        if let Some(name) = option_task_name {
            task_name = name
        } else {
            task_name = task.id()
        }
        format!("Task {task_name} has lost its waker, and will never be woken again.")
    }
}

/// Warning for if a task has never yielded
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct NeverYielded {
    enabled: bool,
    min_duration: Duration,
    description: String,
}

impl NeverYielded {
    pub(crate) const DEFAULT_DURATION: Duration = Duration::from_secs(1);
    pub(crate) fn new(min_duration: Duration) -> Self {
        Self {
            enabled: true,
            min_duration,
            description: format!(
                "tasks have never yielded (threshold {}ms)",
                min_duration.as_millis()
            ),
        }
    }
}

impl Default for NeverYielded {
    fn default() -> Self {
        Self::new(Self::DEFAULT_DURATION)
    }
}

impl Warn<Task> for NeverYielded {
    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn summary(&self) -> &str {
        self.description.as_str()
    }

    fn check(&self, task: &Task) -> Warning {
        // Don't fire warning for tasks that are not async
        if task.is_blocking() {
            return Warning::Ok;
        }
        // Don't fire warning for tasks that are waiting to run
        if task.is_completed() || (!task.is_completed() && !task.is_running()) {
            return Warning::Ok;
        }

        if task.total_polls() > 1 {
            return Warning::Ok;
        }

        // Avoid short-lived task false positives
        if task.busy(SystemTime::now()) >= self.min_duration {
            return Warning::Warn;
        }

        return Warning::Ok;
    }

    fn format(&self, task: &Task) -> String {
        let option_task_name = task.name.clone();
        let task_name;
        if let Some(name) = option_task_name {
            task_name = name
        } else {
            task_name = task.id()
        }
        format!(
            "Task {task_name} has never yielded ({:?})",
            task.busy(SystemTime::now()),
        )
    }
}

/// Warning for if a task's driving future was auto-boxed by the runtime
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct AutoBoxedFuture {
    enabled: bool,
}

impl AutoBoxedFuture {
    fn new() -> Self {
        Self { enabled: true }
    }
}

impl Warn<Task> for AutoBoxedFuture {
    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn summary(&self) -> &str {
        "tasks have been boxed by the runtime due to their size"
    }

    fn check(&self, task: &Task) -> Warning {
        let (Some(size_bytes), Some(original_size_bytes)) =
            (task.size_bytes(), task.original_size_bytes())
        else {
            return Warning::Ok;
        };

        if original_size_bytes != size_bytes {
            Warning::Warn
        } else {
            Warning::Ok
        }
    }

    fn format(&self, task: &Task) -> String {
        let original_size = task
            .original_size_bytes()
            .expect("warning should not trigger if original size is None");
        let boxed_size = task
            .size_bytes()
            .expect("warning should not trigger if size is None");
        let option_task_name = task.name.clone();
        let task_name;
        if let Some(name) = option_task_name {
            task_name = name
        } else {
            task_name = task.id()
        }
        format!(
            "Task {task_name}'s future was auto-boxed by the runtime when spawning, due to its size (originally \
            {original_size} bytes, boxed size {boxed_size} bytes)",

        )
    }
}

/// Warning for if a task's driving future if large
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct LargeFuture {
    enabled: bool,
    min_size: usize,
    description: String,
}
impl LargeFuture {
    pub(crate) const DEFAULT_MIN_SIZE_BYTES: usize = 1024;
    pub(crate) fn new(min_size: usize) -> Self {
        Self {
            enabled: true,
            min_size,
            description: format!("tasks are {} bytes or larger", min_size),
        }
    }
}

impl Default for LargeFuture {
    fn default() -> Self {
        Self::new(Self::DEFAULT_MIN_SIZE_BYTES)
    }
}

impl Warn<Task> for LargeFuture {
    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn summary(&self) -> &str {
        self.description.as_str()
    }

    fn check(&self, task: &Task) -> Warning {
        // Don't fire warning for tasks that are not async
        if task.is_blocking() {
            return Warning::Ok;
        }

        if let Some(size_bytes) = task.size_bytes() {
            if size_bytes >= self.min_size {
                return Warning::Warn;
            }
        }
        Warning::Ok
    }

    fn format(&self, task: &Task) -> String {
        let option_task_name = task.name.clone();
        let task_name;
        if let Some(name) = option_task_name {
            task_name = name
        } else {
            task_name = task.id()
        }
        format!(
            "Task {} occupies a large amount of stack space ({} bytes)",
            task_name,
            task.size_bytes()
                .expect("warning should not trigger if size is None"),
        )
    }
}
