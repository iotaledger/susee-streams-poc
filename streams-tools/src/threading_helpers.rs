use async_trait::async_trait;

#[async_trait(?Send)]
pub trait Worker {
    type OptionsType: Send;
    type ResultType: Send;
    type ErrorType: Send;

    async fn run(opt: Self::OptionsType) -> Result<Self::ResultType, Self::ErrorType>;
}

pub async fn run_worker_in_own_thread<W>(worker_opt: W::OptionsType) -> Result<W::ResultType, W::ErrorType>
    where
        W: Worker,
        <W as Worker>::OptionsType: 'static,
        <W as Worker>::ResultType: 'static,
        <W as Worker>::ErrorType: 'static,
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

pub fn run_background_worker_in_own_thread<W>(worker_opt: W::OptionsType)
    -> std::thread::JoinHandle<std::result::Result<<W as Worker>::ResultType, <W as Worker>::ErrorType>>
    where
        W: Worker,
        <W as Worker>::OptionsType: 'static,
        <W as Worker>::ResultType: 'static,
        <W as Worker>::ErrorType: 'static,
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