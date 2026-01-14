use actix_web::rt;
use futures::future::FutureExt;
use std::fmt::{Debug, Display};
use std::io;

use melierx_backend::configuration::get_configuration;
use melierx_backend::issue_delivery_worker::run_worker_until_stopped;
use melierx_backend::startup::Application;
use melierx_backend::telemetry::{get_subscriber, init_subscriber};

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    let subscriber =
        get_subscriber("melierx_backend".into(), "info".into(), io::stdout);
    init_subscriber(subscriber);

    let configuration =
        get_configuration().expect("Failed to read configuration.");

    let application = Application::build(configuration.clone()).await?;
    let application_task = rt::spawn(application.run_until_stopped());
    let worker_task = rt::spawn(run_worker_until_stopped(configuration));

    futures::select! {
        o = application_task.fuse() => report_exit("API", o),
        o = worker_task.fuse() => report_exit("Background worker", o),
    };

    Ok(())
}

fn report_exit(
    task_name: &str,
    outcome: Result<Result<(), impl Debug + Display>, impl Debug + Display>,
) {
    match outcome {
        Ok(Ok(())) => {
            tracing::info!("{} has exited", task_name);
        }
        Ok(Err(e)) => {
            tracing::error!(
                error.cause_chain = ?e,
                error.message = %e,
                "{} failed",
                task_name
            );
        }
        Err(e) => {
            tracing::error!(
                error.cause_chain = ?e,
                error.message = %e,
                "{} task failed to complete",
                task_name
            );
        }
    }
}
