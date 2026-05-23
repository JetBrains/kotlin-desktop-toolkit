use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicI64, Ordering},
};

use anyhow::Context;
use windows::{
    Foundation::TypedEventHandler,
    System::{DispatcherQueue, DispatcherQueueHandler, DispatcherQueuePriority},
    UI::Composition::{Compositor, Core::CompositorController},
    Win32::System::Threading::GetCurrentThreadId,
};

pub struct CompositorDriver {
    controller: CompositorController,
    dispatcher_queue: DispatcherQueue,
    ui_thread_id: u32,
    autocommit_enabled: AtomicBool,
    enqueue_pending: AtomicBool,
    commit_needed_token: AtomicI64,
}

static_assertions::assert_impl_all!(CompositorDriver: Send, Sync);

impl CompositorDriver {
    pub fn new(controller: &CompositorController, dispatcher_queue: DispatcherQueue) -> anyhow::Result<Arc<Self>> {
        let driver = Arc::new(Self {
            controller: controller.clone(),
            dispatcher_queue,
            // SAFETY: `GetCurrentThreadId` has no preconditions and is always safe to call.
            ui_thread_id: unsafe { GetCurrentThreadId() },
            autocommit_enabled: AtomicBool::new(true),
            enqueue_pending: AtomicBool::new(false),
            commit_needed_token: AtomicI64::new(0),
        });
        let weak = Arc::downgrade(&driver);
        let handler = TypedEventHandler::new(move |_, _| {
            if let Some(driver) = weak.upgrade() {
                driver.on_commit_needed();
            }
            Ok(())
        });
        let token = controller
            .CommitNeeded(&handler)
            .context("CompositorController::CommitNeeded subscribe")?;
        driver.commit_needed_token.store(token, Ordering::Release);
        Ok(driver)
    }

    pub fn compositor(&self) -> anyhow::Result<Compositor> {
        Ok(self.controller.Compositor()?)
    }

    pub fn pause_autocommit(&self) {
        self.autocommit_enabled.store(false, Ordering::Release);
    }

    pub fn publish_and_resume_autocommit(&self) -> anyhow::Result<()> {
        if !self.autocommit_enabled.swap(true, Ordering::AcqRel) {
            self.controller.Commit()?;
        }
        Ok(())
    }

    pub fn shutdown(&self) {
        let token = self.commit_needed_token.swap(0, Ordering::AcqRel);
        if token != 0
            && let Err(e) = self.controller.RemoveCommitNeeded(token)
        {
            log::warn!("CompositorDriver::RemoveCommitNeeded failed: {e}");
        }
    }

    fn on_commit_needed(self: &Arc<Self>) {
        // SAFETY: `GetCurrentThreadId` has no preconditions and is always safe to call.
        if unsafe { GetCurrentThreadId() } == self.ui_thread_id {
            self.drain();
            return;
        }
        if self.enqueue_pending.swap(true, Ordering::AcqRel) {
            return;
        }
        let weak = Arc::downgrade(self);
        let handler = DispatcherQueueHandler::new(move || {
            if let Some(driver) = weak.upgrade() {
                driver.drain();
            }
            Ok(())
        });
        if !matches!(
            self.dispatcher_queue
                .TryEnqueueWithPriority(DispatcherQueuePriority::High, &handler),
            Ok(true)
        ) {
            self.enqueue_pending.store(false, Ordering::Release);
        }
    }

    fn drain(&self) {
        self.enqueue_pending.store(false, Ordering::Release);
        if self.autocommit_enabled.load(Ordering::Acquire)
            && let Err(e) = self.controller.Commit()
        {
            log::warn!("CompositorDriver drain Commit failed: {e}");
        }
    }
}

impl Drop for CompositorDriver {
    fn drop(&mut self) {
        self.shutdown();
    }
}
