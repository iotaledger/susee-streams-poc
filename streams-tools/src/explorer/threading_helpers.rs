use iota_streams::core::async_trait;

use super::error::Result;

#[async_trait(?Send)]
pub (crate) trait Worker {
    type OptionsType: Send;
    type ResultType: Send;

    async fn run(opt: Self::OptionsType) -> Result<Self::ResultType>;
}

pub (crate) async fn run_worker_in_own_thread<W>(worker_opt: W::OptionsType) -> Result<W::ResultType>
    where
        W: Worker,
        <W as Worker>::OptionsType: 'static,
        <W as Worker>::ResultType: 'static
{
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("build runtime");

        // Combine it with a `LocalSet,  which means it can spawn !Send futures...
        let local = tokio::task::LocalSet::new();
        local.block_on(&rt, W::run(worker_opt))
    })
        .join()
        .unwrap()
}

#[derive(Clone, Copy, Debug)]
struct LocalExec;

impl<F> hyper::rt::Executor<F> for LocalExec
    where
        F: std::future::Future + 'static, // not requiring `Send`
{
    fn execute(&self, fut: F) {
        // This will spawn into the currently running `LocalSet`.
        tokio::task::spawn_local(fut);
    }
}