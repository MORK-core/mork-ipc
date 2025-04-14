use alloc::boxed::Box;
use alloc::collections::VecDeque;
use mork_capability::cap::NotificationCap;
use mork_hal::context::HALContextTrait;
use mork_task::task::TaskContext;
use mork_task::task_state::ThreadStateEnum;

pub struct Notification {
    state: NotificationState,
    signal: usize,
    block_tasks: VecDeque<Box<TaskContext>>
}

impl Notification {
    pub fn new() -> Notification {
        Self {
            state: NotificationState::OnIdle,
            signal: 0,
            block_tasks: VecDeque::new(),
        }
    }

    pub fn from_cap(cap: &NotificationCap) -> &mut Self {
        unsafe {
            &mut *((cap.base_ptr() << 12) as usize as *mut Self)
        }
    }

    pub fn signal(&mut self, badge: usize) -> Option<Box<TaskContext>> {
        match self.state {
            NotificationState::OnIdle | NotificationState::OnActive => {
                self.state = NotificationState::OnActive;
                self.signal |= 1 << badge;
                None
            }
            NotificationState::OnReceive => {
                let mut waked = self.block_tasks.pop_front().unwrap();
                waked.hal_context.set_mr(0, badge);
                waked.state = ThreadStateEnum::ThreadStateRestart;
                if self.block_tasks.is_empty() {
                    self.state = NotificationState::OnIdle;
                }
                Some(waked)
            }
        }
    }

    pub fn receive(&mut self, task: &mut TaskContext) {
        match self.state {
            NotificationState::OnIdle | NotificationState::OnReceive => unsafe {
                self.state = NotificationState::OnReceive;
                task.state = ThreadStateEnum::ThreadStateBlockedOnNotification;
                self.block_tasks.push_back(Box::from_raw(task));
            }
            NotificationState::OnActive => {
                task.hal_context.set_badge(self.signal);
                self.signal = 0;
                self.state = NotificationState::OnIdle;
            }
        }
    }
}

enum NotificationState {
    OnIdle,
    OnActive,
    OnReceive,
}