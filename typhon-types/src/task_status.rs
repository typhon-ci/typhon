use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/** The different status a task can have. */
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    /** The task is pending (either it is running or it will run) */
    Pending {
        /** when `start` is `None`, this means the task has not
         * started yet. Otherwise, the task is running. */
        start: Option<OffsetDateTime>,
    },
    /** The task is done and succeeded */
    Success(TimeRange),
    /** The task is done and failed */
    Error(TimeRange),
    /** The task was canceled: either while running (then the payload
     * is a `Some(TimeRange {start,end})`) or before running. */
    // TODO: we should have either a TimeRange or a {end}, right?
    Canceled(Option<TimeRange>),
}

/** The kind of status a task can have: basically [`TaskStatus`] without
 * any time information. */
#[derive(Copy, Clone, Debug, Hash, Default, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum TaskStatusKind {
    #[default]
    Pending = 0,
    Success = 1,
    Error = 2,
    Canceled = 3,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: OffsetDateTime,
    pub end: OffsetDateTime,
}

impl From<&TaskStatus> for TaskStatusKind {
    fn from(status: &TaskStatus) -> Self {
        match status {
            TaskStatus::Pending { .. } => Self::Pending,
            TaskStatus::Success(..) => Self::Success,
            TaskStatus::Error(..) => Self::Error,
            TaskStatus::Canceled(..) => Self::Canceled,
        }
    }
}

impl From<TaskStatus> for TaskStatusKind {
    fn from(status: TaskStatus) -> Self {
        status.clone().into()
    }
}

const SUCCESS_TIME_INVARIANT: &str =
    "a `TaskStatus::Success` requires a start time and an end time";
const ERROR_TIME_INVARIANT: &str = "a `TaskStatus::Error` requires a start time and an end time";
impl TaskStatusKind {
    /** Promotes a `TaskStatusKind` to a `TaskStatus`, given a start
     * time and a finish time. Note those are optional: a success task
     * status always has a both start and a end while a canceled one
     * might have no time information. */
    pub fn into_task_status(
        self,
        start: Option<OffsetDateTime>,
        end: Option<OffsetDateTime>,
    ) -> TaskStatus {
        let range = start.zip(end).map(|(start, end)| TimeRange { start, end });
        match self {
            Self::Pending => TaskStatus::Pending { start },
            Self::Success => TaskStatus::Success(range.expect(SUCCESS_TIME_INVARIANT)),
            Self::Error => TaskStatus::Error(range.expect(ERROR_TIME_INVARIANT)),
            Self::Canceled => TaskStatus::Canceled(range),
        }
    }
}
impl TaskStatus {
    /** Extracts the (possibly non-exsitent) start and finish times of
     * a `TaskStatus`. */
    pub fn times(self) -> (Option<OffsetDateTime>, Option<OffsetDateTime>) {
        match self {
            Self::Pending { start } => (start, None),
            Self::Success(range) | Self::Error(range) | Self::Canceled(Some(range)) => {
                (Some(range.start), Some(range.end))
            }
            Self::Canceled(None) => (None, None),
        }
    }
    pub fn union(&self, rhs: &Self) -> Self {
        let (lhs_start, lhs_end) = self.times();
        let (rhs_start, rhs_end) = rhs.times();
        let start = lhs_start.min(rhs_start).or(lhs_start).or(rhs_start);
        let end = lhs_end.max(rhs_end);
        let lhs_kind: TaskStatusKind = self.into();
        let rhs_kind: TaskStatusKind = rhs.into();
        let range = start.zip(end).map(|(start, end)| TimeRange { start, end });
        match lhs_kind.max(rhs_kind) {
            TaskStatusKind::Error => Self::Error(range.expect(ERROR_TIME_INVARIANT)),
            TaskStatusKind::Pending => Self::Pending { start },
            TaskStatusKind::Canceled => Self::Canceled(range),
            TaskStatusKind::Success => Self::Success(range.expect(SUCCESS_TIME_INVARIANT)),
        }
    }
}

impl TryFrom<i32> for TaskStatusKind {
    type Error = ();
    fn try_from(n: i32) -> Result<TaskStatusKind, ()> {
        let arr = [Self::Pending, Self::Success, Self::Error, Self::Canceled];
        arr.get(n as usize).ok_or(()).copied()
    }
}
impl From<TaskStatusKind> for i32 {
    fn from(x: TaskStatusKind) -> i32 {
        (x as u8) as i32
    }
}

impl std::fmt::Display for TaskStatusKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Success => write!(f, "success"),
            Self::Error => write!(f, "error"),
            Self::Canceled => write!(f, "canceled"),
        }
    }
}

impl core::cmp::PartialOrd for TaskStatusKind {
    fn partial_cmp(&self, rhs: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(rhs))
    }
}
impl core::cmp::Ord for TaskStatusKind {
    fn cmp(&self, rhs: &Self) -> core::cmp::Ordering {
        use core::cmp::Ordering;
        if self == rhs {
            return Ordering::Equal;
        }
        match (self, rhs) {
            (TaskStatusKind::Error, _) => Ordering::Greater,
            (_, TaskStatusKind::Error) => Ordering::Less,
            (TaskStatusKind::Pending, _) => Ordering::Greater,
            (_, TaskStatusKind::Pending) => Ordering::Less,
            (TaskStatusKind::Canceled, _) => Ordering::Greater,
            (_, TaskStatusKind::Canceled) => Ordering::Less,
            (TaskStatusKind::Success, TaskStatusKind::Success) => Ordering::Greater,
        }
    }
}
