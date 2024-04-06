use std::time::Duration;

use croner::Cron;
use tokio::{sync::oneshot, task::JoinHandle, time::{Instant, sleep_until}};
use tracing::{info, span, Instrument, Level};
use crate::{db::DbCommand, Error};
use chrono::{ DateTime, Utc };

use super::CommandSender;

// ┌───────────── minute (0–59)
// │ ┌───────────── hour (0–23)
// │ │ ┌───────────── day of the month (1–31)
// │ │ │ ┌───────────── month (1–12)
// │ │ │ │ ┌───────────── day of the week (0–6) (Sunday to Saturday;
// │ │ │ │ │                                   7 is also Sunday on some systems)
// │ │ │ │ │
// │ │ │ │ │
// * * * * * 
const DB_OPTIMIZE_CRON_SCHEDULE: &str = "10 */3 * * *";
const DB_VACUUM_CRON_SCHEDULE: &str = "30 4 */2 * *";
const DB_COMPRESS_MESSAGES_CRON_SCHEDULE: &str = "50 2 * * *";
//const DB_COMPRESS_MESSAGES_CRON_SCHEDULE: &str = "*/2 * * * *";
// This caps the max sleep the cron jobs will do. The reason for this is in case the montonic
// timer gets out of sync due to device sleep. This makes it so we can miss the assigned time 
// by at most this - 1 second
const MAX_SLEEP_DURATION: Duration = Duration::from_secs(60*10);
const MAX_COMPRESS_DURATION: Duration = Duration::from_secs(5);


#[must_use]
pub fn spawn_db_background_jobs_task(command_sender: CommandSender) -> JoinHandle<Result<(), Error>> {
    let optimize_schedule = Cron::new(DB_OPTIMIZE_CRON_SCHEDULE).parse().expect("Invalid schedule specified by DB_OPTIMIZE_CRON_SCHEDULE");
    let vacuum_schedule = Cron::new(DB_VACUUM_CRON_SCHEDULE).parse().expect("Invalid schedule specified by DB_VACUUM_CRON_SCHEDULE");
    let compress_schedule = Cron::new(DB_COMPRESS_MESSAGES_CRON_SCHEDULE).parse().expect("Invalid schedule specified by DB_COMPRESS_MESSAGES_CRON_SCHEDULE");
    
    fn get_next(cron: &Cron) -> Result<DateTime<Utc>, Error> {
        let next = cron.find_next_occurrence(&Utc::now(), false)?;
        Ok(next)
    }

    fn get_instant(next: &DateTime<Utc>) -> Result<Instant, Error> {
        let now = Utc::now();
        let nowi = Instant::now();

        Ok(if now < *next {
            nowi + MAX_SLEEP_DURATION.min((*next - now).to_std()?)
        } else {
            nowi
        })
    }

    tokio::spawn(async move {
        let mut next_optimize = get_next(&optimize_schedule)?;
        let mut next_vacuum = get_next(&vacuum_schedule)?;
        let mut next_compress = get_next(&compress_schedule)?;
        let mut optimize_instant;
        let mut vacuum_instant;
        let mut compress_instant;

        loop {
            {
                let _span = span!(Level::INFO, "Scheduling background tasks").entered();
                optimize_instant = get_instant(&next_optimize)?;
                vacuum_instant = get_instant(&next_vacuum)?;
                compress_instant = get_instant(&next_compress)?;

                info!("Next optimize: {next_optimize}");
                info!("Next vacuum: {next_vacuum}");
                info!("Next compress: {next_compress}");
            }
            tokio::select! {
                _ = sleep_until(optimize_instant) => {    
                    if Utc::now() < next_optimize {
                        continue;
                    }            

                    let span = span!(Level::INFO, "Running database optimize background task");
                    async {
                        // DB Optimize
                        let (s, r) = oneshot::channel();
                        command_sender
                            .send_async(DbCommand::Optimize { respond_to: s })
                            .await?;
                        let duration = r.await??;
                        info!("DB Optimize ran in {} μs", duration.as_micros());
                    
                        // Update the next run time
                        next_optimize = get_next(&optimize_schedule)?;
                        
                        Ok::<_, Error>(())
                    }
                    .instrument(span)
                    .await?
                },
                _ = sleep_until(vacuum_instant) => {    
                    if Utc::now() < next_vacuum {
                        continue;
                    }            

                    let span = span!(Level::INFO, "Running database vacuum background task");
                    async {
                        let (s, r) = oneshot::channel();
                        command_sender
                            .send_async(DbCommand::Vacuum { respond_to: s })
                            .await?;
                        let duration = r.await??;
                        info!("DB Vacuum ran in {} μs", duration.as_micros());
                    
                        // Update the next run time
                        next_vacuum = get_next(&vacuum_schedule)?;
                        
                        Ok::<_, Error>(())
                    }
                    .instrument(span)
                    .await?
                },
                _ = sleep_until(compress_instant) => {    
                    if Utc::now() < next_compress {
                        continue;
                    }            

                    let span = span!(Level::INFO, "Running database compress background task");
                    async {
                        let mut tot_time = Duration::from_secs(0);
                        while tot_time < MAX_COMPRESS_DURATION {
                            let (s, r) = oneshot::channel();
                            command_sender
                                .send_async(DbCommand::Compress { respond_to: s })
                                .await?;
                            let (duration, more) = r.await??;
                            tot_time += duration;
                            if !more {
                                break;
                            }
                        }
                        info!("DB compress ran in {} s", tot_time.as_secs_f32());
                    
                        // Update the next run time
                        next_compress = get_next(&compress_schedule)?;
                        
                        Ok::<_, Error>(())
                    }
                    .instrument(span)
                    .await?
                },
            }
        }
    })
}